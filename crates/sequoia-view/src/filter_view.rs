#![forbid(unsafe_code)]

use sequoia_core::Length;
use sequoia_layout::LayoutStyle;
use sequoia_style::{Backdrop, Effect, Filter};

use crate::any::AnyView;
use crate::view::{PaintCtx, View};

pub struct DrawGradientMask<'a>(pub &'a sequoia_draw::Gradient);

impl sequoia_style::GradientAlphas for DrawGradientMask<'_> {
    fn endpoint_alphas(&self) -> (f32, f32) {
        let stops = self.0.stops();
        if stops.is_empty() {
            return (1.0, 1.0);
        }
        (
            stops.first().map(|s| s.color.a).unwrap_or(1.0),
            stops.last().map(|s| s.color.a).unwrap_or(1.0),
        )
    }

    fn direction(&self) -> (f32, f32) {
        let d = self.0.direction();
        (d.x, d.y)
    }
}

pub fn draw_gradient_mask(gradient: &sequoia_draw::Gradient) -> sequoia_style::Mask {
    sequoia_style::gradient_mask(DrawGradientMask(gradient))
}

#[derive(Clone)]
pub struct Filtered {
    child: AnyView,
    filter: Filter,
}

impl Filtered {
    pub fn new<V: View + 'static>(child: V, filter: Filter) -> Self {
        Self {
            child: AnyView::new(child),
            filter,
        }
    }

    pub fn style(&self) -> Filter {
        self.filter
    }
}

impl View for Filtered {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn foreground_filter(&self) -> Option<Filter> {
        Some(self.filter)
    }

    fn paint_self(&self, _ctx: &mut PaintCtx) {}
}

#[derive(Clone)]
pub struct BackdropFiltered {
    child: AnyView,
    filter: Backdrop,
}

impl BackdropFiltered {
    pub fn new<V: View + 'static>(child: V, filter: Backdrop) -> Self {
        Self {
            child: AnyView::new(child),
            filter,
        }
    }

    pub fn style(&self) -> Backdrop {
        self.filter
    }
}

impl View for BackdropFiltered {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        if self.filter.is_empty() {
            return;
        }
        ctx.push_backdrop(ctx.rect, self.filter);
    }
}

/// Wraps a single view and paints a [`Effect`] for it.
///
/// Effects are applied after the inner view is fully built. [`EffectNode`]
/// exposes no `.child()` method, so construct the child first:
///
/// ```ignore
/// let menu = Container::new().child(Row::new() /* ... */);
/// let menu = EffectNode::new(menu, Effect::Shadow(spec));
/// ```
#[derive(Clone)]
pub struct EffectNode {
    child: AnyView,
    effect: Effect,
}

impl EffectNode {
    pub fn new<V: View + 'static>(child: V, effect: Effect) -> Self {
        Self {
            child: AnyView::new(child),
            effect,
        }
    }

    pub fn effect(&self) -> &Effect {
        &self.effect
    }
}

impl View for EffectNode {
    fn body(&self) -> impl View + 'static {
        crate::views::Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn foreground_filter(&self) -> Option<Filter> {
        match &self.effect {
            Effect::Blur(f) => Some(*f),
            _ => None,
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        match &self.effect {
            Effect::Backdrop(b) if !b.is_empty() => {
                ctx.push_backdrop(ctx.rect, *b);
            }
            Effect::Shadow(spec) if spec.is_visible() => {
                ctx.list.push_shadow(sequoia_core::Shadow {
                    rect: ctx.rect,
                    color: spec.color,
                    radius: spec.spread.max(0.0),
                    blur: spec.blur,
                    spread: spec.spread,
                    offset: spec.offset,
                    inset: false,
                });
            }
            _ => {}
        }
    }
}

pub trait FilterExt: View + Sized + 'static {
    fn blur_own(self, radius: f32) -> Filtered {
        Filtered::new(self, Filter::blur(radius))
    }

    fn blur_own_with(self, build: impl FnOnce(Filter) -> Filter) -> Filtered {
        Filtered::new(self, build(Filter::NONE))
    }

    fn blur_behind(self, radius: f32) -> BackdropFiltered {
        BackdropFiltered::new(self, Backdrop::blur(radius))
    }

    fn blur_behind_with(self, build: impl FnOnce(Backdrop) -> Backdrop) -> BackdropFiltered {
        BackdropFiltered::new(self, build(Backdrop::NONE))
    }

    fn effect(self, effect: Effect) -> EffectNode {
        EffectNode::new(self, effect)
    }
}

impl<T: View + 'static> FilterExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_shortcuts_are_canonical() {
        let w = crate::views::Unit.blur_behind(16.0);
        assert_eq!(w.style().blur, 16.0);
        assert!(!w.style().is_empty());
        let w = crate::views::Unit.blur_own(4.0);
        assert!(!w.style().is_empty());
        let off = crate::views::Unit.blur_own(0.0);
        assert!(off.style().is_empty());
    }

    #[test]
    fn blur_with_builders() {
        use sequoia_style::gradient;
        let w = crate::views::Unit.blur_own_with(|b| b.blur_radius(4.0).mask(gradient(1.0, 0.0)));
        assert!(w.style().mask.is_some());
        let w =
            crate::views::Unit.blur_behind_with(|b| b.blur_radius(8.0).mask(gradient(1.0, 0.0)));
        assert!(w.style().mask.is_some());
    }

    #[test]
    fn effect_node_routes_backdrop_and_shadow() {
        let b = Effect::Backdrop(Backdrop::blur(8.0));
        assert!(!b.is_empty());
        let s = Effect::Shadow(sequoia_style::ShadowSpec::new(
            sequoia_core::Color::BLACK,
            8.0,
        ));
        assert!(!s.is_empty());
        let _ = crate::views::Unit.effect(b);
        let _ = crate::views::Unit.effect(s);
    }
}
