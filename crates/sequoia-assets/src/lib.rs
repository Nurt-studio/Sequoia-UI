#![forbid(unsafe_code)]

mod icons;
mod photo;
mod raster;
mod registry;
mod svg;

pub use crate::photo::{
    MAX_PHOTO_BYTES, MAX_PHOTO_DIM, PhotoError, decode_pixels, decode_pixels_thumbnail,
    load_pixels, load_pixels_thumbnail,
};
pub use crate::raster::Sprite;
pub use crate::registry::{AssetError, AssetRegistry, IconDirReport, icon_key};
pub use crate::svg::{
    FillRule, Seg, Shape, Stroke, StrokeLineCap, StrokeLineJoin, SubPath, SvgError, SvgIcon,
    SvgPoint, parse_svg,
};

/// Embeds an SVG file at compile time.
///
/// The path is relative to the source file that invokes the macro, exactly like
/// `include_str!`. Use it with `App::icon`:
///
/// ```ignore
/// App::new().icon("logo", include_svg!("../assets/icons/logo.svg"))
/// ```
#[macro_export]
macro_rules! include_svg {
    ($path:literal) => {
        include_str!($path)
    };
}

pub mod prelude {
    pub use crate::include_svg;
    pub use crate::photo::{
        MAX_PHOTO_BYTES, MAX_PHOTO_DIM, PhotoError, decode_pixels, decode_pixels_thumbnail,
        load_pixels, load_pixels_thumbnail,
    };
    pub use crate::raster::{MAX_RASTER_PX, Sprite};
    pub use crate::registry::{AssetError, AssetRegistry, IconDirReport, icon_key};
    pub use crate::svg::{SvgError, SvgIcon, parse_svg};
}
