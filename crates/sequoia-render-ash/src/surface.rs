//! Surfaces used by the experimental Vulkan renderer.
//!
//! [`VulkanSurface`] owns a native window surface and the Vulkan instance it
//! needs. [`AshSurface`] is the renderer-facing counterpart of wgpu's
//! `WgpuSurface`: a leaf view that lets user code record Vulkan commands into
//! the running UI.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::CStr;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use ash::{Entry, Instance, vk};
use sequoia_core::{Color, Edges, Length, Rect};
use sequoia_view::{KeyEvent, LayoutStyle, PointerEvent, SurfaceInputBinding, View};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use crate::{AshError, AshOptions};

/// A window-backed Vulkan surface plus its owning instance.
///
/// This is the Vulkan equivalent of wgpu's window surface handle. It is used
/// by [`crate::VulkanRenderer`] and can be built once and shared with
/// [`crate::VulkanRenderer::with_surface`].
pub struct VulkanSurface {
    entry: Entry,
    instance: Instance,
    surface_fn: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    window: Arc<Window>,
}

impl VulkanSurface {
    /// Creates an instance and surface with default options.
    pub fn new(window: Arc<Window>) -> Result<Self, AshError> {
        Self::with_options(window, &AshOptions::default())
    }

    /// Creates an instance and surface using `options`.
    pub fn with_options(window: Arc<Window>, options: &AshOptions) -> Result<Self, AshError> {
        unsafe { Self::init(window, options) }
    }

    unsafe fn init(window: Arc<Window>, options: &AshOptions) -> Result<Self, AshError> {
        let entry = Entry::load()?;

        let display_handle = window
            .display_handle()
            .map_err(|error| AshError::Window(format!("display handle: {error}")))?
            .as_raw();
        let window_handle = window
            .window_handle()
            .map_err(|error| AshError::Window(format!("window handle: {error}")))?
            .as_raw();

        let mut extensions: Vec<*const std::ffi::c_char> =
            ash_window::enumerate_required_extensions(display_handle)?.to_vec();

        let instance_extensions = entry.enumerate_instance_extension_properties(None)?;
        let has_instance_extension = |name: &CStr| {
            instance_extensions
                .iter()
                .any(|extension| CStr::from_ptr(extension.extension_name.as_ptr()) == name)
        };

        let mut instance_flags = vk::InstanceCreateFlags::empty();
        if has_instance_extension(vk::KHR_PORTABILITY_ENUMERATION_NAME) {
            extensions.push(vk::KHR_PORTABILITY_ENUMERATION_NAME.as_ptr());
            instance_flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
        }

        let mut layer_names: Vec<*const std::ffi::c_char> = Vec::new();
        if options.validation {
            let layers = entry.enumerate_instance_layer_properties()?;
            if let Some(layer) = layers.iter().find(|layer| {
                CStr::from_ptr(layer.layer_name.as_ptr()) == c"VK_LAYER_KHRONOS_validation"
            }) {
                layer_names.push(layer.layer_name.as_ptr());
            } else {
                log::warn!("sequoia-render-ash: VK_LAYER_KHRONOS_validation is not available");
            }
        }

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"sequoia-render-ash")
            .application_version(1)
            .engine_name(c"sequoia")
            .engine_version(1)
            .api_version(vk::make_api_version(0, 1, 0, 0));
        let instance_info = vk::InstanceCreateInfo::default()
            .flags(instance_flags)
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layer_names);
        let instance = entry.create_instance(&instance_info, None)?;

        let surface = match ash_window::create_surface(
            &entry,
            &instance,
            display_handle,
            window_handle,
            None,
        ) {
            Ok(surface) => surface,
            Err(error) => {
                instance.destroy_instance(None);
                return Err(AshError::Vulkan(error));
            }
        };
        let surface_fn = ash::khr::surface::Instance::new(&entry, &instance);

        Ok(Self {
            entry,
            instance,
            surface_fn,
            surface,
            window,
        })
    }

    /// Returns the raw Vulkan surface handle.
    #[must_use]
    pub fn raw(&self) -> vk::SurfaceKHR {
        self.surface
    }

    /// Returns the window this surface was created from.
    #[must_use]
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Returns the Vulkan entry used to create the surface.
    #[must_use]
    pub fn entry(&self) -> &Entry {
        &self.entry
    }

    /// Returns the owning Vulkan instance.
    #[must_use]
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    /// Returns the surface extension loader.
    #[must_use]
    pub fn surface_fn(&self) -> &ash::khr::surface::Instance {
        &self.surface_fn
    }

    /// Queries the surface capabilities for `physical_device`.
    pub fn capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceCapabilitiesKHR, AshError> {
        unsafe {
            Ok(self
                .surface_fn
                .get_physical_device_surface_capabilities(physical_device, self.surface)?)
        }
    }

    /// Queries the supported surface formats for `physical_device`.
    pub fn formats(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::SurfaceFormatKHR>, AshError> {
        unsafe {
            Ok(self
                .surface_fn
                .get_physical_device_surface_formats(physical_device, self.surface)?)
        }
    }

    /// Queries the supported present modes for `physical_device`.
    pub fn present_modes(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::PresentModeKHR>, AshError> {
        unsafe {
            Ok(self
                .surface_fn
                .get_physical_device_surface_present_modes(physical_device, self.surface)?)
        }
    }

    /// Returns whether `queue_family` can present to this surface.
    pub fn supports_present(
        &self,
        physical_device: vk::PhysicalDevice,
        queue_family: u32,
    ) -> Result<bool, AshError> {
        unsafe {
            Ok(self.surface_fn.get_physical_device_surface_support(
                physical_device,
                queue_family,
                self.surface,
            )?)
        }
    }
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        unsafe {
            self.surface_fn.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

/// An integer rectangle in physical pixels, used by [`AshFrame`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AshRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl AshRegion {
    pub(crate) fn from_phys(rect: Rect) -> Option<Self> {
        if !rect.left().is_finite()
            || !rect.top().is_finite()
            || !rect.size.width.is_finite()
            || !rect.size.height.is_finite()
        {
            return None;
        }
        let x = rect.left().floor().clamp(0.0, 16384.0) as u32;
        let y = rect.top().floor().clamp(0.0, 16384.0) as u32;
        let width = (rect.right().ceil() - rect.left().floor()).clamp(0.0, 16384.0) as u32;
        let height = (rect.bottom().ceil() - rect.top().floor()).clamp(0.0, 16384.0) as u32;
        (width > 0 && height > 0).then_some(Self {
            x,
            y,
            width,
            height,
        })
    }
}

/// Basic information about the physical device driving the renderer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AshAdapterInfo {
    pub name: String,
    pub vendor: u32,
    pub device: u32,
}

/// State handed to an [`AshSurfaceRenderer`] for one draw call.
///
/// The renderer is already inside a Vulkan render pass and has started the
/// command buffer. Implementations record commands with `command_buffer` into
/// `target` and must leave the pipeline state in a valid condition for the
/// engine (the engine rebinds its own state before the next draw).
pub struct AshFrame<'a> {
    pub device: &'a ash::Device,
    pub queue: &'a vk::Queue,
    pub command_buffer: vk::CommandBuffer,
    pub render_pass: vk::RenderPass,
    pub pipeline_layout: vk::PipelineLayout,
    pub screen_set: vk::DescriptorSet,
    pub target: vk::ImageView,
    pub format: vk::Format,
    pub size: (u32, u32),
    pub scale: f32,
    pub region: AshRegion,
    pub adapter: AshAdapterInfo,
    pub index: u64,
    pub delta: Duration,
    pub(crate) redraw: &'a Cell<bool>,
}

impl AshFrame<'_> {
    /// Requests another frame from the host renderer.
    pub fn request_frame(&self) {
        self.redraw.set(true);
    }
}

/// User drawing callback invoked for each [`AshSurface`].
pub trait AshSurfaceRenderer {
    /// Records Vulkan commands for one frame.
    fn render(&mut self, frame: &mut AshFrame<'_>);

    /// Returns `true` when cached content needs to be redrawn.
    fn is_dirty(&self) -> bool {
        true
    }

    /// Marks cached content as up to date.
    fn mark_clean(&mut self) {}
}

/// Visual decoration drawn around a surface.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SurfaceFrame {
    pub(crate) background: Option<[f32; 4]>,
    pub(crate) border: Option<([f32; 4], f32)>,
    pub(crate) radius: f32,
    pub(crate) inset: [f32; 4],
}

/// Type-erased binding between an [`AshSurface`] and `VulkanRenderer`.
pub(crate) struct AshSurfaceBinding {
    pub(crate) draw: Box<dyn Fn(&mut AshFrame<'_>)>,
    pub(crate) frame: SurfaceFrame,
    pub(crate) key: usize,
    pub(crate) cached: bool,
    pub(crate) format: Option<vk::Format>,
    pub(crate) check_dirty: Box<dyn Fn() -> bool>,
    pub(crate) mark_clean: RefCell<Box<dyn FnMut()>>,
}

/// A view that embeds an [`AshSurfaceRenderer`] in the Sequoia layout.
///
/// The surface fills its parent by default. Cloning shares the renderer.
/// When the wgpu backend is selected these views are ignored, just as
/// `WgpuSurface` views are ignored by the Vulkan backend.
pub struct AshSurface<R> {
    pub(crate) renderer: Rc<RefCell<R>>,
    pub(crate) style: LayoutStyle,
    pub(crate) background: Color,
    pub(crate) border_color: Color,
    pub(crate) border_width: f32,
    pub(crate) radius: f32,
    pub(crate) padding: Edges,
    pub(crate) cached: bool,
    pub(crate) format: Option<vk::Format>,
    pub(crate) on_pointer: Option<Rc<dyn Fn(&PointerEvent, Rect)>>,
    pub(crate) on_key: Option<Rc<dyn Fn(&KeyEvent)>>,
    pub(crate) click_through: bool,
    pub(crate) focusable_flag: bool,
}

impl<R> Clone for AshSurface<R> {
    fn clone(&self) -> Self {
        Self {
            renderer: Rc::clone(&self.renderer),
            style: self.style,
            background: self.background,
            border_color: self.border_color,
            border_width: self.border_width,
            radius: self.radius,
            padding: self.padding,
            cached: self.cached,
            format: self.format,
            on_pointer: self.on_pointer.clone(),
            on_key: self.on_key.clone(),
            click_through: self.click_through,
            focusable_flag: self.focusable_flag,
        }
    }
}

impl<R: AshSurfaceRenderer + 'static> AshSurface<R> {
    /// Creates a surface that fills its parent and shares `renderer`.
    pub fn new(renderer: R) -> Self {
        Self {
            renderer: Rc::new(RefCell::new(renderer)),
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
            background: Color::TRANSPARENT,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            radius: 0.0,
            padding: Edges::ZERO,
            cached: false,
            format: None,
            on_pointer: None,
            on_key: None,
            click_through: false,
            focusable_flag: false,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width.max(0.0);
        self.border_color = color;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.padding = Edges::xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.padding.left = x;
        self.padding.right = x;
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.padding.top = y;
        self.padding.bottom = y;
        self
    }

    pub fn cached(mut self, cached: bool) -> Self {
        self.cached = cached;
        self
    }

    /// Renders the surface into an offscreen image with `format`.
    pub fn format(mut self, format: vk::Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Returns the requested offscreen format, if any.
    pub fn requested_format(&self) -> Option<vk::Format> {
        self.format
    }

    pub fn on_pointer(mut self, f: impl Fn(&PointerEvent, Rect) + 'static) -> Self {
        self.on_pointer = Some(Rc::new(f));
        self
    }

    pub fn on_key(mut self, f: impl Fn(&KeyEvent) + 'static) -> Self {
        self.on_key = Some(Rc::new(f));
        self
    }

    pub fn click_through(mut self, click_through: bool) -> Self {
        self.click_through = click_through;
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable_flag = focusable;
        self
    }

    pub fn renderer(&self) -> Rc<RefCell<R>> {
        Rc::clone(&self.renderer)
    }
}

impl<R: AshSurfaceRenderer + 'static> View for AshSurface<R> {
    fn body(&self) -> impl View + 'static {
        sequoia_view::Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
    }

    fn captures_pointer(&self) -> bool {
        !self.click_through
    }

    fn focusable(&self) -> bool {
        self.focusable_flag
    }

    fn surface_input(&self) -> Option<SurfaceInputBinding> {
        if self.on_pointer.is_none() && self.on_key.is_none() {
            return None;
        }
        let pointer = self.on_pointer.clone();
        let key = self.on_key.clone();
        Some(SurfaceInputBinding {
            on_pointer: Box::new(move |ev, area| {
                if let Some(f) = &pointer {
                    f(ev, area);
                }
            }),
            on_key: Box::new(move |ev| {
                if let Some(f) = &key {
                    f(ev);
                }
            }),
        })
    }

    fn surface_target(&self) -> Option<Rc<dyn Any>> {
        fn rgba(color: Color) -> [f32; 4] {
            let linear = color.to_linear();
            [linear.r, linear.g, linear.b, linear.a]
        }
        let has_border = self.border_width > 0.0 && self.border_color.a > 0.0;
        let bw = if has_border { self.border_width } else { 0.0 };
        let renderer = Rc::clone(&self.renderer);
        let key = Rc::as_ptr(&renderer) as usize;
        let dirty = Rc::clone(&renderer);
        let clean = Rc::clone(&renderer);
        let binding: Rc<dyn Any> = Rc::new(AshSurfaceBinding {
            draw: Box::new(move |frame| renderer.borrow_mut().render(frame)),
            frame: SurfaceFrame {
                background: (self.background.a > 0.0).then(|| rgba(self.background)),
                border: has_border.then(|| (rgba(self.border_color), self.border_width)),
                radius: self.radius,
                inset: [
                    self.padding.left + bw,
                    self.padding.top + bw,
                    self.padding.right + bw,
                    self.padding.bottom + bw,
                ],
            },
            key,
            cached: self.cached,
            format: self.format,
            check_dirty: Box::new(move || dirty.borrow().is_dirty()),
            mark_clean: RefCell::new(Box::new(move || clean.borrow_mut().mark_clean())),
        });
        Some(binding)
    }
}
