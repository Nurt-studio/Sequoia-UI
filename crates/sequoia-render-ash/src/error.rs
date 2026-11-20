use std::fmt;

use ash::vk;

/// Errors reported by the experimental Vulkan renderer.
#[derive(Debug)]
#[non_exhaustive]
pub enum AshError {
    /// The Vulkan loader could not be opened.
    Load(ash::LoadingError),
    /// A Vulkan call failed.
    Vulkan(vk::Result),
    /// No physical device exposes a graphics + present queue with a swapchain.
    NoSuitableDevice,
    /// The window handle could not be converted into a raw handle.
    Window(String),
    /// The shader module could not be translated to SPIR-V.
    Shader(String),
    /// The surface does not expose any usable sRGB swapchain format.
    UnsupportedSwapchainFormat,
    /// An image exceeded the renderer's limits.
    ImageTooLarge { width: u32, height: u32 },
}

impl fmt::Display for AshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => write!(f, "cannot load the Vulkan loader: {error}"),
            Self::Vulkan(error) => write!(f, "Vulkan call failed: {error}"),
            Self::NoSuitableDevice => write!(
                f,
                "no Vulkan device supports a graphics queue, presentation, and VK_KHR_swapchain"
            ),
            Self::Window(error) => write!(f, "cannot obtain a raw window handle: {error}"),
            Self::Shader(error) => write!(f, "cannot compile the built-in WGSL shader: {error}"),
            Self::UnsupportedSwapchainFormat => {
                write!(f, "the surface exposes no sRGB colour format")
            }
            Self::ImageTooLarge { width, height } => {
                write!(f, "image {width}x{height} exceeds the renderer limits")
            }
        }
    }
}

impl std::error::Error for AshError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Load(error) => Some(error),
            _ => None,
        }
    }
}

impl From<vk::Result> for AshError {
    fn from(value: vk::Result) -> Self {
        Self::Vulkan(value)
    }
}

impl From<ash::LoadingError> for AshError {
    fn from(value: ash::LoadingError) -> Self {
        Self::Load(value)
    }
}
