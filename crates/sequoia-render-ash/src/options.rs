use sequoia_core::Color;

use crate::AshError;

/// Presentation pacing requested by the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentMode {
    /// Tear-free presentation via `VK_PRESENT_MODE_FIFO_KHR`.
    #[default]
    Vsync,
    /// Prefer mailbox, fall back to FIFO when the surface does not support it.
    LowLatency,
    /// Prefer immediate, fall back to FIFO when the surface does not support it.
    Immediate,
}

/// Configuration for [`crate::AshRenderer`].
#[derive(Clone)]
pub struct AshOptions {
    /// Presentation pacing.
    pub present_mode: PresentMode,
    /// Optional swapchain format. Only sRGB formats are honoured; other
    /// formats are ignored and the automatic sRGB pick is used instead.
    pub preferred_format: Option<ash::vk::Format>,
    /// Enable the `VK_LAYER_KHRONOS_validation` layer when available.
    pub validation: bool,
    /// Authored (sRGB) colour used to clear the swapchain image.
    pub clear_color: Color,
}

impl Default for AshOptions {
    fn default() -> Self {
        Self {
            present_mode: PresentMode::Vsync,
            preferred_format: None,
            validation: false,
            clear_color: Color::rgba(0.055, 0.063, 0.082, 1.0),
        }
    }
}

pub(crate) fn choose_surface_format(
    formats: &[ash::vk::SurfaceFormatKHR],
    preferred: Option<ash::vk::Format>,
) -> Result<ash::vk::SurfaceFormatKHR, AshError> {
    let is_srgb = |format: ash::vk::Format| {
        matches!(
            format,
            ash::vk::Format::B8G8R8A8_SRGB
                | ash::vk::Format::R8G8B8A8_SRGB
                | ash::vk::Format::A8B8G8R8_SRGB_PACK32
        )
    };

    if let Some(preferred) = preferred
        && is_srgb(preferred)
        && let Some(found) = formats.iter().find(|f| f.format == preferred)
    {
        return Ok(*found);
    }

    formats
        .iter()
        .copied()
        .find(|f| is_srgb(f.format))
        .ok_or(AshError::UnsupportedSwapchainFormat)
}

pub(crate) fn choose_present_mode(
    modes: &[ash::vk::PresentModeKHR],
    requested: PresentMode,
) -> ash::vk::PresentModeKHR {
    use ash::vk::PresentModeKHR as Mode;

    let has = |mode: Mode| modes.contains(&mode);
    match requested {
        PresentMode::Vsync => Mode::FIFO,
        PresentMode::LowLatency => {
            if has(Mode::MAILBOX) {
                Mode::MAILBOX
            } else {
                Mode::FIFO
            }
        }
        PresentMode::Immediate => {
            if has(Mode::IMMEDIATE) {
                Mode::IMMEDIATE
            } else {
                Mode::FIFO
            }
        }
    }
}
