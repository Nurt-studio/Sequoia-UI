#![forbid(unsafe_code)]

pub mod a11y;
mod animated;
mod any;
pub mod app;
pub mod async_view;
mod backdrop;
mod canvas;
mod clock;
pub mod dev_overlay;
mod dnd;
mod dock;
pub mod dock_float;
pub mod dock_view;
pub mod dock_window;
pub mod dockable;
pub mod edge_dock;
pub mod error_boundary;
mod filter_view;
mod focus_ring;
mod hit_shape;
mod icon;
mod image;
mod layout_live;
mod leaf;
mod list;
pub mod live;
mod live_view;
mod modifiers;
mod overlay;
mod paint_draw;
mod painter;
mod portal;
mod presence;
mod responsive;
mod rich_text;
mod runtime;
mod scroll_view;
mod scrub;
mod split_panel;
mod styleable;
mod surface;
pub mod table;
pub mod thumbnail;
mod track;
mod transition;
pub mod tree;
mod value;
mod view;
mod views;
mod virtual_list;

pub use a11y::{Accessible, AccessibleNode, AccessibleRole};
pub use animated::{
    Animated, MAX_ANIM_DT, Opacity, Pulse, Shake, Translate, fade_sample, pop_sample, pulse_sample,
    shake_sample, slide_progress,
};
pub use any::AnyView;
pub use async_view::AsyncView;
pub use backdrop::BackdropBlur;
pub use canvas::{Canvas, KeyBinding};
pub use clock::{FrameClock, MAX_FRAME_STEP_SECS};
pub use dev_overlay::DevOverlay;
pub use dnd::{
    DRAG_THRESHOLD, DragEnd, DragId, DragOptions, DragPayload, DragSession, DropPosition,
    exceeded_threshold, grab_offset, insertion_index, move_item, preview_origin,
};
pub use dock::DockArea;
pub use dock_float::DockFloatLayer;
pub use dock_view::{DockSlot, DockView};
pub use dock_window::{DetachedWindow, DockWindowHost, DockWindowRoot};
pub use dockable::{DockHeader, DockPanel, Dockable};
pub use edge_dock::{
    EDGE_GHOST_PX, EDGE_HINT_PX, EDGE_LINE_PX, EdgeGhost, EdgeSide, EdgeSlab, MIN_VISIBLE_H,
    MIN_VISIBLE_W, UNDOCK_THRESHOLD_PX, clamp_card_top_left, edge_ghost_rect, edge_hint_at,
    exceeded_undock, main_first_index,
};
pub use error_boundary::ErrorBoundary;
pub use filter_view::{
    BackdropFiltered, DrawGradientMask, EffectNode, FilterExt, Filtered, draw_gradient_mask,
};
pub use focus_ring::{FocusRing, RING_OFFSET, RING_WIDTH, ring_rect};
pub use hit_shape::{HitCircle, HitRect};
pub use icon::Icon;
pub use image::Image;
pub use layout_live::AnimatedSize;
pub use leaf::Leaf;
pub use list::List;
pub use live::{AnimSpec, Live, MemoLiveExt, Prop, SignalLiveExt, SpringSpec};
pub use live_view::{LiveBackground, LiveExt, LiveOffset, LiveOpacity};
pub use modifiers::{
    Clip, Enter, Motion, Rotated, Scaled, StatePatch, States, ViewModifiers, When, fade, slide_y,
    when,
};
pub use overlay::{
    Anchor, Dismiss, InputMode, OverlayEntry, OverlayId, OverlayLayer, OverlayOverflow, Placement,
    Position,
};
pub use painter::Painter;
pub use portal::{DismissableLayer, Popper, Portal};
pub use presence::{
    ENTER_SECS, EXIT_SECS, Presence, enter_progress, enter_sample, exit_done, exit_progress,
    exit_sample,
};
pub use responsive::{
    Breakpoint, Breakpoints, Responsive, ScreenOrientation, WithSize, orientation_of,
    screen_orientation_of,
};
pub use rich_text::RichText;
pub use runtime::{ElementId, InteractState, Runtime, RuntimeError, Stats, ViewState};
pub use scroll_view::{ScrollAxis, ScrollView, ScrollbarVisibility};
pub use scrub::Scrub;
pub use sequoia_animation::Easing;
pub use sequoia_dock::{
    DecodeError, DockDrop, DockModel, DockNode, DockOp, FloatingPanel, PanelId, PanelPayload,
    SplitAxis, decode as dock_decode, drop_from_fraction, drop_from_fraction_or_float,
    encode as dock_encode, preview_rect as dock_preview_rect,
};
pub use split_panel::SplitPanel;
pub use styleable::{Decoratable, Styleable};
pub use surface::{SurfaceDraw, SurfaceInputBinding};
pub use table::{SortDir, Table, TableColumn, table_filters};
pub use thumbnail::{ThumbnailCache, shared_cache};
pub use track::{Orientation, Pad2D, Track1D};
pub use transition::{Sequence, SequenceFrame, fade_in, pop_in, slide_in, stagger};
pub use tree::{FlatRow, TreeNode, TreeView};
pub use value::{Value, ValueExt, derived, spring_spec, value};
pub use view::{CustomLayout, EventCtx, PaintCtx, TextMetrics, View};
pub use views::{
    AspectRatio, Column, FocusScope, Fragment, Grid, IntoChildren, Margined, Panel, Row, Spacer,
    Stack, StackPos, Text, Unit, Wrap,
};
pub use virtual_list::VirtualList;

pub use sequoia_assets::{
    AssetRegistry, MAX_PHOTO_DIM, PhotoError, Sprite, decode_pixels, decode_pixels_thumbnail,
    icon_key, load_pixels, load_pixels_thumbnail,
};

pub use sequoia_core::{
    BrushStroke, Fill, GradientKind, GradientStops, MAX_GRADIENT_STOPS, Pixels, Size, WindowId,
};

pub use sequoia_draw::{
    Dash, FillRule, Gradient, GradientStop, LineCap, LineJoin, MinMax, Path, Peaks, Radius,
    RasterOptions, Shape, ShapeCache, ShapeStats, Span, StrokeStyle, Transform, WaveformCache,
    WaveformOpts, bake_strip, layout_bars, waveform_line_points, white_noise,
};

pub use paint_draw::GRADIENT_BAND_PX;

pub use sequoia_input::{
    CursorIcon, DropEffect, Event, EventPhase, Key, KeyEvent, Modifiers, PenTool, PointerButton,
    PointerEvent, effect_for_modifiers,
};
pub use sequoia_layout::{
    Align, Axis, ContainerStyle, Dock, DockStyle, FlexItem, GridFlow, GridPlacement, GridStyle,
    Indents, Inset, Justify, LayoutCtx, LayoutStyle, Overflow, Track,
};
pub use sequoia_style::{
    Backdrop, BorderSide, BoxStyle, Effect, Filter, Mask, ShadowSpec, VisualState,
};
pub use sequoia_text::{
    ElideMode, FontStyle, FontWeight, LetterSpacing, RichAlign, RichCaret, RichHit, RichLine,
    RichParagraph, RichRect, TextSpan, WrapMode,
};

pub mod prelude {
    pub use crate::a11y::{Accessible, AccessibleNode, AccessibleRole};
    pub use crate::animated::{Animated, Opacity, Pulse, Shake, Translate};
    pub use crate::any::AnyView;
    pub use crate::async_view::AsyncView;
    pub use crate::backdrop::BackdropBlur;
    pub use crate::canvas::{Canvas, KeyBinding};
    pub use crate::dev_overlay::DevOverlay;
    pub use crate::dnd::{
        DRAG_THRESHOLD, DragEnd, DragId, DragOptions, DragPayload, DragSession, DropPosition,
        exceeded_threshold, grab_offset, insertion_index, move_item, preview_origin,
    };
    pub use crate::dock::DockArea;
    pub use crate::dock_float::DockFloatLayer;
    pub use crate::dock_view::{DockSlot, DockView};
    pub use crate::dock_window::{DetachedWindow, DockWindowHost, DockWindowRoot};
    pub use crate::dockable::{DockHeader, DockPanel, Dockable};
    pub use crate::edge_dock::{
        EDGE_GHOST_PX, EDGE_HINT_PX, EDGE_LINE_PX, EdgeGhost, EdgeSide, EdgeSlab,
        clamp_card_top_left, edge_ghost_rect, edge_hint_at, exceeded_undock, main_first_index,
    };
    pub use crate::error_boundary::ErrorBoundary;
    pub use crate::filter_view::{
        BackdropFiltered, EffectNode, FilterExt, Filtered, draw_gradient_mask,
    };
    pub use crate::focus_ring::{FocusRing, RING_OFFSET, RING_WIDTH, ring_rect};
    pub use crate::hit_shape::{HitCircle, HitRect};
    pub use crate::icon::Icon;
    pub use crate::image::Image;
    pub use crate::layout_live::AnimatedSize;
    pub use crate::leaf::Leaf;
    pub use crate::list::List;
    pub use crate::live::{AnimSpec, Live, MemoLiveExt, Prop, SignalLiveExt, SpringSpec};
    pub use crate::live_view::{LiveBackground, LiveExt, LiveOffset, LiveOpacity};
    pub use crate::modifiers::{
        Clip, Enter, Motion, Rotated, Scaled, StatePatch, States, ViewModifiers, When, fade,
        slide_y, when,
    };
    pub use crate::overlay::{
        Anchor, Dismiss, InputMode, OverlayEntry, OverlayId, OverlayLayer, OverlayOverflow,
        Placement, Position,
    };
    pub use crate::painter::Painter;
    pub use crate::portal::{DismissableLayer, Popper, Portal};
    pub use crate::presence::{
        ENTER_SECS, EXIT_SECS, Presence, enter_progress, enter_sample, exit_done, exit_progress,
        exit_sample,
    };
    pub use crate::responsive::{
        Breakpoint, Breakpoints, Responsive, ScreenOrientation, WithSize, orientation_of,
        screen_orientation_of,
    };
    pub use crate::rich_text::RichText;
    pub use crate::runtime::{ElementId, InteractState, Runtime, RuntimeError, Stats, ViewState};
    pub use crate::scroll_view::{ScrollAxis, ScrollView, ScrollbarVisibility};
    pub use crate::scrub::Scrub;
    pub use crate::split_panel::SplitPanel;
    pub use crate::styleable::{Decoratable, Styleable};
    pub use crate::surface::{SurfaceDraw, SurfaceInputBinding};
    pub use crate::table::{SortDir, Table, TableColumn};
    pub use crate::thumbnail::{ThumbnailCache, shared_cache};
    pub use crate::track::{Orientation, Pad2D, Track1D};
    pub use crate::transition::{Sequence, fade_in, pop_in, slide_in, stagger};
    pub use crate::tree::{FlatRow, TreeNode, TreeView};
    pub use crate::value::{Value, ValueExt, derived, spring_spec, value};
    pub use crate::view::{CustomLayout, EventCtx, PaintCtx, TextMetrics, View};
    pub use crate::views::{
        AspectRatio, Column, FocusScope, Fragment, Grid, IntoChildren, Margined, Panel, Row,
        Spacer, Stack, StackPos, Text, Unit, Wrap,
    };
    pub use crate::virtual_list::VirtualList;
    pub use sequoia_animation::Easing;

    pub use sequoia_assets::{
        AssetRegistry, MAX_PHOTO_DIM, PhotoError, Sprite, decode_pixels, decode_pixels_thumbnail,
        load_pixels, load_pixels_thumbnail,
    };
    pub use sequoia_core::BrushStop;
    pub use sequoia_core::{
        BrushStroke, Fill, GradientKind, GradientStops, MAX_GRADIENT_STOPS, Pixels, WindowId,
    };
    pub use sequoia_draw::{
        Dash, FillRule, Gradient, GradientStop, LineCap, LineJoin, MinMax, Path, Peaks, Radius,
        RasterOptions, Shape, ShapeCache, StrokeStyle, Transform, WaveformCache, WaveformOpts,
        bake_strip, layout_bars, waveform_line_points, white_noise,
    };
    pub use sequoia_input::{
        CursorIcon, DropEffect, Event, EventPhase, Key, KeyEvent, Modifiers, PenTool,
        PointerButton, PointerEvent, effect_for_modifiers,
    };
    pub use sequoia_layout::{
        Align, Axis, ContainerStyle, Dock, DockStyle, FlexItem, GridFlow, GridPlacement, GridStyle,
        Indents, Inset, Justify, LayoutCtx, LayoutStyle, Overflow, Track, WrapStyle,
    };
    pub use sequoia_style::{Backdrop, BorderSide, BoxStyle, Effect, Filter, Mask, ShadowSpec};
    pub use sequoia_text::{
        ElideMode, FontStyle, FontWeight, LetterSpacing, RichAlign, RichCaret, RichHit, RichLine,
        RichParagraph, RichRect, TextSpan, WrapMode,
    };
}
