use std::rc::Rc;

use sequoia_core::{Length, Size};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_reactive::Signal;

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime};
use crate::view::View;
use crate::views::Unit;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Breakpoint {
    #[default]
    Compact,

    Medium,

    Expanded,
}

impl Breakpoint {
    pub const COMPACT_MAX: f32 = 600.0;

    pub const MEDIUM_MAX: f32 = 840.0;

    pub const LARGE: f32 = 1200.0;

    pub fn of(width: f32) -> Self {
        Breakpoints::MATERIAL.of(width)
    }

    pub fn of_size(size: Size) -> Self {
        Self::of(size.width)
    }

    pub fn is_compact(self) -> bool {
        matches!(self, Self::Compact)
    }

    pub fn is_medium(self) -> bool {
        matches!(self, Self::Medium)
    }

    pub fn is_expanded(self) -> bool {
        matches!(self, Self::Expanded)
    }

    pub fn is_wide(size: Size) -> bool {
        size.width >= Self::LARGE
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Breakpoints {
    pub compact_max: f32,
    pub medium_max: f32,
}

impl Breakpoints {
    pub const MATERIAL: Self = Self {
        compact_max: Breakpoint::COMPACT_MAX,
        medium_max: Breakpoint::MEDIUM_MAX,
    };

    pub const fn new(compact_max: f32, medium_max: f32) -> Self {
        Self {
            compact_max,
            medium_max,
        }
    }

    pub fn of(self, width: f32) -> Breakpoint {
        if width < self.compact_max {
            Breakpoint::Compact
        } else if width < self.medium_max {
            Breakpoint::Medium
        } else {
            Breakpoint::Expanded
        }
    }

    pub fn of_size(self, size: Size) -> Breakpoint {
        self.of(size.width)
    }
}

impl Default for Breakpoints {
    fn default() -> Self {
        Self::MATERIAL
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScreenOrientation {
    Portrait,
    Landscape,
    #[default]
    Square,
}

pub fn screen_orientation_of(size: Size) -> ScreenOrientation {
    if size.width > size.height {
        ScreenOrientation::Landscape
    } else if size.height > size.width {
        ScreenOrientation::Portrait
    } else {
        ScreenOrientation::Square
    }
}

pub fn orientation_of(size: Size) -> ScreenOrientation {
    screen_orientation_of(size)
}

pub struct Responsive {
    window: Signal<Size>,
    build: Rc<dyn Fn(Size) -> AnyView>,
    style: LayoutStyle,
}

impl Responsive {
    pub fn new(window: Signal<Size>, build: impl Fn(Size) -> AnyView + 'static) -> Self {
        Self {
            window,
            build: Rc::new(build),
            style: LayoutStyle::default(),
        }
    }

    pub fn window(rt: &Runtime, build: impl Fn(Size) -> AnyView + 'static) -> Self {
        Self::new(rt.window_size(), build)
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn fill_size(mut self) -> Self {
        self.style = self
            .style
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0));
        self
    }

    pub fn fit(mut self) -> Self {
        self.style = self.style.fit();
        self
    }
}

impl Clone for Responsive {
    fn clone(&self) -> Self {
        Self {
            window: self.window.clone(),
            build: Rc::clone(&self.build),
            style: self.style,
        }
    }
}

impl View for Responsive {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let size = self.window.get();
        Some(vec![(0, (self.build)(size))])
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
    }
}

pub struct WithSize {
    size: Signal<Size>,
    build: Rc<dyn Fn(Size) -> AnyView>,
    style: LayoutStyle,
}

impl WithSize {
    pub fn new(build: impl Fn(Size) -> AnyView + 'static) -> Self {
        Self::with_signal(Signal::new(Size::ZERO), build)
    }

    pub fn with_initial(initial: Size, build: impl Fn(Size) -> AnyView + 'static) -> Self {
        Self::with_signal(Signal::new(initial), build)
    }

    pub fn with_signal(size: Signal<Size>, build: impl Fn(Size) -> AnyView + 'static) -> Self {
        Self {
            size,
            build: Rc::new(build),
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fit,
                ..LayoutStyle::default()
            },
        }
    }

    pub fn size_signal(&self) -> Signal<Size> {
        self.size.clone()
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn fill_size(mut self) -> Self {
        self.style = self
            .style
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0));
        self
    }

    pub fn fit(mut self) -> Self {
        self.style = self.style.fit();
        self
    }
}

impl Clone for WithSize {
    fn clone(&self) -> Self {
        Self {
            size: self.size.clone(),
            build: Rc::clone(&self.build),
            style: self.style,
        }
    }
}

impl View for WithSize {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let size = self.size.get();
        Some(vec![(0, (self.build)(size))])
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
    }

    fn arrange_override(
        &self,
        _rt: &Runtime,
        _el: ElementId,
        rect: RectAlias,
        _ctx: &mut LayoutCtx,
    ) -> bool {
        self.size.set(rect.size);
        false
    }
}

type RectAlias = sequoia_core::Rect;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoint_thresholds() {
        assert_eq!(Breakpoint::of(0.0), Breakpoint::Compact);
        assert_eq!(Breakpoint::of(599.9), Breakpoint::Compact);
        assert_eq!(Breakpoint::of(600.0), Breakpoint::Medium);
        assert_eq!(Breakpoint::of(839.9), Breakpoint::Medium);
        assert_eq!(Breakpoint::of(840.0), Breakpoint::Expanded);
        assert_eq!(Breakpoint::of(4000.0), Breakpoint::Expanded);
        assert!(Breakpoint::is_wide(Size::new(1200.0, 800.0)));
        assert!(!Breakpoint::is_wide(Size::new(1199.0, 800.0)));
    }

    #[test]
    fn custom_breakpoints() {
        let b = Breakpoints::new(500.0, 1000.0);
        assert_eq!(b.of(499.0), Breakpoint::Compact);
        assert_eq!(b.of(500.0), Breakpoint::Medium);
        assert_eq!(b.of(1000.0), Breakpoint::Expanded);
    }

    #[test]
    fn orientation_cases() {
        assert_eq!(
            screen_orientation_of(Size::new(800.0, 600.0)),
            ScreenOrientation::Landscape
        );
        assert_eq!(
            screen_orientation_of(Size::new(600.0, 800.0)),
            ScreenOrientation::Portrait
        );
        assert_eq!(
            screen_orientation_of(Size::new(600.0, 600.0)),
            ScreenOrientation::Square
        );
    }
}
