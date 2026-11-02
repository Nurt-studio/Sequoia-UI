//! Device and cache sharing for applications with several windows.
//!
//! A [`SharedGpu`] owns one wgpu instance, adapter, device, and queue. Every
//! renderer created through it uses the same device and the same image cache,
//! so an image is uploaded once no matter how many windows draw it. Pipelines, instance buffers, and
//! the glyph atlas stay per window because they depend on the surface format
//! and the window's text system.

use std::sync::{Arc, Mutex};

use winit::window::Window;

use crate::gpu::resources::ImageCache;
use crate::renderer::{GpuRenderer, GpuSurfaceSetup, request_device, select_backend};
use crate::types::*;

/// One GPU device shared by all renderers created from it.
pub struct SharedGpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
    backend: GpuBackend,
    images: crate::gpu::resources::SharedImageCache,
}

impl SharedGpu {
    /// Creates a device from the adapter used by `window`.
    ///
    /// The window only takes part in adapter selection; it is not configured
    /// here. Call [`SharedGpu::renderer`] to build a renderer for each window.
    pub fn new(window: Arc<Window>, options: GpuOptions) -> Result<Self, RenderError> {
        let (instance, surface, adapter) = select_backend(window, options)?;
        let info = adapter.get_info();
        log::info!(
            "sequoia-render: shared GPU '{}' ({:?}, {} {})",
            info.name,
            info.backend,
            info.driver,
            info.driver_info
        );
        drop(surface);
        let (device, queue) = request_device(&adapter)?;
        let budget = MemoryBudget::default();
        let images = Arc::new(Mutex::new(ImageCache::new(
            budget.images,
            budget.max_images,
        )));
        let backend = backend_of(&info);
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            info,
            backend,
            images,
        })
    }

    /// Returns the backend the shared device runs on.
    pub fn backend(&self) -> GpuBackend {
        self.backend
    }

    /// Returns information about the shared adapter.
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.info.clone()
    }

    /// Returns `true` when this device satisfies a backend request.
    pub fn supports(&self, requested: GpuBackend) -> bool {
        requested == GpuBackend::Auto || requested == self.backend
    }

    /// Creates a renderer for `window` that reuses this device and image cache.
    pub fn renderer(
        &self,
        window: Arc<Window>,
        width: u32,
        height: u32,
        options: GpuOptions,
    ) -> Result<GpuRenderer, RenderError> {
        GpuRenderer::create(window, width, height, options, Some(self))
    }

    pub(crate) fn surface_setup(
        &self,
        window: Arc<Window>,
    ) -> Result<GpuSurfaceSetup, RenderError> {
        let surface = self
            .instance
            .create_surface(window)
            .map_err(RenderError::CreateSurface)?;
        let caps = surface.get_capabilities(&self.adapter);
        if caps.formats.is_empty() {
            return Err(RenderError::NoAdapter);
        }
        let view_formats_supported = self
            .adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::SURFACE_VIEW_FORMATS);
        Ok(GpuSurfaceSetup {
            surface,
            adapter_info: self.info.clone(),
            caps,
            view_formats_supported,
            device: self.device.clone(),
            queue: self.queue.clone(),
            images: Arc::clone(&self.images),
        })
    }

    /// Returns the number of shared image textures.
    pub fn image_count(&self) -> usize {
        crate::gpu::resources::lock_images(&self.images).len()
    }

    /// Returns the bytes held by the shared image cache.
    pub fn image_bytes(&self) -> u64 {
        crate::gpu::resources::lock_images(&self.images).bytes()
    }
}

fn backend_of(info: &wgpu::AdapterInfo) -> GpuBackend {
    match info.backend {
        wgpu::Backend::Vulkan => GpuBackend::Vulkan,
        wgpu::Backend::Metal => GpuBackend::Metal,
        wgpu::Backend::Dx12 => GpuBackend::Dx12,
        wgpu::Backend::Gl => GpuBackend::Gl,
        wgpu::Backend::BrowserWebGpu => GpuBackend::WebGpu,
        _ => GpuBackend::Auto,
    }
}
