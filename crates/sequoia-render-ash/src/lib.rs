//! Experimental pure-`ash` Vulkan renderer for Sequoia.
//!
//! This crate is a separate backend used alongside `sequoia-render`'s wgpu
//! implementation. It consumes the same [`sequoia_core::RenderList`] and
//! [`sequoia_text::GlyphAtlas`] and shares the WGSL renderer source compiled to
//! SPIR-V by the `sequoia-shader` crate.
//!
//! [`AshRenderer`] drives a window. [`AshSurface`] is the embeddable-surface
//! counterpart of wgpu's `WgpuSurface`: user code implements
//! [`AshSurfaceRenderer`] and receives an [`AshFrame`] with the raw Vulkan
//! device, queue and target view. Select this backend from `sequoia-platform`
//! with `App::render_backend(RenderBackend::VulkanAsh)` or the
//! `SEQUOIA_RENDER_BACKEND=ash` environment variable.
//!
//! Covered: quads, shadows, glyphs, images, gradients, rounded clips, render
//! targets, partial target damage and embedded surfaces. Backdrops, filters,
//! isolated opacity groups and custom WGSL shaders are not implemented yet.

#![allow(unsafe_op_in_unsafe_fn)]

mod error;
mod options;
mod renderer;
mod scene;
mod shader;
mod surface;

pub use ash::vk;
pub use error::AshError;
pub use options::{AshOptions, PresentMode};
pub use renderer::AshRenderer;
pub use surface::{
    AshAdapterInfo, AshFrame, AshRegion, AshSurface, AshSurfaceRenderer, VulkanSurface,
};

/// Explicit name for the Vulkan renderer, matching [`VulkanSurface`].
pub type VulkanRenderer = AshRenderer;
