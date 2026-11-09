//! Resize borders for the undecorated main window.
//!
//! The native frame is disabled, so the outermost strips of the client area
//! act as resize handles. They sit above everything else in the root stack so
//! edge drags always win over the content underneath.

use sequoia_ui::layout::{Inset, LayoutStyle, Position};
use sequoia_ui::prelude::*;

const EDGE: f32 = 4.0;
const CORNER: f32 = 8.0;

#[derive(Clone, Copy)]
enum Zone {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Zone {
    fn direction(self) -> ResizeDirection {
        match self {
            Self::North => ResizeDirection::North,
            Self::NorthEast => ResizeDirection::NorthEast,
            Self::East => ResizeDirection::East,
            Self::SouthEast => ResizeDirection::SouthEast,
            Self::South => ResizeDirection::South,
            Self::SouthWest => ResizeDirection::SouthWest,
            Self::West => ResizeDirection::West,
            Self::NorthWest => ResizeDirection::NorthWest,
        }
    }

    fn cursor(self) -> CursorIcon {
        match self {
            Self::North | Self::South => CursorIcon::RowResize,
            Self::East | Self::West => CursorIcon::ColResize,
            Self::NorthWest | Self::SouthEast => CursorIcon::NwseResize,
            Self::NorthEast | Self::SouthWest => CursorIcon::NeswResize,
        }
    }

    fn style(self) -> LayoutStyle {
        let edge = Some(EDGE);
        let corner = Some(CORNER);
        let inset = match self {
            Self::North => Inset::only(Some(0.0), Some(0.0), Some(0.0), None),
            Self::South => Inset::only(Some(0.0), None, Some(0.0), Some(0.0)),
            Self::East => Inset::only(None, Some(0.0), Some(0.0), Some(0.0)),
            Self::West => Inset::only(Some(0.0), Some(0.0), None, Some(0.0)),
            Self::NorthWest => Inset::only(Some(0.0), Some(0.0), None, None),
            Self::NorthEast => Inset::only(None, Some(0.0), Some(0.0), None),
            Self::SouthWest => Inset::only(Some(0.0), None, None, Some(0.0)),
            Self::SouthEast => Inset::only(None, None, Some(0.0), Some(0.0)),
        };
        let (width, height) = match self {
            Self::North | Self::South => (None, edge),
            Self::East | Self::West => (edge, None),
            _ => (corner, corner),
        };
        let mut style = LayoutStyle::default()
            .position(Position::Absolute(inset))
            .hit_testable(true);
        if let Some(width) = width {
            style = style.width(Length::Px(width));
        }
        if let Some(height) = height {
            style = style.height(Length::Px(height));
        }
        style
    }
}

#[derive(Clone, Copy)]
struct ResizeZone {
    zone: Zone,
}

impl Leaf for ResizeZone {
    fn layout_style(&self) -> LayoutStyle {
        self.zone.style()
    }

    fn paint(&self, _ctx: &mut PaintCtx) {}

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            ..
        }) = event
        {
            eprintln!("resize zone down: {:?}", self.zone.direction());
            if let Some(windows) = ctx.environment.get::<Windows>() {
                eprintln!(
                    "begin_resize queued: {}",
                    windows.begin_resize(ctx.window, self.zone.direction())
                );
            } else {
                eprintln!("no Windows in environment");
            }
            ctx.handled();
        }
    }

    fn cursor(&self, _state: VisualState) -> CursorIcon {
        self.zone.cursor()
    }
}
impl_view_for_leaf!(ResizeZone);

fn zone(zone: Zone) -> ResizeZone {
    ResizeZone { zone }
}

/// Full-size transparent layer with the eight resize strips around the edge.
pub(crate) fn resize_borders() -> impl View + 'static {
    Stack::new()
        .fill_size()
        .hit_testable(false)
        .child(zone(Zone::North))
        .child(zone(Zone::South))
        .child(zone(Zone::East))
        .child(zone(Zone::West))
        .child(zone(Zone::NorthWest))
        .child(zone(Zone::NorthEast))
        .child(zone(Zone::SouthWest))
        .child(zone(Zone::SouthEast))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_test::Harness;

    #[test]
    fn edges_get_resize_cursors_and_the_middle_stays_clear() {
        let h = Harness::new(resize_borders());
        h.layout(800.0, 600.0);

        h.pointer_move(798.0, 300.0);
        assert_eq!(h.cursor_icon(), CursorIcon::ColResize);
        h.pointer_move(2.0, 300.0);
        assert_eq!(h.cursor_icon(), CursorIcon::ColResize);
        h.pointer_move(400.0, 2.0);
        assert_eq!(h.cursor_icon(), CursorIcon::RowResize);
        h.pointer_move(400.0, 598.0);
        assert_eq!(h.cursor_icon(), CursorIcon::RowResize);
        h.pointer_move(2.0, 2.0);
        assert_eq!(h.cursor_icon(), CursorIcon::NwseResize);
        h.pointer_move(798.0, 598.0);
        assert_eq!(h.cursor_icon(), CursorIcon::NwseResize);
        h.pointer_move(798.0, 2.0);
        assert_eq!(h.cursor_icon(), CursorIcon::NeswResize);
        h.pointer_move(2.0, 598.0);
        assert_eq!(h.cursor_icon(), CursorIcon::NeswResize);

        h.pointer_move(400.0, 300.0);
        assert_eq!(h.cursor_icon(), CursorIcon::Default);
    }
}
