#![forbid(unsafe_code)]

pub mod brush;
pub mod dirty;
pub mod geometry;
pub mod paint;
pub mod pixel;
pub mod time;
pub mod window;

pub use crate::brush::{
    Fill, GradientKind, GradientStop as BrushStop, GradientStops, MAX_GRADIENT_STOPS,
    Stroke as BrushStroke,
};

pub use crate::dirty::Dirty;

pub use crate::geometry::{
    Constraints, Edges, Length, LengthExt, Point, Rect, Size, Transform, Vec2,
};
pub use crate::paint::{
    BackdropDraw, BackdropSpan, Color, FilterDraw, FilterSpan, GlyphQuad, GradientDraw, ImageDraw,
    ImageSource, MAX_WAVEFORM_BARS, OpacityNode, OpacitySpan, Quad, RenderItem, RenderList,
    RoundedClip, Shadow, SpanMask, TextDraw, WaveformDraw,
};
pub use crate::pixel::Pixels;
pub use crate::window::WindowId;

pub mod prelude {
    pub use crate::brush::GradientStop as CoreGradientStop;
    pub use crate::brush::{Fill, GradientKind, GradientStops, Stroke as BrushStroke};
    pub use crate::dirty::Dirty;
    pub use crate::geometry::{
        Constraints, Edges, Length, LengthExt, Point, Rect, Size, Transform, Vec2,
    };
    pub use crate::paint::{
        BackdropDraw, BackdropSpan, Color, FilterDraw, FilterSpan, GlyphQuad, GradientDraw,
        ImageDraw, ImageSource, MAX_WAVEFORM_BARS, OpacityNode, OpacitySpan, Quad, RenderItem,
        RenderList, RoundedClip, Shadow, SpanMask, TextDraw, WaveformDraw,
    };
    pub use crate::pixel::Pixels;
    pub use crate::time::DurationExt;
    pub use crate::window::WindowId;
}
