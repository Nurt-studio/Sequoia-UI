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
pub use crate::registry::{AssetError, AssetRegistry, icon_key};
pub use crate::svg::{
    FillRule, Seg, Shape, Stroke, StrokeLineCap, StrokeLineJoin, SubPath, SvgError, SvgIcon,
    SvgPoint, parse_svg,
};

pub mod prelude {
    pub use crate::photo::{
        MAX_PHOTO_BYTES, MAX_PHOTO_DIM, PhotoError, decode_pixels, decode_pixels_thumbnail,
        load_pixels, load_pixels_thumbnail,
    };
    pub use crate::raster::{MAX_RASTER_PX, Sprite};
    pub use crate::registry::{AssetError, AssetRegistry, icon_key};
    pub use crate::svg::{SvgError, SvgIcon, parse_svg};
}
