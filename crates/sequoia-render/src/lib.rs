#![forbid(unsafe_code)]

pub mod gpu;
pub mod pass;
pub mod renderer;
pub mod scene;
pub mod surface;
pub mod types;
pub mod utils;

#[cfg(test)]
mod tests;

pub use renderer::GpuRenderer;
pub use surface::{PendingFrames, Revision, WgpuFrame, WgpuRenderer, WgpuSurface};
pub use types::{
    FrameSkip, GpuBackend, GpuOptions, PowerPreference, PresentMode, Region, RenderError,
    RenderStats,
};
