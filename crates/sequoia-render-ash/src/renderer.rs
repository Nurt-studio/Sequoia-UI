//! Vulkan renderer built directly on `ash`.
//!
//! The implementation is intentionally single-threaded and uses one frame in
//! flight. Instance data is written into persistently mapped host-visible
//! buffers, and the shared WGSL module is translated to SPIR-V once and linked
//! into four pipelines.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::mem;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ash::{Device, Instance, vk};
use winit::window::Window;

use sequoia_core::{
    Color, ImageSource, PassNode, Rect, RenderList, RenderTargetId, TargetFormat, TargetPersistence,
};
use sequoia_text::GlyphAtlas;
use sequoia_view::SurfaceDraw;

use crate::scene::{
    BLIT_BYTES, GLYPH_BYTES, GRADIENT_BYTES, QUAD_BYTES, BlitQuad, GpuQuad, Scene, SceneInput,
    build_scene, decode_image, item_opacities,
};
use crate::shader::compile_render_module;
use crate::surface::{
    AshAdapterInfo, AshFrame, AshRegion, AshSurfaceBinding, SurfaceFrame, VulkanSurface,
};
use crate::{AshError, AshOptions};

const MAX_CACHED_IMAGES: usize = 64;
const MAX_CACHED_TARGETS: usize = 8;
const MAX_TARGET_PASSES: usize = 16;
const SCREEN_SLOTS: usize = MAX_TARGET_PASSES * 2 + 2;
const SURFACE_SLOT: usize = MAX_TARGET_PASSES + 1;
const SURFACE_KEY_BIT: u64 = 1 << 60;
const INSTANCE_BUFFER_SEED: u64 = 4096;

/// A Vulkan-backed renderer for one window.
pub struct AshRenderer {
    surface: VulkanSurface,
    physical_device: vk::PhysicalDevice,
    adapter: AshAdapterInfo,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    device: Device,
    swapchain_fn: ash::khr::swapchain::Device,
    queue: vk::Queue,
    queue_family: u32,

    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    in_flight: vk::Fence,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,

    descriptor_pool: vk::DescriptorPool,
    screen_layout: vk::DescriptorSetLayout,
    texture_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    screen_set: vk::DescriptorSet,
    screen_buffer: HostBuffer,
    screen_stride: u64,
    sampler: vk::Sampler,

    quad_buffer: HostBuffer,
    glyph_buffer: HostBuffer,
    blit_buffer: HostBuffer,
    gradient_buffer: HostBuffer,

    atlas_texture: Option<Texture>,
    atlas_id: u64,
    atlas_version: u64,
    images: HashMap<u64, CachedTexture>,

    target_formats: HashMap<vk::Format, TargetFormatState>,
    render_targets: HashMap<RenderTargetId, RenderTargetTexture>,
    warned_hdr_target: bool,

    swapchain: Option<SwapchainState>,
    scene: Scene,
    viewport: (u32, u32),
    frame_index: u64,
    last_frame: Option<Instant>,
    redraw: Cell<bool>,
    options: AshOptions,

    surface_jobs: Vec<SurfaceJob>,
    surface_textures: HashMap<usize, RenderTargetTexture>,
    surface_blits: HashMap<u64, vk::DescriptorSet>,
}

struct SurfaceJob {
    binding: Rc<AshSurfaceBinding>,
    content: AshRegion,
    cached: bool,
    format: vk::Format,
    slot: usize,
    scale: f32,
    delta: Duration,
}

fn target_damage_scissor(damage: Rect, scale: f32, size: (u32, u32)) -> Option<vk::Rect2D> {
    let left = (damage.left() * scale).floor().max(0.0) as u32;
    let top = (damage.top() * scale).floor().max(0.0) as u32;
    let right = (damage.right() * scale).ceil().clamp(0.0, size.0 as f32) as u32;
    let bottom = (damage.bottom() * scale).ceil().clamp(0.0, size.1 as f32) as u32;
    let width = right.saturating_sub(left);
    let height = bottom.saturating_sub(top);
    (width > 0 && height > 0).then(|| {
        vk::Rect2D::default()
            .offset(vk::Offset2D {
                x: left.min(size.0) as i32,
                y: top.min(size.1) as i32,
            })
            .extent(vk::Extent2D { width, height })
    })
}

struct SwapchainState {
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    render_pass: vk::RenderPass,
    pipelines: Pipelines,
    extent: vk::Extent2D,
    format: vk::Format,
}

#[derive(Clone, Copy)]
struct Pipelines {
    quad: vk::Pipeline,
    glyph: vk::Pipeline,
    blit: vk::Pipeline,
    gradient: vk::Pipeline,
}

impl Pipelines {
    unsafe fn destroy(self, device: &Device) {
        for pipeline in [self.quad, self.glyph, self.blit, self.gradient] {
            if pipeline != vk::Pipeline::null() {
                device.destroy_pipeline(pipeline, None);
            }
        }
    }
}

struct TargetFormatState {
    clear_pass: vk::RenderPass,
    load_pass: vk::RenderPass,
    pipelines: Pipelines,
}

impl TargetFormatState {
    unsafe fn destroy(self, device: &Device) {
        self.pipelines.destroy(device);
        device.destroy_render_pass(self.clear_pass, None);
        device.destroy_render_pass(self.load_pass, None);
    }
}

struct RenderTargetTexture {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
    framebuffer: vk::Framebuffer,
    set: vk::DescriptorSet,
    format: vk::Format,
    size: (u32, u32),
    last_used: u64,
    persistent: bool,
}

#[derive(Clone)]
struct DrawPass {
    target: Option<RenderTargetId>,
    format: vk::Format,
    extent: vk::Extent2D,
    screen_slot: usize,
    runs: Range<u32>,
    clear: bool,
    scissor: Option<vk::Rect2D>,
}

struct Texture {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
    set: vk::DescriptorSet,
}

struct CachedTexture {
    texture: Texture,
    version: Option<u64>,
    last_used: u64,
}

struct HostBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: usize,
    mapped: *mut u8,
}

impl AshRenderer {
    /// Creates a renderer with default [`AshOptions`].
    pub fn new(window: Arc<Window>, width: u32, height: u32) -> Result<Self, AshError> {
        Self::with_options(window, width, height, AshOptions::default())
    }

    /// Creates a renderer with explicit options.
    pub fn with_options(
        window: Arc<Window>,
        width: u32,
        height: u32,
        options: AshOptions,
    ) -> Result<Self, AshError> {
        let surface = VulkanSurface::with_options(window, &options)?;
        Self::with_surface(surface, width, height, options)
    }

    /// Creates a renderer for an existing [`VulkanSurface`].
    pub fn with_surface(
        surface: VulkanSurface,
        width: u32,
        height: u32,
        options: AshOptions,
    ) -> Result<Self, AshError> {
        unsafe { Self::init(surface, width, height, options) }
    }

    unsafe fn init(
        surface: VulkanSurface,
        width: u32,
        height: u32,
        options: AshOptions,
    ) -> Result<Self, AshError> {
        let instance = surface.instance();
        let (physical_device, queue_family) = Self::pick_physical_device(
            instance,
            surface.surface_fn(),
            surface.raw(),
        )?;

        let device_extensions = instance.enumerate_device_extension_properties(physical_device)?;
        let has_device_extension = |name: &CStr| {
            device_extensions
                .iter()
                .any(|extension| CStr::from_ptr(extension.extension_name.as_ptr()) == name)
        };
        let mut device_extensions_enabled = vec![vk::KHR_SWAPCHAIN_NAME.as_ptr()];
        if has_device_extension(vk::KHR_PORTABILITY_SUBSET_NAME) {
            device_extensions_enabled.push(vk::KHR_PORTABILITY_SUBSET_NAME.as_ptr());
        }

        let priorities = [1.0f32];
        let queue_info = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&priorities)];
        let features = vk::PhysicalDeviceFeatures::default();
        let device_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extensions_enabled)
            .enabled_features(&features);
        let device = instance.create_device(physical_device, &device_info, None)?;

        let memory_properties = instance.get_physical_device_memory_properties(physical_device);
        let properties = instance.get_physical_device_properties(physical_device);
        let adapter = AshAdapterInfo {
            name: CStr::from_ptr(properties.device_name.as_ptr())
                .to_string_lossy()
                .into_owned(),
            vendor: properties.vendor_id,
            device: properties.device_id,
        };
        let screen_alignment =
            (properties.limits.min_uniform_buffer_offset_alignment as u64).max(16);
        let screen_stride = 16u64.div_ceil(screen_alignment) * screen_alignment;
        let queue = device.get_device_queue(queue_family, 0);
        let swapchain_fn = ash::khr::swapchain::Device::new(instance, &device);

        let command_pool = device.create_command_pool(
            &vk::CommandPoolCreateInfo::default()
                .queue_family_index(queue_family)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
            None,
        )?;
        let command_buffers = device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )?;
        let command_buffer = command_buffers[0];

        let in_flight = device.create_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
            None,
        )?;
        let image_available = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;
        let render_finished = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;

        let screen_layout = device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&[
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            ]),
            None,
        )?;
        let texture_layout = device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&[
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            ]),
            None,
        )?;
        let set_layouts = [screen_layout, texture_layout];
        let pipeline_layout = device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts),
            None,
        )?;

        let texture_sets = MAX_CACHED_IMAGES as u32 + MAX_CACHED_TARGETS as u32 + 4;
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(texture_sets),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLER)
                .descriptor_count(texture_sets),
        ];
        let descriptor_pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .max_sets(texture_sets + 8)
                .pool_sizes(&pool_sizes),
            None,
        )?;

        let screen_sets = device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&screen_layout)),
        )?;
        let screen_set = screen_sets[0];
        let screen_buffer = create_host_buffer(
            &device,
            &memory_properties,
            screen_stride * SCREEN_SLOTS as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;
        let buffer_info = [vk::DescriptorBufferInfo::default()
            .buffer(screen_buffer.buffer)
            .offset(0)
            .range(16)];
        let screen_write = vk::WriteDescriptorSet::default()
            .dst_set(screen_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .buffer_info(&buffer_info);
        device.update_descriptor_sets(&[screen_write], &[]);

        let sampler = device.create_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .anisotropy_enable(false)
                .compare_enable(false)
                .min_lod(0.0)
                .max_lod(0.0)
                .border_color(vk::BorderColor::INT_OPAQUE_BLACK),
            None,
        )?;

        let quad_buffer = create_host_buffer(
            &device,
            &memory_properties,
            INSTANCE_BUFFER_SEED,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let glyph_buffer = create_host_buffer(
            &device,
            &memory_properties,
            INSTANCE_BUFFER_SEED,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let blit_buffer = create_host_buffer(
            &device,
            &memory_properties,
            INSTANCE_BUFFER_SEED,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let gradient_buffer = create_host_buffer(
            &device,
            &memory_properties,
            INSTANCE_BUFFER_SEED,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;

        let mut renderer = Self {
            surface,
            physical_device,
            adapter,
            memory_properties,
            device,
            swapchain_fn,
            queue,
            queue_family,
            command_pool,
            command_buffer,
            in_flight,
            image_available,
            render_finished,
            descriptor_pool,
            screen_layout,
            texture_layout,
            pipeline_layout,
            screen_set,
            screen_buffer,
            screen_stride,
            sampler,
            quad_buffer,
            glyph_buffer,
            blit_buffer,
            gradient_buffer,
            atlas_texture: None,
            atlas_id: 0,
            atlas_version: u64::MAX,
            images: HashMap::new(),
            target_formats: HashMap::new(),
            render_targets: HashMap::new(),
            warned_hdr_target: false,
            swapchain: None,
            scene: Scene::default(),
            viewport: (width, height),
            frame_index: 0,
            last_frame: None,
            redraw: Cell::new(false),
            options,
            surface_jobs: Vec::new(),
            surface_textures: HashMap::new(),
            surface_blits: HashMap::new(),
        };

        if width > 0 && height > 0 {
            renderer.recreate_swapchain()?;
        }
        Ok(renderer)
    }

    unsafe fn pick_physical_device(
        instance: &Instance,
        surface_fn: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
    ) -> Result<(vk::PhysicalDevice, u32), AshError> {
        let devices = instance.enumerate_physical_devices()?;
        for physical_device in devices {
            let extensions = instance.enumerate_device_extension_properties(physical_device)?;
            let has_swapchain = extensions.iter().any(|extension| {
                CStr::from_ptr(extension.extension_name.as_ptr()) == vk::KHR_SWAPCHAIN_NAME
            });
            if !has_swapchain {
                continue;
            }
            let families = instance.get_physical_device_queue_family_properties(physical_device);
            for (index, family) in families.iter().enumerate() {
                if !family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    continue;
                }
                let index = index as u32;
                if let Ok(true) =
                    surface_fn.get_physical_device_surface_support(physical_device, index, surface)
                {
                    return Ok((physical_device, index));
                }
            }
        }
        Err(AshError::NoSuitableDevice)
    }

    /// The swapchain extent currently in use, or the requested viewport.
    pub fn size(&self) -> (u32, u32) {
        self.swapchain
            .as_ref()
            .map(|state| (state.extent.width, state.extent.height))
            .unwrap_or(self.viewport)
    }

    /// The window this renderer presents to.
    pub fn window(&self) -> &Window {
        self.surface.window()
    }

    /// The windowing surface backing this renderer.
    pub fn surface(&self) -> &VulkanSurface {
        &self.surface
    }

    /// The queue family used for graphics and presentation.
    pub fn queue_family(&self) -> u32 {
        self.queue_family
    }

    /// Basic information about the physical device in use.
    pub fn adapter_info(&self) -> AshAdapterInfo {
        self.adapter.clone()
    }

    /// Sets the colour used to clear the swapchain image.
    pub fn set_clear_color(&mut self, color: Color) {
        self.options.clear_color = color;
    }

    /// Returns the colour used to clear the swapchain image.
    pub fn clear_color(&self) -> Color {
        self.options.clear_color
    }

    /// Consumes a redraw request made through [`crate::AshFrame::request_frame`].
    pub fn take_redraw_request(&self) -> bool {
        self.redraw.take()
    }

    /// Blocks until the device is idle.
    pub fn wait_idle(&mut self) -> Result<(), AshError> {
        unsafe {
            self.device.device_wait_idle()?;
        }
        Ok(())
    }

    /// Resizes the swapchain. Zero dimensions suspend presentation until the
    /// next non-zero resize.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), AshError> {
        if self.viewport == (width, height) {
            return Ok(());
        }
        self.viewport = (width, height);
        self.recreate_swapchain()
    }

    fn recreate_swapchain(&mut self) -> Result<(), AshError> {
        unsafe {
            self.device.queue_wait_idle(self.queue)?;
            self.destroy_swapchain();
        }
        if self.viewport.0 == 0 || self.viewport.1 == 0 {
            return Ok(());
        }
        let state = unsafe { self.create_swapchain(self.viewport.0, self.viewport.1)? };
        self.swapchain = Some(state);
        Ok(())
    }

    unsafe fn create_swapchain(&self, width: u32, height: u32) -> Result<SwapchainState, AshError> {
        let capabilities = self.surface.surface_fn().get_physical_device_surface_capabilities(
            self.physical_device,
            self.surface.raw(),
        )?;
        let formats = self
            .surface
            .surface_fn()
            .get_physical_device_surface_formats(self.physical_device, self.surface.raw())?;
        let present_modes = self
            .surface
            .surface_fn()
            .get_physical_device_surface_present_modes(self.physical_device, self.surface.raw())?;

        let format =
            crate::options::choose_surface_format(&formats, self.options.preferred_format)?;
        let present_mode =
            crate::options::choose_present_mode(&present_modes, self.options.present_mode);

        let mut extent = capabilities.current_extent;
        if extent.width == u32::MAX || extent.height == u32::MAX {
            extent = vk::Extent2D {
                width: width.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: height.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            };
        }
        extent.width = extent.width.max(1);
        extent.height = extent.height.max(1);

        let mut image_count = capabilities.min_image_count;
        if capabilities.max_image_count > 0 {
            image_count = image_count.min(capabilities.max_image_count);
        }
        image_count = image_count.max(1);

        let supported = capabilities.supported_composite_alpha;
        let composite_alpha = if supported.contains(vk::CompositeAlphaFlagsKHR::OPAQUE) {
            vk::CompositeAlphaFlagsKHR::OPAQUE
        } else if supported.contains(vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED) {
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED
        } else if supported.contains(vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED) {
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED
        } else {
            vk::CompositeAlphaFlagsKHR::INHERIT
        };

        let swapchain = self.swapchain_fn.create_swapchain(
            &vk::SwapchainCreateInfoKHR::default()
                .surface(self.surface.raw())
                .min_image_count(image_count)
                .image_format(format.format)
                .image_color_space(format.color_space)
                .image_extent(extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(capabilities.current_transform)
                .composite_alpha(composite_alpha)
                .present_mode(present_mode)
                .clipped(true)
                .old_swapchain(vk::SwapchainKHR::null()),
            None,
        )?;

        let images = self.swapchain_fn.get_swapchain_images(swapchain)?;
        let render_pass = self.create_render_pass(format.format)?;
        let pipelines = self.create_pipelines(render_pass, format.format)?;

        let subresource = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);
        let mut views = Vec::with_capacity(images.len());
        for image in &images {
            let view = self.device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .components(vk::ComponentMapping::default())
                    .subresource_range(subresource),
                None,
            )?;
            views.push(view);
        }

        let mut framebuffers = Vec::with_capacity(views.len());
        for view in &views {
            let framebuffer = self.device.create_framebuffer(
                &vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(std::slice::from_ref(view))
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1),
                None,
            )?;
            framebuffers.push(framebuffer);
        }

        Ok(SwapchainState {
            swapchain,
            images,
            views,
            framebuffers,
            render_pass,
            pipelines,
            extent,
            format: format.format,
        })
    }

    unsafe fn create_render_pass(&self, format: vk::Format) -> Result<vk::RenderPass, AshError> {
        let attachments = [vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];
        let color_attachments = [vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
        let subpasses = [vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments)];
        let dependencies = [
            vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
            vk::SubpassDependency::default()
                .src_subpass(0)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
        ];
        let info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        Ok(self.device.create_render_pass(&info, None)?)
    }

    unsafe fn create_pipelines(
        &self,
        render_pass: vk::RenderPass,
        format: vk::Format,
    ) -> Result<Pipelines, AshError> {
        let words = compile_render_module()?;
        let module = self
            .device
            .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&words), None)?;

        let result = (|| {
            let quad = create_pipeline(
                &self.device,
                self.pipeline_layout,
                render_pass,
                format,
                module,
                c"vs_quad",
                c"fs_quad",
                QUAD_BYTES as u32,
                &[
                    (0, vk::Format::R32G32B32A32_SFLOAT, 0),
                    (1, vk::Format::R32G32B32A32_SFLOAT, 16),
                    (2, vk::Format::R32_SFLOAT, 32),
                    (3, vk::Format::R32G32B32A32_SFLOAT, 36),
                    (4, vk::Format::R32_SFLOAT, 52),
                    (5, vk::Format::R32G32B32A32_SFLOAT, 56),
                    (6, vk::Format::R32_SFLOAT, 72),
                    (7, vk::Format::R32_SFLOAT, 76),
                    (8, vk::Format::R32_SFLOAT, 80),
                    (9, vk::Format::R32G32_SFLOAT, 84),
                    (10, vk::Format::R32_SFLOAT, 92),
                    (11, vk::Format::R32G32B32A32_SFLOAT, 96),
                    (12, vk::Format::R32G32_SFLOAT, 112),
                ],
            )?;
            let glyph = match create_pipeline(
                &self.device,
                self.pipeline_layout,
                render_pass,
                format,
                module,
                c"vs_glyph",
                c"fs_glyph",
                GLYPH_BYTES as u32,
                &[
                    (0, vk::Format::R32G32B32A32_SFLOAT, 0),
                    (1, vk::Format::R32G32B32A32_SFLOAT, 16),
                    (2, vk::Format::R32G32B32A32_SFLOAT, 32),
                    (3, vk::Format::R32_SFLOAT, 48),
                    (4, vk::Format::R32G32B32A32_SFLOAT, 52),
                    (5, vk::Format::R32_SFLOAT, 68),
                    (6, vk::Format::R32G32B32A32_SFLOAT, 72),
                    (7, vk::Format::R32G32_SFLOAT, 88),
                ],
            ) {
                Ok(pipeline) => pipeline,
                Err(error) => {
                    self.device.destroy_pipeline(quad, None);
                    return Err(error);
                }
            };
            let blit = match create_pipeline(
                &self.device,
                self.pipeline_layout,
                render_pass,
                format,
                module,
                c"vs_blit",
                c"fs_blit",
                BLIT_BYTES as u32,
                &[
                    (0, vk::Format::R32G32B32A32_SFLOAT, 0),
                    (1, vk::Format::R32G32B32A32_SFLOAT, 16),
                    (2, vk::Format::R32G32B32A32_SFLOAT, 32),
                    (3, vk::Format::R32_SFLOAT, 48),
                    (4, vk::Format::R32G32B32A32_SFLOAT, 52),
                    (5, vk::Format::R32G32_SFLOAT, 68),
                    (6, vk::Format::R32_SFLOAT, 76),
                    (7, vk::Format::R32_SFLOAT, 80),
                ],
            ) {
                Ok(pipeline) => pipeline,
                Err(error) => {
                    self.device.destroy_pipeline(quad, None);
                    self.device.destroy_pipeline(glyph, None);
                    return Err(error);
                }
            };
            let gradient = match create_pipeline(
                &self.device,
                self.pipeline_layout,
                render_pass,
                format,
                module,
                c"vs_gradient",
                c"fs_gradient",
                GRADIENT_BYTES as u32,
                &[
                    (0, vk::Format::R32G32B32A32_SFLOAT, 0),
                    (1, vk::Format::R32G32B32A32_SFLOAT, 16),
                    (2, vk::Format::R32G32B32A32_SFLOAT, 32),
                    (3, vk::Format::R32G32B32A32_SFLOAT, 48),
                    (4, vk::Format::R32G32B32A32_SFLOAT, 64),
                    (5, vk::Format::R32G32B32A32_SFLOAT, 80),
                    (6, vk::Format::R32G32B32A32_SFLOAT, 96),
                    (7, vk::Format::R32G32B32A32_SFLOAT, 112),
                    (8, vk::Format::R32_SFLOAT, 128),
                    (9, vk::Format::R32G32B32A32_SFLOAT, 152),
                    (10, vk::Format::R32G32_SFLOAT, 168),
                ],
            ) {
                Ok(pipeline) => pipeline,
                Err(error) => {
                    self.device.destroy_pipeline(quad, None);
                    self.device.destroy_pipeline(glyph, None);
                    self.device.destroy_pipeline(blit, None);
                    return Err(error);
                }
            };
            Ok(Pipelines {
                quad,
                glyph,
                blit,
                gradient,
            })
        })();

        self.device.destroy_shader_module(module, None);
        result
    }

    unsafe fn destroy_swapchain(&mut self) {
        let Some(state) = self.swapchain.take() else {
            return;
        };
        state.pipelines.destroy(&self.device);
        for framebuffer in state.framebuffers {
            self.device.destroy_framebuffer(framebuffer, None);
        }
        for view in state.views {
            self.device.destroy_image_view(view, None);
        }
        self.swapchain_fn.destroy_swapchain(state.swapchain, None);
        self.device.destroy_render_pass(state.render_pass, None);
        let _ = state.images;
    }

    fn sync_atlas(&mut self, atlas: &mut GlyphAtlas) -> Result<(), AshError> {
        let (width, height) = atlas.size();
        let id = atlas.id();
        let version = atlas.version();
        if self.atlas_texture.is_some() && self.atlas_id == id && self.atlas_version == version {
            let _ = atlas.take_dirty();
            return Ok(());
        }
        let expected = width as usize * height as usize * 4;
        if atlas.pixels().len() < expected || expected == 0 {
            let _ = atlas.take_dirty();
            return Ok(());
        }
        let texture = unsafe {
            let texture = self.create_texture(width, height)?;
            if let Err(error) =
                self.upload_texture(&texture, &atlas.pixels()[..expected], width, height)
            {
                self.destroy_texture(texture);
                return Err(error);
            }
            texture
        };
        let _ = atlas.take_dirty();
        if let Some(old) = self.atlas_texture.take() {
            unsafe { self.destroy_texture(old) };
        }
        self.atlas_texture = Some(texture);
        self.atlas_id = id;
        self.atlas_version = version;
        Ok(())
    }

    fn sync_images(&mut self) -> Result<(), AshError> {
        let mut wanted: Vec<(u64, ImageSource, Option<u64>)> = Vec::new();
        let mut seen = HashSet::new();
        for source in &self.scene.blit_sources {
            let key = source.cache_key();
            if seen.insert(key) {
                wanted.push((key, source.clone(), source.dynamic_version()));
            }
        }

        for (key, source, version) in wanted {
            if let Some(cached) = self.images.get_mut(&key) {
                cached.last_used = self.frame_index;
                if cached.version == version {
                    continue;
                }
            }
            let Some((width, height, pixels)) = decode_image(&source) else {
                if let Some(old) = self.images.remove(&key) {
                    unsafe { self.destroy_texture(old.texture) };
                }
                continue;
            };
            let texture = unsafe {
                let texture = self.create_texture(width, height)?;
                if let Err(error) = self.upload_texture(&texture, &pixels, width, height) {
                    self.destroy_texture(texture);
                    return Err(error);
                }
                texture
            };
            if let Some(old) = self.images.remove(&key) {
                unsafe { self.destroy_texture(old.texture) };
            }
            self.images.insert(
                key,
                CachedTexture {
                    texture,
                    version,
                    last_used: self.frame_index,
                },
            );
        }
        self.evict_images();
        Ok(())
    }

    fn evict_images(&mut self) {
        while self.images.len() > MAX_CACHED_IMAGES {
            let key = self
                .images
                .iter()
                .min_by_key(|(_, cached)| cached.last_used)
                .map(|(key, _)| *key);
            let Some(key) = key else {
                break;
            };
            if let Some(old) = self.images.remove(&key) {
                unsafe { self.destroy_texture(old.texture) };
            }
        }
    }

    fn build_frame(
        &mut self,
        list: &RenderList,
        scale: f32,
        extent: vk::Extent2D,
    ) -> Result<Vec<DrawPass>, AshError> {
        let opacities = item_opacities(list);
        let mut scene = Scene::default();
        let mut present = Scene::default();
        let mut passes = Vec::new();
        let mut next_slot = 1usize;
        let max = (
            extent.width.max(1).saturating_mul(4),
            extent.height.max(1).saturating_mul(4),
        );

        for node in list.target_plan() {
            match node {
                PassNode::Render {
                    target,
                    size,
                    format,
                    persistence,
                    damage,
                    items,
                    ..
                } => {
                    let width = ((size.width * scale).ceil() as u32).clamp(1, max.0);
                    let height = ((size.height * scale).ceil() as u32).clamp(1, max.1);
                    let target_extent = vk::Extent2D { width, height };
                    let format = self.target_texture_format(format);
                    let kept = unsafe {
                        self.ensure_render_target(
                            target,
                            target_extent,
                            format,
                            persistence == TargetPersistence::Preserve,
                        )?
                    };
                    let reset = kept && persistence == TargetPersistence::Clear;
                    if items.is_empty() && !reset {
                        continue;
                    }
                    if next_slot >= SCREEN_SLOTS {
                        log::warn!(
                            "sequoia-render-ash: more than {MAX_TARGET_PASSES} target passes; \
                             skipping {target:?}"
                        );
                        continue;
                    }

                    let mut pass_scene = Scene::default();
                    build_scene(
                        SceneInput {
                            items: &list.items()[items.clone()],
                            clips: &list.item_clips()[items.clone()],
                            transforms: &list.item_transforms()[items.clone()],
                            opacities: &opacities[items.clone()],
                            scale,
                            viewport: Rect::from_xywh(
                                0.0,
                                0.0,
                                size.width.max(0.0),
                                size.height.max(0.0),
                            ),
                        },
                        &mut pass_scene,
                    );
                    if pass_scene.runs.is_empty() && !reset {
                        continue;
                    }
                    let runs = scene.append(pass_scene);
                    passes.push(DrawPass {
                        target: Some(target),
                        format,
                        extent: target_extent,
                        screen_slot: next_slot,
                        runs,
                        clear: !kept || reset,
                        scissor: if kept && persistence == TargetPersistence::Preserve {
                            damage.and_then(|rect| target_damage_scissor(rect, scale, (width, height)))
                        } else {
                            None
                        },
                    });
                    next_slot += 1;
                }
                PassNode::Present { items } => {
                    let mut pass_scene = Scene::default();
                    build_scene(
                        SceneInput {
                            items: &list.items()[items.clone()],
                            clips: &list.item_clips()[items.clone()],
                            transforms: &list.item_transforms()[items.clone()],
                            opacities: &opacities[items.clone()],
                            scale,
                            viewport: Rect::from_xywh(
                                0.0,
                                0.0,
                                extent.width as f32 / scale,
                                extent.height as f32 / scale,
                            ),
                        },
                        &mut pass_scene,
                    );
                    present.append(pass_scene);
                }
                _ => {}
            }
        }

        let runs = scene.append(present);
        let format = self
            .swapchain
            .as_ref()
            .map(|state| state.format)
            .unwrap_or(vk::Format::UNDEFINED);
        passes.push(DrawPass {
            target: None,
            format,
            extent,
            screen_slot: 0,
            runs,
            clear: true,
            scissor: None,
        });

        self.scene = scene;
        self.evict_targets();
        Ok(passes)
    }

    fn target_texture_format(&mut self, format: TargetFormat) -> vk::Format {
        let surface_format = self
            .swapchain
            .as_ref()
            .map(|state| state.format)
            .unwrap_or(vk::Format::UNDEFINED);
        if format != TargetFormat::Rgba16Float {
            return surface_format;
        }
        let hdr = vk::Format::R16G16B16A16_SFLOAT;
        let properties = unsafe {
            self.surface
                .instance()
                .get_physical_device_format_properties(self.physical_device, hdr)
        };
        let required = vk::FormatFeatureFlags::COLOR_ATTACHMENT
            | vk::FormatFeatureFlags::SAMPLED_IMAGE
            | vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR;
        if properties.optimal_tiling_features.contains(required) {
            hdr
        } else {
            if !self.warned_hdr_target {
                self.warned_hdr_target = true;
                log::warn!(
                    "sequoia-render-ash: Rgba16Float intermediates are not filterable on this \
                     device - falling back to the window format"
                );
            }
            surface_format
        }
    }

    unsafe fn ensure_render_target(
        &mut self,
        target: RenderTargetId,
        extent: vk::Extent2D,
        format: vk::Format,
        persistent: bool,
    ) -> Result<bool, AshError> {
        let size = (extent.width, extent.height);
        if let Some(entry) = self.render_targets.get_mut(&target) {
            if entry.format == format && entry.size == size {
                entry.last_used = self.frame_index;
                entry.persistent = persistent;
                return Ok(true);
            }
        }
        if let Some(old) = self.render_targets.remove(&target) {
            self.destroy_render_target(old);
        }
        self.ensure_target_format(format)?;

        let image = self.device.create_image(
            &vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(format)
                .extent(vk::Extent3D {
                    width: extent.width,
                    height: extent.height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED),
            None,
        )?;
        let requirements = self.device.get_image_memory_requirements(image);
        let memory_type = find_memory_type(
            &self.memory_properties,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .ok_or(AshError::Vulkan(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY))?;
        let memory = match self.device.allocate_memory(
            &vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(memory_type),
            None,
        ) {
            Ok(memory) => memory,
            Err(error) => {
                self.device.destroy_image(image, None);
                return Err(AshError::Vulkan(error));
            }
        };
        if let Err(error) = self.device.bind_image_memory(image, memory, 0) {
            self.device.destroy_image(image, None);
            self.device.free_memory(memory, None);
            return Err(AshError::Vulkan(error));
        }
        let view = match self.device.create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(vk::ComponentMapping::default())
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                ),
            None,
        ) {
            Ok(view) => view,
            Err(error) => {
                self.device.destroy_image(image, None);
                self.device.free_memory(memory, None);
                return Err(AshError::Vulkan(error));
            }
        };
        let sets = match self.device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.texture_layout)),
        ) {
            Ok(sets) => sets,
            Err(error) => {
                self.device.destroy_image_view(view, None);
                self.device.destroy_image(image, None);
                self.device.free_memory(memory, None);
                return Err(AshError::Vulkan(error));
            }
        };
        let set = sets[0];
        let image_info = [vk::DescriptorImageInfo::default()
            .image_view(view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
        let sampler_info = [vk::DescriptorImageInfo::default().sampler(self.sampler)];
        self.device.update_descriptor_sets(
            &[
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&sampler_info),
            ],
            &[],
        );

        let render_pass = self
            .target_formats
            .get(&format)
            .map(|state| state.clear_pass)
            .unwrap_or(vk::RenderPass::null());
        let framebuffer = match self.device.create_framebuffer(
            &vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(std::slice::from_ref(&view))
                .width(extent.width)
                .height(extent.height)
                .layers(1),
            None,
        ) {
            Ok(framebuffer) => framebuffer,
            Err(error) => {
                let _ = self.device.free_descriptor_sets(self.descriptor_pool, &[set]);
                self.device.destroy_image_view(view, None);
                self.device.destroy_image(image, None);
                self.device.free_memory(memory, None);
                return Err(AshError::Vulkan(error));
            }
        };

        self.render_targets.insert(
            target,
            RenderTargetTexture {
                image,
                memory,
                view,
                framebuffer,
                set,
                format,
                size,
                last_used: self.frame_index,
                persistent,
            },
        );
        Ok(false)
    }

    unsafe fn ensure_target_format(&mut self, format: vk::Format) -> Result<(), AshError> {
        if self.target_formats.contains_key(&format) {
            return Ok(());
        }
        let clear_pass = self.create_target_render_pass(format, false)?;
        let load_pass = match self.create_target_render_pass(format, true) {
            Ok(render_pass) => render_pass,
            Err(error) => {
                self.device.destroy_render_pass(clear_pass, None);
                return Err(error);
            }
        };
        let pipelines = match self.create_pipelines(clear_pass, format) {
            Ok(pipelines) => pipelines,
            Err(error) => {
                self.device.destroy_render_pass(load_pass, None);
                self.device.destroy_render_pass(clear_pass, None);
                return Err(error);
            }
        };
        self.target_formats.insert(
            format,
            TargetFormatState {
                clear_pass,
                load_pass,
                pipelines,
            },
        );
        Ok(())
    }

    unsafe fn create_target_render_pass(
        &self,
        format: vk::Format,
        load: bool,
    ) -> Result<vk::RenderPass, AshError> {
        let attachments = [vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(if load {
                vk::AttachmentLoadOp::LOAD
            } else {
                vk::AttachmentLoadOp::CLEAR
            })
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(if load {
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            } else {
                vk::ImageLayout::UNDEFINED
            })
            .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
        let color_attachments = [vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
        let subpasses = [vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments)];
        let dependencies = [
            vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
                .src_access_mask(vk::AccessFlags::SHADER_READ)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
            vk::SubpassDependency::default()
                .src_subpass(0)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
                .dst_access_mask(vk::AccessFlags::SHADER_READ),
        ];
        let info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        Ok(self.device.create_render_pass(&info, None)?)
    }

    fn evict_targets(&mut self) {
        while self.render_targets.len() > MAX_CACHED_TARGETS {
            let oldest = self
                .render_targets
                .iter()
                .filter(|(_, entry)| entry.last_used != self.frame_index)
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(target, _)| *target);
            let Some(oldest) = oldest else {
                break;
            };
            if let Some(old) = self.render_targets.remove(&oldest) {
                unsafe {
                    self.destroy_render_target(old);
                }
            }
        }
    }

    unsafe fn destroy_render_target(&self, target: RenderTargetTexture) {
        let _ = self
            .device
            .free_descriptor_sets(self.descriptor_pool, &[target.set]);
        self.device
            .destroy_framebuffer(target.framebuffer, None);
        self.device.destroy_image_view(target.view, None);
        self.device.destroy_image(target.image, None);
        self.device.free_memory(target.memory, None);
    }

    unsafe fn update_screen(&self, slot: usize, width: u32, height: u32) {
        let data = [width as f32, height as f32, 0.0, 0.0];
        let bytes = bytemuck::cast_slice(&data);
        let offset = slot as u64 * self.screen_stride;
        let mapped = self.screen_buffer.mapped.add(offset as usize);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), mapped, bytes.len());
    }

    fn phys_rect(rect: Rect, scale: f32) -> Rect {
        Rect::from_xywh(
            rect.left() * scale,
            rect.top() * scale,
            rect.size.width * scale,
            rect.size.height * scale,
        )
    }

    fn append_surface_decoration(&mut self, rect: Rect, frame: &SurfaceFrame, scale: f32) {
        let rect = Self::phys_rect(rect, scale);
        let xywh = [rect.left(), rect.top(), rect.size.width, rect.size.height];
        let quad = |color: [f32; 4], border_color: [f32; 4], border_width: f32| GpuQuad {
            rect: xywh,
            color,
            radius: frame.radius * scale,
            border_color,
            border_width,
            clip_rect: [0.0; 4],
            clip_radius: 0.0,
            softness: 0.0,
            spread: 0.0,
            shadow_offset: [0.0; 2],
            inset: 0.0,
            transform_a: [1.0, 0.0, 0.0, 1.0],
            transform_b: [0.0, 0.0],
        };
        if let Some(color) = frame.background {
            self.scene.push_quad(quad(color, [0.0; 4], 0.0));
        }
        if let Some((color, width)) = frame.border {
            self.scene
                .push_quad(quad([0.0; 4], color, width * scale));
        }
    }

    fn prepare_surfaces(
        &mut self,
        surfaces: &[SurfaceDraw],
        scale: f32,
        delta: Duration,
        passes: &mut [DrawPass],
    ) -> Result<(), AshError> {
        let swapchain_format = self
            .swapchain
            .as_ref()
            .map(|state| state.format)
            .unwrap_or(vk::Format::R8G8B8A8_UNORM);
        let mut cached_index = 0usize;

        for draw in surfaces {
            let Ok(binding) = draw.target.clone().downcast::<AshSurfaceBinding>() else {
                continue;
            };
            self.append_surface_decoration(draw.rect, &binding.frame, scale);

            let [left, top, right, bottom] = binding.frame.inset;
            let content = Rect::from_xywh(
                draw.rect.left() + left,
                draw.rect.top() + top,
                (draw.rect.size.width - left - right).max(0.0),
                (draw.rect.size.height - top - bottom).max(0.0),
            );
            let Some(content) = AshRegion::from_phys(Self::phys_rect(content, scale)) else {
                continue;
            };
            let cached = binding.cached;
            let format = if cached {
                binding.format.unwrap_or(vk::Format::R8G8B8A8_UNORM)
            } else {
                swapchain_format
            };
            let slot = if cached {
                let slot = SURFACE_SLOT + cached_index.min(MAX_TARGET_PASSES - 1);
                cached_index += 1;
                slot
            } else {
                0
            };

            if cached {
                let texture = unsafe {
                    self.ensure_surface_texture(binding.key, (content.width, content.height), format)?
                };
                let key = SURFACE_KEY_BIT | (binding.key as u64 & !SURFACE_KEY_BIT);
                self.surface_blits.insert(key, texture);
                self.scene.push_blit(
                    BlitQuad {
                        rect: [
                            content.x as f32,
                            content.y as f32,
                            content.width as f32,
                            content.height as f32,
                        ],
                        uv: [0.0, 0.0, 1.0, 1.0],
                        clip_rect: [0.0; 4],
                        clip_radius: 0.0,
                        transform_a: [1.0, 0.0, 0.0, 1.0],
                        transform_b: [0.0, 0.0],
                        opacity: 1.0,
                        nearest: 0.0,
                    },
                    key,
                );
            }

            self.surface_jobs.push(SurfaceJob {
                binding,
                content,
                cached,
                format,
                slot,
                scale,
                delta,
            });
        }

        if let Some(present) = passes.iter_mut().rev().find(|pass| pass.target.is_none()) {
            present.runs.end = self.scene.runs.len() as u32;
        }
        Ok(())
    }

    unsafe fn ensure_surface_texture(
        &mut self,
        key: usize,
        size: (u32, u32),
        format: vk::Format,
    ) -> Result<vk::DescriptorSet, AshError> {
        if let Some(existing) = self.surface_textures.get(&key)
            && existing.size == size
            && existing.format == format
        {
            return Ok(existing.set);
        }
        if let Some(old) = self.surface_textures.remove(&key) {
            self.destroy_render_target(old);
        }
        self.ensure_target_format(format)?;
        let render_pass = self.target_formats[&format].clear_pass;

        let extent = vk::Extent3D {
            width: size.0,
            height: size.1,
            depth: 1,
        };
        let image = self.device.create_image(
            &vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(format)
                .extent(extent)
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED),
            None,
        )?;
        let requirements = self.device.get_image_memory_requirements(image);
        let Some(memory_type) = find_memory_type(
            &self.memory_properties,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ) else {
            self.device.destroy_image(image, None);
            return Err(AshError::Vulkan(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY));
        };
        let memory = self.device.allocate_memory(
            &vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(memory_type),
            None,
        )?;
        if let Err(error) = self.device.bind_image_memory(image, memory, 0) {
            self.device.destroy_image(image, None);
            self.device.free_memory(memory, None);
            return Err(AshError::Vulkan(error));
        }
        let view = match self.device.create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .level_count(1)
                        .layer_count(1),
                ),
            None,
        ) {
            Ok(view) => view,
            Err(error) => {
                self.device.destroy_image(image, None);
                self.device.free_memory(memory, None);
                return Err(AshError::Vulkan(error));
            }
        };
        let sets = match self.device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.texture_layout)),
        ) {
            Ok(sets) => sets,
            Err(error) => {
                self.device.destroy_image_view(view, None);
                self.device.destroy_image(image, None);
                self.device.free_memory(memory, None);
                return Err(AshError::Vulkan(error));
            }
        };
        let set = sets[0];
        let image_info = [vk::DescriptorImageInfo::default()
            .image_view(view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
        let sampler_info = [vk::DescriptorImageInfo::default().sampler(self.sampler)];
        self.device.update_descriptor_sets(
            &[
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&sampler_info),
            ],
            &[],
        );
        let framebuffer = match self.device.create_framebuffer(
            &vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(std::slice::from_ref(&view))
                .width(size.0)
                .height(size.1)
                .layers(1),
            None,
        ) {
            Ok(framebuffer) => framebuffer,
            Err(error) => {
                let _ = self.device.free_descriptor_sets(self.descriptor_pool, &[set]);
                self.device.destroy_image_view(view, None);
                self.device.destroy_image(image, None);
                self.device.free_memory(memory, None);
                return Err(AshError::Vulkan(error));
            }
        };
        self.surface_textures.insert(
            key,
            RenderTargetTexture {
                image,
                memory,
                view,
                framebuffer,
                set,
                format,
                size,
                last_used: self.frame_index,
                persistent: true,
            },
        );
        Ok(set)
    }

    /// Renders `list` with glyphs sourced from `atlas`.
    ///
    /// `scale` converts logical coordinates into physical pixels.
    pub fn render(
        &mut self,
        list: &RenderList,
        atlas: &mut GlyphAtlas,
        scale: f32,
    ) -> Result<(), AshError> {
        self.render_with_damage_regions(list, atlas, scale, &[], None)
    }

    /// Renders one frame with a single damage rectangle.
    pub fn render_with_damage(
        &mut self,
        list: &RenderList,
        atlas: &mut GlyphAtlas,
        scale: f32,
        surfaces: &[SurfaceDraw],
        damage: Option<Rect>,
    ) -> Result<(), AshError> {
        self.render_with_damage_regions(
            list,
            atlas,
            scale,
            surfaces,
            damage.as_ref().map(std::slice::from_ref),
        )
    }

    /// Renders one frame, including embedded [`crate::AshSurface`] views.
    ///
    /// Damage regions are accepted for API parity with the wgpu backend. The
    /// experimental Vulkan renderer currently repaints the whole swapchain.
    pub fn render_with_damage_regions(
        &mut self,
        list: &RenderList,
        atlas: &mut GlyphAtlas,
        scale: f32,
        surfaces: &[SurfaceDraw],
        _damage: Option<&[Rect]>,
    ) -> Result<(), AshError> {
        if self.viewport.0 == 0 || self.viewport.1 == 0 || self.swapchain.is_none() {
            return Ok(());
        }
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };

        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight], true, u64::MAX)?;
        }

        self.sync_atlas(atlas)?;

        let extent = self
            .swapchain
            .as_ref()
            .map(|state| state.extent)
            .unwrap_or(vk::Extent2D {
                width: self.viewport.0,
                height: self.viewport.1,
            });
        self.frame_index = self.frame_index.wrapping_add(1);
        let delta = self
            .last_frame
            .map(|previous| previous.elapsed())
            .unwrap_or_default();
        self.last_frame = Some(Instant::now());
        let mut passes = self.build_frame(list, scale, extent)?;
        self.surface_jobs.clear();
        self.surface_blits.clear();
        self.prepare_surfaces(surfaces, scale, delta, &mut passes)?;
        self.sync_images()?;

        unsafe {
            self.quad_buffer.upload(
                &self.device,
                &self.memory_properties,
                bytemuck::cast_slice(&self.scene.quads),
            )?;
            self.glyph_buffer.upload(
                &self.device,
                &self.memory_properties,
                bytemuck::cast_slice(&self.scene.glyphs),
            )?;
            self.blit_buffer.upload(
                &self.device,
                &self.memory_properties,
                bytemuck::cast_slice(&self.scene.blits),
            )?;
            self.gradient_buffer.upload(
                &self.device,
                &self.memory_properties,
                bytemuck::cast_slice(&self.scene.gradients),
            )?;
            for pass in &passes {
                self.update_screen(pass.screen_slot, pass.extent.width, pass.extent.height);
            }
            for job in &self.surface_jobs {
                self.update_screen(job.slot, job.content.width, job.content.height);
            }
        }

        let (image_index, _suboptimal) = {
            let state = self.swapchain.as_ref().expect("checked above");
            match unsafe {
                self.swapchain_fn.acquire_next_image(
                    state.swapchain,
                    u64::MAX,
                    self.image_available,
                    vk::Fence::null(),
                )
            } {
                Ok(result) => result,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.recreate_swapchain()?;
                    return Ok(());
                }
                Err(error) => return Err(AshError::Vulkan(error)),
            }
        };

        unsafe {
            self.record_frame(image_index, &passes)?;
        }

        let submit_result = unsafe {
            self.device.reset_fences(&[self.in_flight])?;
            let wait_semaphores = [self.image_available];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let signal_semaphores = [self.render_finished];
            let command_buffers = [self.command_buffer];
            let submit = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);
            self.device
                .queue_submit(self.queue, &[submit], self.in_flight)
        };
        if let Err(error) = submit_result {
            return Err(AshError::Vulkan(error));
        }

        let present_result = unsafe {
            let state = self.swapchain.as_ref().expect("checked above");
            let swapchains = [state.swapchain];
            let image_indices = [image_index];
            let wait_semaphores = [self.render_finished];
            let present = vk::PresentInfoKHR::default()
                .wait_semaphores(&wait_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);
            self.swapchain_fn.queue_present(self.queue, &present)
        };

        match present_result {
            Ok(false) => Ok(()),
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::SUBOPTIMAL_KHR) => {
                self.recreate_swapchain()
            }
            Err(error) => Err(AshError::Vulkan(error)),
        }
    }

    unsafe fn record_runs(
        &self,
        command_buffer: vk::CommandBuffer,
        pipelines: Pipelines,
        runs: Range<u32>,
    ) {
        let device = &self.device;
        let atlas_set = self.atlas_texture.as_ref().map(|texture| texture.set);
        for index in runs {
            let Some(run) = self.scene.runs.get(index as usize) else {
                continue;
            };
            match *run {
                crate::scene::Run::Quads { start, len } => {
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipelines.quad,
                    );
                    device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[self.quad_buffer.buffer],
                        &[0],
                    );
                    device.cmd_draw(command_buffer, 4, len, 0, start);
                }
                crate::scene::Run::Glyphs { start, len } => {
                    let Some(atlas_set) = atlas_set else {
                        continue;
                    };
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipelines.glyph,
                    );
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        1,
                        &[atlas_set],
                        &[],
                    );
                    device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[self.glyph_buffer.buffer],
                        &[0],
                    );
                    device.cmd_draw(command_buffer, 4, len, 0, start);
                }
                crate::scene::Run::Images { start, len } => {
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipelines.blit,
                    );
                    device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[self.blit_buffer.buffer],
                        &[0],
                    );
                    for offset in 0..len {
                        let index = (start + offset) as usize;
                        let Some(key) = self.scene.blit_keys.get(index) else {
                            continue;
                        };
                        let set = if key & SURFACE_KEY_BIT != 0 {
                            self.surface_blits.get(key).copied()
                        } else {
                            self.images.get(key).map(|cached| cached.texture.set)
                        };
                        let Some(set) = set else {
                            continue;
                        };
                        device.cmd_bind_descriptor_sets(
                            command_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            self.pipeline_layout,
                            1,
                            &[set],
                            &[],
                        );
                        device.cmd_draw(command_buffer, 4, 1, 0, start + offset);
                    }
                }
                crate::scene::Run::Gradients { start, len } => {
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipelines.gradient,
                    );
                    device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[self.gradient_buffer.buffer],
                        &[0],
                    );
                    device.cmd_draw(command_buffer, 4, len, 0, start);
                }
            }
        }
    }

    unsafe fn record_frame(&mut self, image_index: u32, passes: &[DrawPass]) -> Result<(), AshError> {
        let jobs = mem::take(&mut self.surface_jobs);
        let command_buffer = self.command_buffer;
        let device = &self.device;
        device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
        device.begin_command_buffer(
            command_buffer,
            &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )?;

        let clear = self.options.clear_color.to_linear();
        let swapchain_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [clear.r, clear.g, clear.b, clear.a],
            },
        };
        let surface_clear = {
            let color = Color::TRANSPARENT.to_linear();
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [color.r, color.g, color.b, color.a],
                },
            }
        };

        for job in &jobs {
            if !job.cached || !(job.binding.check_dirty)() {
                continue;
            }
            let Some(surface) = self.surface_textures.get(&job.binding.key) else {
                continue;
            };
            let (view, framebuffer) = (surface.view, surface.framebuffer);
            let render_pass = self.target_formats[&job.format].clear_pass;
            let extent = vk::Extent2D {
                width: job.content.width,
                height: job.content.height,
            };
            let area = vk::Rect2D::default().extent(extent);
            device.cmd_begin_render_pass(
                command_buffer,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(render_pass)
                    .framebuffer(framebuffer)
                    .render_area(area)
                    .clear_values(std::slice::from_ref(&surface_clear)),
                vk::SubpassContents::INLINE,
            );
            device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport::default()
                    .width(extent.width as f32)
                    .height(extent.height as f32)
                    .max_depth(1.0)],
            );
            device.cmd_set_scissor(command_buffer, 0, &[area]);
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.screen_set],
                &[job.slot as u32 * self.screen_stride as u32],
            );
            let mut frame = AshFrame {
                device,
                queue: &self.queue,
                command_buffer,
                render_pass,
                pipeline_layout: self.pipeline_layout,
                screen_set: self.screen_set,
                target: view,
                format: job.format,
                size: (extent.width, extent.height),
                scale: job.scale,
                region: AshRegion {
                    x: 0,
                    y: 0,
                    width: extent.width,
                    height: extent.height,
                },
                adapter: self.adapter.clone(),
                index: self.frame_index,
                delta: job.delta,
                redraw: &self.redraw,
            };
            (job.binding.draw)(&mut frame);
            device.cmd_end_render_pass(command_buffer);
            job.binding.mark_clean.borrow_mut()();
        }

        let (present_view, present_framebuffer, present_pass, present_pipelines, present_format) = {
            let state = self.swapchain.as_ref().expect("swapchain");
            let index = image_index as usize;
            (
                state.views[index],
                state.framebuffers[index],
                state.render_pass,
                state.pipelines,
                state.format,
            )
        };

        for pass in passes {
            let (framebuffer, render_pass, pipelines) = if let Some(target) = pass.target {
                let texture = self.render_targets.get(&target).expect("render target");
                let state = &self.target_formats[&pass.format];
                let render_pass = if pass.clear {
                    state.clear_pass
                } else {
                    state.load_pass
                };
                (texture.framebuffer, render_pass, state.pipelines)
            } else {
                (present_framebuffer, present_pass, present_pipelines)
            };

            let area = vk::Rect2D::default().extent(pass.extent);
            device.cmd_begin_render_pass(
                command_buffer,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(render_pass)
                    .framebuffer(framebuffer)
                    .render_area(area)
                    .clear_values(std::slice::from_ref(&swapchain_clear)),
                vk::SubpassContents::INLINE,
            );
            device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport::default()
                    .width(pass.extent.width as f32)
                    .height(pass.extent.height as f32)
                    .max_depth(1.0)],
            );
            device.cmd_set_scissor(command_buffer, 0, &[pass.scissor.unwrap_or(area)]);
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.screen_set],
                &[pass.screen_slot as u32 * self.screen_stride as u32],
            );

            self.record_runs(command_buffer, pipelines, pass.runs.clone());

            if pass.target.is_none() {
                for job in &jobs {
                    if job.cached {
                        continue;
                    }
                    let mut frame = AshFrame {
                        device,
                        queue: &self.queue,
                        command_buffer,
                        render_pass,
                        pipeline_layout: self.pipeline_layout,
                        screen_set: self.screen_set,
                        target: present_view,
                        format: present_format,
                        size: (pass.extent.width, pass.extent.height),
                        scale: job.scale,
                        region: job.content,
                        adapter: self.adapter.clone(),
                        index: self.frame_index,
                        delta: job.delta,
                        redraw: &self.redraw,
                    };
                    (job.binding.draw)(&mut frame);
                }
            }

            device.cmd_end_render_pass(command_buffer);
        }

        device.end_command_buffer(command_buffer)?;
        Ok(())
    }

    unsafe fn create_texture(&self, width: u32, height: u32) -> Result<Texture, AshError> {
        if width == 0 || height == 0 {
            return Err(AshError::ImageTooLarge { width, height });
        }
        let image = self.device.create_image(
            &vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED),
            None,
        )?;
        let requirements = self.device.get_image_memory_requirements(image);
        let memory_type = find_memory_type(
            &self.memory_properties,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .ok_or(AshError::Vulkan(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY))?;
        let memory = self.device.allocate_memory(
            &vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(memory_type),
            None,
        )?;
        if let Err(error) = self.device.bind_image_memory(image, memory, 0) {
            self.device.destroy_image(image, None);
            self.device.free_memory(memory, None);
            return Err(AshError::Vulkan(error));
        }
        let view = self.device.create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .components(vk::ComponentMapping::default())
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                ),
            None,
        )?;
        let sets = match self.device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.texture_layout)),
        ) {
            Ok(sets) => sets,
            Err(error) => {
                self.device.destroy_image_view(view, None);
                self.device.destroy_image(image, None);
                self.device.free_memory(memory, None);
                return Err(AshError::Vulkan(error));
            }
        };
        let set = sets[0];
        let image_info = [vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(view)];
        let sampler_info = [vk::DescriptorImageInfo::default().sampler(self.sampler)];
        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&sampler_info),
        ];
        self.device.update_descriptor_sets(&writes, &[]);
        Ok(Texture {
            image,
            memory,
            view,
            set,
        })
    }

    unsafe fn upload_texture(
        &self,
        texture: &Texture,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), AshError> {
        let staging = create_host_buffer(
            &self.device,
            &self.memory_properties,
            pixels.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        )?;
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), staging.mapped, pixels.len());

        let command_buffer = self.device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(self.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )?[0];

        let result = (|| {
            self.device.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            image_barrier(
                &self.device,
                command_buffer,
                texture.image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            );
            let region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(
                    vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1),
                )
                .image_offset(vk::Offset3D::default())
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                });
            self.device.cmd_copy_buffer_to_image(
                command_buffer,
                staging.buffer,
                texture.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
            image_barrier(
                &self.device,
                command_buffer,
                texture.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            );
            self.device.end_command_buffer(command_buffer)?;
            let command_buffers = [command_buffer];
            let submit = vk::SubmitInfo::default().command_buffers(&command_buffers);
            self.device
                .queue_submit(self.queue, &[submit], vk::Fence::null())?;
            self.device.queue_wait_idle(self.queue)?;
            Ok(())
        })();

        self.device
            .free_command_buffers(self.command_pool, &[command_buffer]);
        destroy_host_buffer(&self.device, &staging);
        result
    }

    unsafe fn destroy_texture(&self, texture: Texture) {
        let _ = self
            .device
            .free_descriptor_sets(self.descriptor_pool, &[texture.set]);
        self.device.destroy_image_view(texture.view, None);
        self.device.destroy_image(texture.image, None);
        self.device.free_memory(texture.memory, None);
    }
}

impl Drop for AshRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            if let Some(texture) = self.atlas_texture.take() {
                self.destroy_texture(texture);
            }
            let images: Vec<Texture> = self
                .images
                .drain()
                .map(|(_, cached)| cached.texture)
                .collect();
            for texture in images {
                self.destroy_texture(texture);
            }
            let surfaces: Vec<RenderTargetTexture> =
                self.surface_textures.drain().map(|(_, texture)| texture).collect();
            for texture in surfaces {
                self.destroy_render_target(texture);
            }
            let formats: Vec<TargetFormatState> =
                self.target_formats.drain().map(|(_, state)| state).collect();
            for state in formats {
                state.destroy(&self.device);
            }
            self.destroy_swapchain();

            destroy_host_buffer(&self.device, &self.screen_buffer);
            destroy_host_buffer(&self.device, &self.quad_buffer);
            destroy_host_buffer(&self.device, &self.glyph_buffer);
            destroy_host_buffer(&self.device, &self.blit_buffer);
            destroy_host_buffer(&self.device, &self.gradient_buffer);

            self.device.destroy_sampler(self.sampler, None);
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device
                .destroy_descriptor_set_layout(self.screen_layout, None);
            self.device
                .destroy_descriptor_set_layout(self.texture_layout, None);

            self.device.destroy_semaphore(self.image_available, None);
            self.device.destroy_semaphore(self.render_finished, None);
            self.device.destroy_fence(self.in_flight, None);
            self.device.destroy_command_pool(self.command_pool, None);

            self.device.destroy_device(None);

            let _ = (
                &self.physical_device,
                &self.swapchain_fn,
                &self.surface,
                &self.options,
            );
        }
    }
}

unsafe fn create_host_buffer(
    device: &Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    size: u64,
    usage: vk::BufferUsageFlags,
) -> Result<HostBuffer, AshError> {
    let size = size.max(4);
    let buffer = device.create_buffer(
        &vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE),
        None,
    )?;
    let requirements = device.get_buffer_memory_requirements(buffer);
    let Some(memory_type) = find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    ) else {
        device.destroy_buffer(buffer, None);
        return Err(AshError::Vulkan(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY));
    };
    let memory = match device.allocate_memory(
        &vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type),
        None,
    ) {
        Ok(memory) => memory,
        Err(error) => {
            device.destroy_buffer(buffer, None);
            return Err(AshError::Vulkan(error));
        }
    };
    if let Err(error) = device.bind_buffer_memory(buffer, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_buffer(buffer, None);
        return Err(AshError::Vulkan(error));
    }
    let mapped = match device.map_memory(memory, 0, size, vk::MemoryMapFlags::empty()) {
        Ok(mapped) => mapped.cast::<u8>(),
        Err(error) => {
            device.free_memory(memory, None);
            device.destroy_buffer(buffer, None);
            return Err(AshError::Vulkan(error));
        }
    };
    Ok(HostBuffer {
        buffer,
        memory,
        size: size as usize,
        mapped,
    })
}

unsafe fn destroy_host_buffer(device: &Device, buffer: &HostBuffer) {
    device.unmap_memory(buffer.memory);
    device.destroy_buffer(buffer.buffer, None);
    device.free_memory(buffer.memory, None);
}

impl HostBuffer {
    unsafe fn upload(
        &mut self,
        device: &Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        bytes: &[u8],
    ) -> Result<(), AshError> {
        if bytes.is_empty() {
            return Ok(());
        }
        if bytes.len() > self.size {
            let capacity = (bytes.len() as u64)
                .next_power_of_two()
                .max(INSTANCE_BUFFER_SEED);
            let replacement = create_host_buffer(
                device,
                memory_properties,
                capacity,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            )?;
            let old = mem::replace(self, replacement);
            destroy_host_buffer(device, &old);
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.mapped, bytes.len());
        Ok(())
    }
}

unsafe fn find_memory_type(
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    type_bits: u32,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    (0..memory_properties.memory_type_count).find(|index| {
        type_bits & (1 << index) != 0
            && memory_properties.memory_types[*index as usize]
                .property_flags
                .contains(flags)
    })
}

unsafe fn image_barrier(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    src_access: vk::AccessFlags,
    dst_access: vk::AccessFlags,
    src_stage: vk::PipelineStageFlags,
    dst_stage: vk::PipelineStageFlags,
) {
    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1),
        )
        .src_access_mask(src_access)
        .dst_access_mask(dst_access);
    device.cmd_pipeline_barrier(
        command_buffer,
        src_stage,
        dst_stage,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[barrier],
    );
}

unsafe fn create_pipeline(
    device: &Device,
    layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    format: vk::Format,
    module: vk::ShaderModule,
    vertex_entry: &CStr,
    fragment_entry: &CStr,
    stride: u32,
    attributes: &[(u32, vk::Format, u32)],
) -> Result<vk::Pipeline, AshError> {
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(module)
            .name(vertex_entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(module)
            .name(fragment_entry),
    ];
    let binding = vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(stride)
        .input_rate(vk::VertexInputRate::INSTANCE);
    let attributes: Vec<vk::VertexInputAttributeDescription> = attributes
        .iter()
        .map(|&(location, format, offset)| {
            vk::VertexInputAttributeDescription::default()
                .location(location)
                .binding(0)
                .format(format)
                .offset(offset)
        })
        .collect();
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(std::slice::from_ref(&binding))
        .vertex_attribute_descriptions(&attributes);
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
        .primitive_restart_enable(false);
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);
    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false)
        .line_width(1.0);
    let multisample = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let blend_attachment = [vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::ONE)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA)];
    let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .attachments(&blend_attachment);
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let _ = format;
    let info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisample)
        .color_blend_state(&color_blend)
        .dynamic_state(&dynamic)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0)
        .base_pipeline_index(-1);
    let pipelines = device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
        .map_err(|(_, error)| AshError::Vulkan(error))?;
    Ok(pipelines[0])
}
