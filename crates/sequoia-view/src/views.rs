use sequoia_core::{Color, Constraints, Edges, Length, Rect, Size};
use sequoia_layout::{
    Align, Axis, ContainerStyle, GridFlow, GridPlacement, GridStyle, Indents, Inset, Justify,
    LayoutCtx, LayoutStyle, MasonryStyle, Overflow, Position, Track, WrapStyle,
};
use sequoia_style::BoxStyle;
use sequoia_text::{ElideMode, FontStyle, FontWeight, LetterSpacing, WrapMode};

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime};
use crate::view::{PaintCtx, View};

macro_rules! impl_style_extras {
    ($name:ident) => {
        impl $name {
            pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
                self.style = self.style.margin(margin);
                self
            }

            pub fn margin_xy(mut self, x: f32, y: f32) -> Self {
                self.style = self.style.margin_xy(x, y);
                self
            }

            pub fn margin_x(mut self, x: f32) -> Self {
                self.style = self.style.margin_x(x);
                self
            }

            pub fn margin_y(mut self, y: f32) -> Self {
                self.style = self.style.margin_y(y);
                self
            }

            pub fn margin_left(mut self, v: f32) -> Self {
                self.style = self.style.margin_left(v);
                self
            }

            pub fn margin_right(mut self, v: f32) -> Self {
                self.style = self.style.margin_right(v);
                self
            }

            pub fn margin_top(mut self, v: f32) -> Self {
                self.style = self.style.margin_top(v);
                self
            }

            pub fn margin_bottom(mut self, v: f32) -> Self {
                self.style = self.style.margin_bottom(v);
                self
            }

            pub fn indent_top(mut self, v: f32) -> Self {
                self.style = self.style.indent_top(v);
                self
            }

            pub fn indent_bottom(mut self, v: f32) -> Self {
                self.style = self.style.indent_bottom(v);
                self
            }

            pub fn indent_left(mut self, v: f32) -> Self {
                self.style = self.style.indent_left(v);
                self
            }

            pub fn indent_right(mut self, v: f32) -> Self {
                self.style = self.style.indent_right(v);
                self
            }

            pub fn clear_indents(mut self) -> Self {
                self.style = self.style.clear_indents();
                self
            }

            pub fn indent(mut self, indents: Indents) -> Self {
                self.style = self.style.indent(indents);
                self
            }

            pub fn indent_all(mut self, v: f32) -> Self {
                self.style = self.style.indent_all(v);
                self
            }

            pub fn indent_xy(mut self, x: f32, y: f32) -> Self {
                self.style = self.style.indent_xy(x, y);
                self
            }

            pub fn indent_axes(mut self, horizontal: f32, vertical: f32) -> Self {
                self.style = self.style.indent_axes(horizontal, vertical);
                self
            }

            pub fn flex_grow(mut self, weight: f32) -> Self {
                self.style = self.style.flex_grow(weight);
                self
            }

            pub fn flex_shrink(mut self, shrink: f32) -> Self {
                self.style = self.style.flex_shrink(shrink);
                self
            }

            pub fn flex_basis(mut self, basis: Option<Length>) -> Self {
                self.style = self.style.flex_basis(basis);
                self
            }

            pub fn align_self(mut self, align: Option<Align>) -> Self {
                self.style = self.style.align_self(align);
                self
            }

            pub fn align_self_value(mut self, align: Align) -> Self {
                self.style = self.style.align_self_value(align);
                self
            }

            pub fn aspect_ratio(mut self, ratio: Option<f32>) -> Self {
                self.style = self.style.aspect_ratio(ratio);
                self
            }

            pub fn aspect(mut self, ratio: f32) -> Self {
                self.style = self.style.aspect(ratio);
                self
            }

            pub fn position(mut self, position: Position) -> Self {
                self.style = self.style.position(position);
                self
            }

            pub fn absolute(mut self) -> Self {
                self.style = self.style.absolute();
                self
            }

            pub fn absolute_at(mut self, left: f32, top: f32) -> Self {
                self.style = self.style.absolute_at(left, top);
                self
            }

            pub fn absolute_inset(mut self, inset: Inset) -> Self {
                self.style = self.style.absolute_inset(inset);
                self
            }

            pub fn fit(mut self) -> Self {
                self.style = self.style.fit();
                self
            }

            pub fn hit_testable(mut self, on: bool) -> Self {
                self.style = self.style.hit_testable(on);
                self
            }
        }
    };
}

pub trait IntoChildren {
    fn into_children(self) -> Vec<AnyView>;
}

impl<V: View + 'static, const N: usize> IntoChildren for [V; N] {
    fn into_children(self) -> Vec<AnyView> {
        self.into_iter().map(AnyView::new).collect()
    }
}

impl<V: View + 'static> IntoChildren for Vec<V> {
    fn into_children(self) -> Vec<AnyView> {
        self.into_iter().map(AnyView::new).collect()
    }
}

impl<V: View + 'static> IntoChildren for Option<V> {
    fn into_children(self) -> Vec<AnyView> {
        self.into_iter().map(AnyView::new).collect()
    }
}

macro_rules! impl_into_children_tuple {
    ($($T:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($T: View + 'static),*> IntoChildren for ($($T,)*) {
            fn into_children(self) -> Vec<AnyView> {
                let ($($T,)*) = self;
                vec![$(AnyView::new($T)),*]
            }
        }
    };
}

impl_into_children_tuple!(A);
impl_into_children_tuple!(A, B);
impl_into_children_tuple!(A, B, C);
impl_into_children_tuple!(A, B, C, D);
impl_into_children_tuple!(A, B, C, D, E);
impl_into_children_tuple!(A, B, C, D, E, F);
impl_into_children_tuple!(A, B, C, D, E, F, G);
impl_into_children_tuple!(A, B, C, D, E, F, G, H);
impl_into_children_tuple!(A, B, C, D, E, F, G, H, I);
impl_into_children_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_into_children_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_into_children_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

macro_rules! impl_child_helpers {
    ($name:ident) => {
        impl $name {
            pub fn child_if<V: View + 'static>(self, cond: bool, make: impl FnOnce() -> V) -> Self {
                if cond { self.child(make()) } else { self }
            }

            pub fn child_opt<V: View + 'static>(self, child: Option<V>) -> Self {
                match child {
                    Some(v) => self.child(v),
                    None => self,
                }
            }
        }
    };
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Unit;

impl View for Unit {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }
}

pub struct Text {
    content: Box<dyn Fn() -> String>,
    font_size: Option<f32>,
    color: Option<Color>,
    weight: Option<FontWeight>,
    family: Option<String>,
    font_style: FontStyle,
    letter_spacing: f32,
    wrap_width: Option<f32>,
    wrap_mode: WrapMode,
    truncate_width: Option<f32>,

    truncate_fill: bool,
    elide_mode: ElideMode,
    underline: bool,
    strikethrough: bool,
    style: LayoutStyle,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        let s = content.into();
        Self {
            content: Box::new(move || s.clone()),
            font_size: None,
            color: None,
            weight: None,
            family: None,
            font_style: FontStyle::Normal,
            letter_spacing: 0.0,
            wrap_width: None,
            wrap_mode: WrapMode::default(),
            truncate_width: None,
            truncate_fill: false,
            elide_mode: ElideMode::default(),
            underline: false,
            strikethrough: false,
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
        }
    }

    pub fn with(content: impl Fn() -> String + 'static) -> Self {
        Self {
            content: Box::new(content),
            font_size: None,
            color: None,
            weight: None,
            family: None,
            font_style: FontStyle::Normal,
            letter_spacing: 0.0,
            wrap_width: None,
            wrap_mode: WrapMode::default(),
            truncate_width: None,
            truncate_fill: false,
            elide_mode: ElideMode::default(),
            underline: false,
            strikethrough: false,
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
        }
    }

    pub fn underline(mut self, on: bool) -> Self {
        self.underline = on;
        self
    }

    pub fn strikethrough(mut self, on: bool) -> Self {
        self.strikethrough = on;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn bold(mut self) -> Self {
        self.weight = Some(FontWeight::BOLD);
        self
    }

    pub fn bold_if(mut self, bold: bool) -> Self {
        self.weight = Some(if bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        });
        self
    }

    pub fn semibold(mut self) -> Self {
        self.weight = Some(FontWeight::SEMIBOLD);
        self
    }

    pub fn medium(mut self) -> Self {
        self.weight = Some(FontWeight::MEDIUM);
        self
    }

    pub fn normal(mut self) -> Self {
        self.weight = Some(FontWeight::NORMAL);
        self
    }

    pub fn light(mut self) -> Self {
        self.weight = Some(FontWeight::LIGHT);
        self
    }

    pub fn black(mut self) -> Self {
        self.weight = Some(FontWeight::BLACK);
        self
    }

    pub fn font(mut self, family: impl Into<String>) -> Self {
        let family = family.into();
        self.family = if family.is_empty() {
            None
        } else {
            Some(family)
        };
        self
    }

    pub fn family(self, family: impl Into<String>) -> Self {
        self.font(family)
    }

    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.font_style = style;
        self
    }

    pub fn italic(mut self) -> Self {
        self.font_style = FontStyle::Italic;
        self
    }

    pub fn oblique(mut self) -> Self {
        self.font_style = FontStyle::Oblique;
        self
    }

    pub fn upright(mut self) -> Self {
        self.font_style = FontStyle::Normal;
        self
    }

    pub fn italic_if(mut self, on: bool) -> Self {
        self.font_style = if on {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
        self
    }

    pub fn letter_spacing(mut self, spacing: LetterSpacing) -> Self {
        self.letter_spacing = if spacing.is_finite() { spacing } else { 0.0 };
        self
    }

    pub fn wrap_width(mut self, width: f32) -> Self {
        self.wrap_width = if width > 0.0 { Some(width) } else { None };
        self
    }

    pub fn wrap_mode(mut self, mode: WrapMode) -> Self {
        self.wrap_mode = mode;
        self
    }

    pub fn truncate(mut self, max_width: f32) -> Self {
        self.truncate_width = if max_width > 0.0 {
            Some(max_width)
        } else {
            None
        };
        self
    }

    pub fn truncate_fill(mut self) -> Self {
        self.truncate_fill = true;
        self
    }

    pub fn elide(mut self, mode: ElideMode) -> Self {
        self.elide_mode = mode;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.style = self.style.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.style = self.style.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.style = self.style.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.style = self.style.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.style = self.style.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.style = self.style.padding_bottom(v);
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        self.style = self.style.margin(margin);
        self
    }

    pub fn margin_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.margin_xy(x, y);
        self
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        self.style = self.style.margin_x(x);
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        self.style = self.style.margin_y(y);
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.style = self.style.margin_left(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.style = self.style.margin_right(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.style = self.style.margin_top(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.style = self.style.margin_bottom(v);
        self
    }

    pub fn indent_top(mut self, v: f32) -> Self {
        self.style = self.style.indent_top(v);
        self
    }

    pub fn indent_bottom(mut self, v: f32) -> Self {
        self.style = self.style.indent_bottom(v);
        self
    }

    pub fn indent_left(mut self, v: f32) -> Self {
        self.style = self.style.indent_left(v);
        self
    }

    pub fn indent_right(mut self, v: f32) -> Self {
        self.style = self.style.indent_right(v);
        self
    }

    pub fn clear_indents(mut self) -> Self {
        self.style = self.style.clear_indents();
        self
    }

    pub fn indent(mut self, indents: Indents) -> Self {
        self.style = self.style.indent(indents);
        self
    }

    pub fn indent_all(mut self, v: f32) -> Self {
        self.style = self.style.indent_all(v);
        self
    }

    pub fn indent_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.indent_xy(x, y);
        self
    }

    pub fn indent_axes(mut self, horizontal: f32, vertical: f32) -> Self {
        self.style = self.style.indent_axes(horizontal, vertical);
        self
    }

    pub fn hit_testable(mut self, on: bool) -> Self {
        self.style = self.style.hit_testable(on);
        self
    }

    pub fn content(&self) -> String {
        (self.content)()
    }

    fn paint_decoration(
        &self,
        ctx: &mut PaintCtx,
        shown: &str,
        x: f32,
        baseline: f32,
        size: f32,
        weight: FontWeight,
        color: Color,
    ) {
        if !self.underline && !self.strikethrough {
            return;
        }
        let width = ctx.text_width_styled_spaced(
            shown,
            size,
            weight,
            self.letter_spacing,
            self.family.as_deref(),
            self.font_style,
        );
        if width <= 0.0 {
            return;
        }
        let thickness = (size * 0.07).max(1.0);
        if self.underline {
            let y = (baseline + size * 0.12).round();
            ctx.quad(Rect::from_xywh(x, y, width, thickness), color, 0.0);
        }
        if self.strikethrough {
            let y = (baseline - size * 0.30).round();
            ctx.quad(Rect::from_xywh(x, y, width, thickness), color, 0.0);
        }
    }
}

impl View for Text {
    fn body(&self) -> impl View + 'static {
        let _ = (self.content)();
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }

    fn accessibility(&self) -> Option<crate::a11y::Accessible> {
        Some(crate::a11y::Accessible::new(
            crate::a11y::AccessibleRole::Text,
            (self.content)(),
        ))
    }

    fn clip_self(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        self.style
    }
    fn intrinsic_size(&self, ctx: &mut LayoutCtx) -> Option<Size> {
        let size = self.font_size.unwrap_or_else(|| ctx.default_font_size());
        let weight = self.weight.unwrap_or_default();
        let family = self.family.as_deref();
        let fstyle = self.font_style;
        let content = (self.content)();
        let pad = self.style.padding;
        let text_size = if let Some(w) = self.truncate_width {
            ctx.text_elided_size_styled_spaced(
                &content,
                size,
                weight,
                w,
                self.elide_mode,
                self.letter_spacing,
                family,
                fstyle,
            )
        } else {
            match self.wrap_width {
                Some(w) => ctx.text_wrapped_size_styled_spaced(
                    &content,
                    size,
                    weight,
                    w,
                    self.wrap_mode,
                    self.letter_spacing,
                    family,
                    fstyle,
                ),
                None => {
                    let w = ctx.text_width_styled_spaced(
                        &content,
                        size,
                        weight,
                        self.letter_spacing,
                        family,
                        fstyle,
                    );
                    Size::new(w, LayoutCtx::text_line_height(size))
                }
            }
        };
        Some(Size::new(
            text_size.width + pad.horizontal_sum(),
            text_size.height + pad.vertical_sum(),
        ))
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let theme = ctx.theme();
        let size = self.font_size.unwrap_or(theme.font_size);
        let weight = self.weight.unwrap_or_default();
        let color = self.color.unwrap_or(theme.text);
        let content = (self.content)();
        let pad = self.style.padding;

        let content_rect = Rect::from_xywh(
            ctx.rect.left() + pad.left,
            ctx.rect.top() + pad.top,
            (ctx.rect.size.width - pad.horizontal_sum()).max(0.0),
            (ctx.rect.size.height - pad.vertical_sum()).max(0.0),
        );
        let x = content_rect.left();

        let family = self.family.as_deref();
        let fstyle = self.font_style;
        let (shown, y, wrapped) = if let Some(w) = self.truncate_width.or(if self.truncate_fill {
            Some(content_rect.size.width.max(0.0))
        } else {
            None
        }) {
            let shown = ctx.elide_styled_spaced(
                &content,
                size,
                weight,
                w,
                self.elide_mode,
                self.letter_spacing,
                family,
                fstyle,
            );
            let y = content_rect.top() + content_rect.size.height * 0.5 + size * 0.35;
            (shown, y, None)
        } else {
            let y = match self.wrap_width {
                Some(_) => {
                    content_rect.top() + LayoutCtx::text_line_height(size) * 0.5 + size * 0.35
                }
                None => content_rect.top() + content_rect.size.height * 0.5 + size * 0.35,
            };

            #[cfg(debug_assertions)]
            {
                if self.wrap_width.is_none() && !content.is_empty() {
                    let full_w = ctx.text_width_styled_spaced(
                        &content,
                        size,
                        weight,
                        self.letter_spacing,
                        family,
                        fstyle,
                    );
                    if full_w > content_rect.size.width + 1.0 && content_rect.size.width > 0.0 {
                        use std::sync::Once;
                        static WARN_ONCE: Once = Once::new();
                        WARN_ONCE.call_once(|| {
                            log::warn!(
                                "sequoia-view: Text `{}` overflows box ({full_w:.1}px > {:.1}px) without truncate — add `.truncate_fill()` or `.truncate(w)`. \
                                 (This warning once per process; subsequent overflows are silent.)",
                                content.chars().take(24).collect::<String>(),
                                content_rect.size.width,
                            );
                        });
                    }
                }
            }
            (content.clone(), y, self.wrap_width)
        };
        match wrapped {
            Some(w) => ctx.draw_text_wrapped_styled_spaced(
                &shown,
                (x, y),
                size,
                weight,
                w,
                self.wrap_mode,
                self.letter_spacing,
                family,
                fstyle,
                color,
            ),
            None => ctx.draw_text_styled_spaced(
                &shown,
                (x, y),
                size,
                weight,
                self.letter_spacing,
                family,
                fstyle,
                color,
            ),
        }
        self.paint_decoration(ctx, &shown, x, y, size, weight, color);
    }
}

#[derive(Clone, Default)]
pub struct Fragment {
    children: Vec<AnyView>,
}

impl Fragment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.children.push(AnyView::new(child));
        self
    }

    pub fn children(mut self, children: impl IntoChildren) -> Self {
        self.children.extend(children.into_children());
        self
    }
}

impl View for Fragment {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(self.children.clone())
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default().hit_testable(false)
    }
    fn hit_testable(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct Margined {
    child: AnyView,
    margin: Edges,
    indents: Indents,
}

impl Margined {
    pub fn new<V: View + 'static>(child: V, margin: impl Into<Edges>) -> Self {
        Self {
            child: AnyView::new(child),
            margin: margin.into(),
            indents: Indents::NONE,
        }
    }

    pub fn with_indents<V: View + 'static>(child: V, indents: Indents) -> Self {
        Self {
            child: AnyView::new(child),
            margin: Edges::ZERO,
            indents,
        }
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        let m = margin.into();
        self.margin = Edges::only(
            sanitize_margin_edge(m.left),
            sanitize_margin_edge(m.top),
            sanitize_margin_edge(m.right),
            sanitize_margin_edge(m.bottom),
        );
        self
    }

    pub fn margin_xy(self, x: f32, y: f32) -> Self {
        self.margin(Edges::xy(x, y))
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        let x = sanitize_margin_edge(x);
        self.margin.left = x;
        self.margin.right = x;
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        let y = sanitize_margin_edge(y);
        self.margin.top = y;
        self.margin.bottom = y;
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.margin.left = sanitize_margin_edge(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.margin.right = sanitize_margin_edge(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.margin.top = sanitize_margin_edge(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.margin.bottom = sanitize_margin_edge(v);
        self
    }

    pub fn indent_top(mut self, v: f32) -> Self {
        self.indents.top = Some(sequoia_layout::sanitize_indent(v));
        self
    }

    pub fn indent_bottom(mut self, v: f32) -> Self {
        self.indents.bottom = Some(sequoia_layout::sanitize_indent(v));
        self
    }

    pub fn indent_left(mut self, v: f32) -> Self {
        self.indents.left = Some(sequoia_layout::sanitize_indent(v));
        self
    }

    pub fn indent_right(mut self, v: f32) -> Self {
        self.indents.right = Some(sequoia_layout::sanitize_indent(v));
        self
    }

    pub fn clear_indents(mut self) -> Self {
        self.indents = Indents::NONE;
        self
    }

    pub fn indent(mut self, indents: Indents) -> Self {
        if let Some(v) = indents.top {
            self.indents.top = Some(sequoia_layout::sanitize_indent(v));
        }
        if let Some(v) = indents.bottom {
            self.indents.bottom = Some(sequoia_layout::sanitize_indent(v));
        }
        if let Some(v) = indents.left {
            self.indents.left = Some(sequoia_layout::sanitize_indent(v));
        }
        if let Some(v) = indents.right {
            self.indents.right = Some(sequoia_layout::sanitize_indent(v));
        }
        self
    }

    pub fn indent_all(self, v: f32) -> Self {
        self.indent(Indents::all(v))
    }

    pub fn indent_xy(self, x: f32, y: f32) -> Self {
        self.indent(Indents::xy(x, y))
    }

    pub fn indent_axes(self, horizontal: f32, vertical: f32) -> Self {
        self.indent(Indents::axes(horizontal, vertical))
    }
}

fn sanitize_margin_edge(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl View for Margined {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            margin: self.margin,
            indent_top: self.indents.top,
            indent_bottom: self.indents.bottom,
            indent_left: self.indents.left,
            indent_right: self.indents.right,
            ..LayoutStyle::default().hit_testable(false)
        }
    }
    fn hit_testable(&self) -> bool {
        false
    }
}

#[derive(Clone, Default)]
pub struct FocusScope {
    children: Vec<AnyView>,
    trap: bool,
}

impl FocusScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.children.push(AnyView::new(child));
        self
    }

    pub fn children(mut self, children: impl IntoChildren) -> Self {
        self.children.extend(children.into_children());
        self
    }

    pub fn trap(mut self, trap: bool) -> Self {
        self.trap = trap;
        self
    }
}

impl View for FocusScope {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(self.children.clone())
    }
    fn focus_trap(&self) -> bool {
        self.trap
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }
}

struct Flex {
    children: Vec<AnyView>,

    flex: Vec<f32>,
    gap: f32,
    cross_gap: f32,
    wrap: bool,
    align_content: Align,
    style: LayoutStyle,
    axis: Axis,
    align: Align,
    justify: Justify,
    deco: BoxStyle,
}

impl Flex {
    fn new(axis: Axis) -> Self {
        Self {
            children: Vec::new(),
            flex: Vec::new(),
            gap: 0.0,
            cross_gap: 0.0,
            wrap: false,
            align_content: Align::Start,
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
            axis,
            align: Align::Start,
            justify: Justify::Start,
            deco: BoxStyle::new(),
        }
    }

    fn sanitize_flex(flex: f32) -> f32 {
        if flex.is_finite() && flex > 0.0 {
            flex
        } else {
            0.0
        }
    }
}

impl View for Flex {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(self.children.clone())
    }
    fn layout_style(&self) -> LayoutStyle {
        let mut style = self.style;
        let edges = self.deco.border_edges();
        style.padding.left += edges.left;
        style.padding.top += edges.top;
        style.padding.right += edges.right;
        style.padding.bottom += edges.bottom;
        style
    }
    fn container(&self) -> Option<ContainerStyle> {
        Some(ContainerStyle {
            axis: self.axis,
            gap: self.gap,
            cross_gap: self.cross_gap,
            align: self.align,
            justify: self.justify,
            wrap: self.wrap,
            align_content: self.align_content,
        })
    }
}

macro_rules! impl_flex_container {
    ($name:ident, $axis:expr) => {
        impl $name {
            pub fn gap(mut self, gap: f32) -> Self {
                let gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
                self.0.gap = gap;
                self.0.cross_gap = gap;
                self
            }

            pub fn main_gap(mut self, gap: f32) -> Self {
                self.0.gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
                self
            }

            pub fn cross_gap(mut self, gap: f32) -> Self {
                self.0.cross_gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
                self
            }

            pub fn gaps(mut self, main: f32, cross: f32) -> Self {
                self.0.gap = if main.is_finite() { main.max(0.0) } else { 0.0 };
                self.0.cross_gap = if cross.is_finite() {
                    cross.max(0.0)
                } else {
                    0.0
                };
                self
            }

            pub fn wrap(mut self, wrap: bool) -> Self {
                self.0.wrap = wrap;
                self
            }

            pub fn wrapped(mut self) -> Self {
                self.0.wrap = true;
                self
            }

            pub fn align_content(mut self, align: Align) -> Self {
                self.0.align_content = align;
                self
            }

            pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
                self.0.style = self.0.style.margin(margin);
                self
            }

            pub fn margin_xy(mut self, x: f32, y: f32) -> Self {
                self.0.style = self.0.style.margin_xy(x, y);
                self
            }

            pub fn margin_x(mut self, x: f32) -> Self {
                self.0.style = self.0.style.margin_x(x);
                self
            }

            pub fn margin_y(mut self, y: f32) -> Self {
                self.0.style = self.0.style.margin_y(y);
                self
            }

            pub fn margin_left(mut self, v: f32) -> Self {
                self.0.style = self.0.style.margin_left(v);
                self
            }

            pub fn margin_right(mut self, v: f32) -> Self {
                self.0.style = self.0.style.margin_right(v);
                self
            }

            pub fn margin_top(mut self, v: f32) -> Self {
                self.0.style = self.0.style.margin_top(v);
                self
            }

            pub fn margin_bottom(mut self, v: f32) -> Self {
                self.0.style = self.0.style.margin_bottom(v);
                self
            }

            pub fn indent_top(mut self, v: f32) -> Self {
                self.0.style = self.0.style.indent_top(v);
                self
            }

            pub fn indent_bottom(mut self, v: f32) -> Self {
                self.0.style = self.0.style.indent_bottom(v);
                self
            }

            pub fn indent_left(mut self, v: f32) -> Self {
                self.0.style = self.0.style.indent_left(v);
                self
            }

            pub fn indent_right(mut self, v: f32) -> Self {
                self.0.style = self.0.style.indent_right(v);
                self
            }

            pub fn clear_indents(mut self) -> Self {
                self.0.style = self.0.style.clear_indents();
                self
            }

            pub fn indent(mut self, indents: Indents) -> Self {
                self.0.style = self.0.style.indent(indents);
                self
            }

            pub fn indent_all(mut self, v: f32) -> Self {
                self.0.style = self.0.style.indent_all(v);
                self
            }

            pub fn indent_xy(mut self, x: f32, y: f32) -> Self {
                self.0.style = self.0.style.indent_xy(x, y);
                self
            }

            pub fn indent_axes(mut self, horizontal: f32, vertical: f32) -> Self {
                self.0.style = self.0.style.indent_axes(horizontal, vertical);
                self
            }

            pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
                self.0.style = self.0.style.padding(padding);
                self
            }

            pub fn pad(self, padding: impl Into<Edges>) -> Self {
                self.padding(padding)
            }

            pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
                self.0.style = self.0.style.padding_xy(x, y);
                self
            }

            pub fn padding_x(mut self, x: f32) -> Self {
                self.0.style = self.0.style.padding_x(x);
                self
            }

            pub fn padding_y(mut self, y: f32) -> Self {
                self.0.style = self.0.style.padding_y(y);
                self
            }

            pub fn padding_left(mut self, v: f32) -> Self {
                self.0.style = self.0.style.padding_left(v);
                self
            }

            pub fn padding_right(mut self, v: f32) -> Self {
                self.0.style = self.0.style.padding_right(v);
                self
            }

            pub fn padding_top(mut self, v: f32) -> Self {
                self.0.style = self.0.style.padding_top(v);
                self
            }

            pub fn padding_bottom(mut self, v: f32) -> Self {
                self.0.style = self.0.style.padding_bottom(v);
                self
            }

            pub fn width(mut self, width: impl Into<Length>) -> Self {
                self.0.style = self.0.style.width(width.into());
                self
            }

            pub fn height(mut self, height: impl Into<Length>) -> Self {
                self.0.style = self.0.style.height(height.into());
                self
            }

            pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
                self.0.style = self.0.style.width(width.into()).height(height.into());
                self
            }

            pub fn square(mut self, side: impl Into<Length>) -> Self {
                let side = side.into();
                self.0.style = self.0.style.width(side).height(side);
                self
            }

            pub fn min_width(mut self, v: f32) -> Self {
                self.0.style = self.0.style.min_width(v);
                self
            }

            pub fn max_width(mut self, v: f32) -> Self {
                self.0.style = self.0.style.max_width(v);
                self
            }

            pub fn min_height(mut self, v: f32) -> Self {
                self.0.style = self.0.style.min_height(v);
                self
            }

            pub fn max_height(mut self, v: f32) -> Self {
                self.0.style = self.0.style.max_height(v);
                self
            }

            pub fn overflow(mut self, overflow: Overflow) -> Self {
                self.0.style = self.0.style.overflow(overflow);
                self
            }

            pub fn flex(mut self, weight: f32) -> Self {
                self.0.style = self.0.style.flex(weight);
                self
            }

            pub fn flex_grow(mut self, weight: f32) -> Self {
                self.0.style = self.0.style.flex_grow(weight);
                self
            }

            pub fn flex_shrink(mut self, shrink: f32) -> Self {
                self.0.style = self.0.style.flex_shrink(shrink);
                self
            }

            pub fn flex_basis(mut self, basis: Option<Length>) -> Self {
                self.0.style = self.0.style.flex_basis(basis);
                self
            }

            pub fn align_self(mut self, align: Option<Align>) -> Self {
                self.0.style = self.0.style.align_self(align);
                self
            }

            pub fn align_self_value(mut self, align: Align) -> Self {
                self.0.style = self.0.style.align_self_value(align);
                self
            }

            pub fn aspect_ratio(mut self, ratio: Option<f32>) -> Self {
                self.0.style = self.0.style.aspect_ratio(ratio);
                self
            }

            pub fn aspect(mut self, ratio: f32) -> Self {
                self.0.style = self.0.style.aspect(ratio);
                self
            }

            pub fn position(mut self, position: Position) -> Self {
                self.0.style = self.0.style.position(position);
                self
            }

            pub fn absolute(mut self) -> Self {
                self.0.style = self.0.style.absolute();
                self
            }

            pub fn absolute_at(mut self, left: f32, top: f32) -> Self {
                self.0.style = self.0.style.absolute_at(left, top);
                self
            }

            pub fn absolute_inset(mut self, inset: Inset) -> Self {
                self.0.style = self.0.style.absolute_inset(inset);
                self
            }

            pub fn expand(self) -> Self {
                self.flex(1.0)
            }

            pub fn fill_width(mut self) -> Self {
                self.0.style = self.0.style.width(Length::Fill(1.0));
                self
            }

            pub fn fill_height(mut self) -> Self {
                self.0.style = self.0.style.height(Length::Fill(1.0));
                self
            }

            pub fn fill_size(self) -> Self {
                self.fill_width().fill_height()
            }

            pub fn fit(mut self) -> Self {
                self.0.style = self.0.style.fit();
                self
            }

            pub fn hit_testable(mut self, on: bool) -> Self {
                self.0.style = self.0.style.hit_testable(on);
                self
            }

            pub fn align_items(mut self, align: Align) -> Self {
                self.0.align = align;
                self
            }

            pub fn justify_content(mut self, justify: Justify) -> Self {
                self.0.justify = justify;
                self
            }

            pub fn center(mut self) -> Self {
                self.0.align = Align::Center;
                self.0.justify = Justify::Center;
                self
            }

            pub fn background(mut self, color: Color) -> Self {
                self.0.deco = self.0.deco.background(color);
                self
            }

            pub fn border(mut self, width: f32, color: Color) -> Self {
                self.0.deco = self.0.deco.border(width, color);
                self
            }

            pub fn border_left(mut self, width: f32, color: Color) -> Self {
                self.0.deco = self.0.deco.border_left(width, color);
                self
            }

            pub fn border_right(mut self, width: f32, color: Color) -> Self {
                self.0.deco = self.0.deco.border_right(width, color);
                self
            }

            pub fn border_top(mut self, width: f32, color: Color) -> Self {
                self.0.deco = self.0.deco.border_top(width, color);
                self
            }

            pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
                self.0.deco = self.0.deco.border_bottom(width, color);
                self
            }

            pub fn border_x(mut self, width: f32, color: Color) -> Self {
                self.0.deco = self.0.deco.border_x(width, color);
                self
            }

            pub fn border_y(mut self, width: f32, color: Color) -> Self {
                self.0.deco = self.0.deco.border_y(width, color);
                self
            }

            pub fn radius(mut self, radius: f32) -> Self {
                self.0.deco = self.0.deco.radius(radius);
                self
            }

            pub fn box_style(mut self, style: BoxStyle) -> Self {
                self.0.deco = style;
                self
            }

            pub fn style(self, style: BoxStyle) -> Self {
                self.box_style(style)
            }
        }
    };
}

pub struct Row(Flex);

impl Row {
    pub fn new() -> Self {
        Self(Flex::new(Axis::Horizontal))
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.0.children.push(AnyView::new(child));
        self.0.flex.push(0.0);
        self
    }

    pub fn child_flex<V: View + 'static>(mut self, child: V, flex: f32) -> Self {
        self.0.children.push(AnyView::new(child));
        self.0.flex.push(Flex::sanitize_flex(flex));
        self
    }

    pub fn children(mut self, children: impl IntoChildren) -> Self {
        let mut children = children.into_children();
        self.0.flex.extend(std::iter::repeat_n(0.0, children.len()));
        self.0.children.append(&mut children);
        self
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Row {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        self.0.children_override()
    }
    fn layout_style(&self) -> LayoutStyle {
        self.0.layout_style()
    }
    fn container(&self) -> Option<ContainerStyle> {
        self.0.container()
    }
    fn flex_weights(&self) -> Option<&[f32]> {
        Some(&self.0.flex)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(self.0.deco);
    }
}

impl_flex_container!(Row, Axis::Horizontal);

pub struct Column(Flex);

impl Column {
    pub fn new() -> Self {
        Self(Flex::new(Axis::Vertical))
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.0.children.push(AnyView::new(child));
        self.0.flex.push(0.0);
        self
    }

    pub fn child_flex<V: View + 'static>(mut self, child: V, flex: f32) -> Self {
        self.0.children.push(AnyView::new(child));
        self.0.flex.push(Flex::sanitize_flex(flex));
        self
    }

    pub fn children(mut self, children: impl IntoChildren) -> Self {
        let mut children = children.into_children();
        self.0.flex.extend(std::iter::repeat_n(0.0, children.len()));
        self.0.children.append(&mut children);
        self
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl View for Column {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        self.0.children_override()
    }
    fn layout_style(&self) -> LayoutStyle {
        self.0.layout_style()
    }
    fn container(&self) -> Option<ContainerStyle> {
        self.0.container()
    }
    fn flex_weights(&self) -> Option<&[f32]> {
        Some(&self.0.flex)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(self.0.deco);
    }
}

impl_flex_container!(Column, Axis::Vertical);

#[derive(Clone)]
pub struct Stack {
    children: Vec<AnyView>,
    positions: Vec<StackPos>,
    style: LayoutStyle,
    deco: BoxStyle,
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            positions: Vec::new(),
            style: LayoutStyle::default(),
            deco: BoxStyle::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StackPos {
    pub top: Option<f32>,

    pub left: Option<f32>,

    pub bottom: Option<f32>,

    pub right: Option<f32>,

    pub dx: f32,

    pub dy: f32,

    pub z: i32,
}

impl StackPos {
    pub const fn top() -> Self {
        Self {
            top: Some(0.0),
            left: None,
            bottom: None,
            right: None,
            dx: 0.0,
            dy: 0.0,
            z: 0,
        }
    }

    pub const fn bottom() -> Self {
        Self {
            top: None,
            left: None,
            bottom: Some(0.0),
            right: None,
            dx: 0.0,
            dy: 0.0,
            z: 0,
        }
    }

    pub const fn left() -> Self {
        Self {
            top: None,
            left: Some(0.0),
            bottom: None,
            right: None,
            dx: 0.0,
            dy: 0.0,
            z: 0,
        }
    }

    pub const fn right() -> Self {
        Self {
            top: None,
            left: None,
            bottom: None,
            right: Some(0.0),
            dx: 0.0,
            dy: 0.0,
            z: 0,
        }
    }

    pub const fn top_left() -> Self {
        Self {
            top: Some(0.0),
            left: Some(0.0),
            bottom: None,
            right: None,
            dx: 0.0,
            dy: 0.0,
            z: 0,
        }
    }

    pub const fn top_right() -> Self {
        Self {
            top: Some(0.0),
            left: None,
            bottom: None,
            right: Some(0.0),
            dx: 0.0,
            dy: 0.0,
            z: 0,
        }
    }

    pub const fn bottom_left() -> Self {
        Self {
            top: None,
            left: Some(0.0),
            bottom: Some(0.0),
            right: None,
            dx: 0.0,
            dy: 0.0,
            z: 0,
        }
    }

    pub const fn bottom_right() -> Self {
        Self {
            top: None,
            left: None,
            bottom: Some(0.0),
            right: Some(0.0),
            dx: 0.0,
            dy: 0.0,
            z: 0,
        }
    }

    pub const fn fill() -> Self {
        Self {
            top: Some(0.0),
            left: Some(0.0),
            bottom: Some(0.0),
            right: Some(0.0),
            dx: 0.0,
            dy: 0.0,
            z: 0,
        }
    }

    pub const fn offset(mut self, dx: f32, dy: f32) -> Self {
        self.dx = dx;
        self.dy = dy;
        self
    }

    pub const fn z(mut self, z: i32) -> Self {
        self.z = z;
        self
    }

    pub const fn is_pinned(self) -> bool {
        self.top.is_some() || self.left.is_some() || self.bottom.is_some() || self.right.is_some()
    }
}

impl Stack {
    pub fn new() -> Self {
        Self::default()
    }

    fn stack_axis(len: Length, measured: f32, start: f32, extent: f32) -> (f32, f32) {
        if len.is_fill() && extent.is_finite() {
            return (extent.max(0.0), start);
        }

        let size = measured.max(0.0);
        let off = if extent.is_finite() {
            ((extent - size) * 0.5).max(0.0)
        } else {
            0.0
        };
        (size, start + off)
    }

    fn pinned_axis(
        pin_start: Option<f32>,
        pin_end: Option<f32>,
        delta: f32,
        len: Length,
        measured: f32,
        start: f32,
        extent: f32,
    ) -> (f32, f32) {
        let delta = if delta.is_finite() { delta } else { 0.0 };
        if !extent.is_finite() {
            return (measured.max(0.0), start + delta);
        }
        let extent = extent.max(0.0);
        match (pin_start, pin_end) {
            (Some(a), Some(b)) => {
                let x = a.max(0.0);
                let w = (extent - a.max(0.0) - b.max(0.0)).max(0.0);
                (w, start + x + delta)
            }
            (Some(a), None) => {
                let size = if len.is_fill() {
                    (extent - a.max(0.0)).max(0.0)
                } else {
                    measured.max(0.0)
                };
                (size, start + a + delta)
            }
            (None, Some(b)) => {
                let size = if len.is_fill() {
                    (extent - b.max(0.0)).max(0.0)
                } else {
                    measured.max(0.0)
                };
                (size, start + extent - b - size + delta)
            }
            (None, None) => {
                let (size, off) = Self::stack_axis(len, measured, start, extent);
                (size, off + delta)
            }
        }
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.children.push(AnyView::new(child));
        self.positions.push(StackPos::default());
        self
    }

    pub fn child_at<V: View + 'static>(mut self, child: V, pos: StackPos) -> Self {
        self.children.push(AnyView::new(child));
        self.positions.push(pos);
        self
    }

    pub fn child_z<V: View + 'static>(mut self, child: V, z: i32) -> Self {
        self.children.push(AnyView::new(child));
        self.positions.push(StackPos::default().z(z));
        self
    }

    pub fn fill<V: View + 'static>(mut self, child: V) -> Self {
        self.children.push(AnyView::new(child));
        self.positions.push(StackPos::fill());
        self
    }

    pub fn overlay<V: View + 'static>(mut self, child: V, dx: f32, dy: f32) -> Self {
        self.children.push(AnyView::new(child));
        self.positions.push(StackPos::top_left().offset(dx, dy));
        self
    }

    pub fn hud<V: View + 'static>(mut self, child: V, dy: f32) -> Self {
        self.children.push(AnyView::new(child));
        self.positions.push(StackPos::bottom().offset(0.0, dy));
        self
    }

    fn paint_order(&self) -> Option<Vec<usize>> {
        if self.positions.windows(2).all(|w| w[0].z <= w[1].z) {
            return None;
        }
        let mut order: Vec<usize> = (0..self.children.len()).collect();
        order.sort_by_key(|&i| self.positions.get(i).map_or(0, |p| p.z));
        Some(order)
    }

    fn pos_at(&self, i: usize, order: Option<&[usize]>) -> StackPos {
        match order {
            None => self.positions.get(i).copied().unwrap_or_default(),
            Some(order) => order
                .get(i)
                .and_then(|&k| self.positions.get(k))
                .copied()
                .unwrap_or_default(),
        }
    }

    pub fn children(mut self, children: impl IntoChildren) -> Self {
        for c in children.into_children() {
            self.children.push(c);
            self.positions.push(StackPos::default());
        }
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.style = self.style.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.style = self.style.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.style = self.style.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.style = self.style.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.style = self.style.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.style = self.style.padding_bottom(v);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into()).height(height.into());
        self
    }

    pub fn square(mut self, side: impl Into<Length>) -> Self {
        let side = side.into();
        self.style = self.style.width(side).height(side);
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.style = self.style.overflow(overflow);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_left(width, color);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_right(width, color);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_top(width, color);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_bottom(width, color);
        self
    }

    pub fn border_x(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_x(width, color);
        self
    }

    pub fn border_y(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_y(width, color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.deco = self.deco.radius(radius);
        self
    }

    pub fn box_style(mut self, style: BoxStyle) -> Self {
        self.deco = style;
        self
    }

    pub fn style(self, style: BoxStyle) -> Self {
        self.box_style(style)
    }
}

impl_style_extras!(Stack);

impl View for Stack {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(match self.paint_order() {
            None => self.children.clone(),
            Some(order) => order
                .iter()
                .filter_map(|&i| self.children.get(i).cloned())
                .collect(),
        })
    }
    fn layout_style(&self) -> LayoutStyle {
        let mut style = self.style;
        let edges = self.deco.border_edges();
        style.padding.left += edges.left;
        style.padding.top += edges.top;
        style.padding.right += edges.right;
        style.padding.bottom += edges.bottom;
        style
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(self.deco);
    }
    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        let mut pad = self.style.padding;
        let edges = self.deco.border_edges();
        pad.left += edges.left;
        pad.top += edges.top;
        pad.right += edges.right;
        pad.bottom += edges.bottom;
        let content = Rect::from_xywh(
            rect.left() + pad.left,
            rect.top() + pad.top,
            (rect.size.width - pad.horizontal_sum()).max(0.0),
            (rect.size.height - pad.vertical_sum()).max(0.0),
        );
        let max = Size::new(content.size.width, content.size.height);
        let order = self.paint_order();
        let kids = rt.children(el);
        for (i, child) in kids.iter().enumerate() {
            let style = rt.child_layout_style(*child);

            if style.is_absolute() {
                rt.arrange_absolute(*child, content, ctx);
                continue;
            }
            let pos = self.pos_at(i, order.as_deref());

            let m = rt.measure_node(
                *child,
                Constraints {
                    min: Size::ZERO,
                    max,
                },
                ctx,
            );
            let (w0, x) = Self::pinned_axis(
                pos.left,
                pos.right,
                pos.dx,
                style.width,
                m.width,
                content.left(),
                content.size.width,
            );
            let (h0, y) = Self::pinned_axis(
                pos.top,
                pos.bottom,
                pos.dy,
                style.height,
                m.height,
                content.top(),
                content.size.height,
            );

            let w = style.clamp_width(w0).max(0.0);
            let h = style.clamp_height(h0).max(0.0);
            rt.arrange_node(*child, Rect::from_xywh(x, y, w, h), ctx);
        }
        true
    }
}

pub struct Grid {
    children: Vec<AnyView>,
    placements: Vec<GridPlacement>,
    style: LayoutStyle,
    grid: GridStyle,
    deco: BoxStyle,
}

impl Grid {
    pub fn new(columns: impl IntoIterator<Item = Track>) -> Self {
        Self {
            children: Vec::new(),
            placements: Vec::new(),
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
            grid: GridStyle::new(columns),
            deco: BoxStyle::new(),
        }
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.children.push(AnyView::new(child));
        self.placements.push(GridPlacement::AUTO);
        self
    }

    pub fn child_at<V: View + 'static>(mut self, child: V, placement: GridPlacement) -> Self {
        self.children.push(AnyView::new(child));
        self.placements.push(placement);
        self
    }

    pub fn child_span<V: View + 'static>(
        mut self,
        child: V,
        column: usize,
        row: usize,
        col_span: usize,
        row_span: usize,
    ) -> Self {
        self.children.push(AnyView::new(child));
        self.placements.push(
            GridPlacement::new()
                .at(column, row)
                .span(col_span, row_span),
        );
        self
    }

    pub fn children(mut self, children: impl IntoChildren) -> Self {
        let mut children = children.into_children();
        self.placements
            .extend(std::iter::repeat_n(GridPlacement::AUTO, children.len()));
        self.children.append(&mut children);
        self
    }

    pub fn rows(mut self, rows: impl IntoIterator<Item = Track>) -> Self {
        self.grid = self.grid.rows(rows);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.grid = self.grid.gap(gap);
        self
    }

    pub fn column_gap(mut self, gap: f32) -> Self {
        self.grid = self.grid.column_gap(gap);
        self
    }

    pub fn row_gap(mut self, gap: f32) -> Self {
        self.grid = self.grid.row_gap(gap);
        self
    }

    pub fn auto_flow(mut self, flow: GridFlow) -> Self {
        self.grid = self.grid.auto_flow(flow);
        self
    }

    pub fn column_flow(mut self) -> Self {
        self.grid = self.grid.column_flow();
        self
    }

    pub fn align_items(mut self, align: Align) -> Self {
        self.grid = self.grid.align_items(align);
        self
    }

    pub fn justify_items(mut self, align: Align) -> Self {
        self.grid = self.grid.justify_items(align);
        self
    }

    pub fn place_items(mut self, align: Align) -> Self {
        self.grid = self.grid.place_items(align);
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        self.style = self.style.margin(margin);
        self
    }

    pub fn margin_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.margin_xy(x, y);
        self
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        self.style = self.style.margin_x(x);
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        self.style = self.style.margin_y(y);
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.style = self.style.margin_left(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.style = self.style.margin_right(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.style = self.style.margin_top(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.style = self.style.margin_bottom(v);
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.style = self.style.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.style = self.style.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.style = self.style.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.style = self.style.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.style = self.style.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.style = self.style.padding_bottom(v);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into()).height(height.into());
        self
    }

    pub fn square(mut self, side: impl Into<Length>) -> Self {
        let side = side.into();
        self.style = self.style.width(side).height(side);
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.style = self.style.overflow(overflow);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.style = self.style.flex_shrink(shrink);
        self
    }

    pub fn flex_basis(mut self, basis: Option<Length>) -> Self {
        self.style = self.style.flex_basis(basis);
        self
    }

    pub fn align_self(mut self, align: Option<Align>) -> Self {
        self.style = self.style.align_self(align);
        self
    }

    pub fn aspect_ratio(mut self, ratio: Option<f32>) -> Self {
        self.style = self.style.aspect_ratio(ratio);
        self
    }

    pub fn aspect(mut self, ratio: f32) -> Self {
        self.style = self.style.aspect(ratio);
        self
    }

    pub fn position(mut self, position: Position) -> Self {
        self.style = self.style.position(position);
        self
    }

    pub fn absolute(mut self) -> Self {
        self.style = self.style.absolute();
        self
    }

    pub fn absolute_at(mut self, left: f32, top: f32) -> Self {
        self.style = self.style.absolute_at(left, top);
        self
    }

    pub fn absolute_inset(mut self, inset: Inset) -> Self {
        self.style = self.style.absolute_inset(inset);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_left(width, color);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_right(width, color);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_top(width, color);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_bottom(width, color);
        self
    }

    pub fn border_x(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_x(width, color);
        self
    }

    pub fn border_y(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_y(width, color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.deco = self.deco.radius(radius);
        self
    }

    pub fn box_style(mut self, style: BoxStyle) -> Self {
        self.deco = style;
        self
    }

    pub fn style(self, style: BoxStyle) -> Self {
        self.box_style(style)
    }
}

impl View for Grid {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(self.children.clone())
    }
    fn layout_style(&self) -> LayoutStyle {
        self.style
    }
    fn grid(&self) -> Option<GridStyle> {
        Some(self.grid.clone())
    }
    fn grid_placements(&self) -> Option<Vec<GridPlacement>> {
        Some(self.placements.clone())
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(self.deco);
    }
}

pub struct Wrap {
    children: Vec<AnyView>,
    style: LayoutStyle,
    wrap: WrapStyle,
    deco: BoxStyle,
}

impl Wrap {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fit,
                ..LayoutStyle::default()
            },
            wrap: WrapStyle::new(Axis::Horizontal),
            deco: BoxStyle::new(),
        }
    }

    pub fn vertical() -> Self {
        Self {
            wrap: WrapStyle::new(Axis::Vertical),
            ..Self::new()
        }
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.wrap = self.wrap.axis(axis);
        self
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.children.push(AnyView::new(child));
        self
    }

    pub fn children(mut self, children: impl IntoChildren) -> Self {
        self.children.extend(children.into_children());
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.wrap = self.wrap.gap(gap);
        self
    }

    pub fn column_gap(mut self, gap: f32) -> Self {
        self.wrap = self.wrap.column_gap(gap);
        self
    }

    pub fn row_gap(mut self, gap: f32) -> Self {
        self.wrap = self.wrap.row_gap(gap);
        self
    }

    pub fn align_items(mut self, align: Align) -> Self {
        self.wrap = self.wrap.align(align);
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.style = self.style.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.style = self.style.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.style = self.style.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.style = self.style.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.style = self.style.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.style = self.style.padding_bottom(v);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into()).height(height.into());
        self
    }

    pub fn square(mut self, side: impl Into<Length>) -> Self {
        let side = side.into();
        self.style = self.style.width(side).height(side);
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.style = self.style.overflow(overflow);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_left(width, color);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_right(width, color);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_top(width, color);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_bottom(width, color);
        self
    }

    pub fn border_x(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_x(width, color);
        self
    }

    pub fn border_y(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_y(width, color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.deco = self.deco.radius(radius);
        self
    }

    pub fn box_style(mut self, style: BoxStyle) -> Self {
        self.deco = style;
        self
    }

    pub fn style(self, style: BoxStyle) -> Self {
        self.box_style(style)
    }
}

impl Default for Wrap {
    fn default() -> Self {
        Self::new()
    }
}

impl_style_extras!(Wrap);

impl View for Wrap {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(self.children.clone())
    }
    fn layout_style(&self) -> LayoutStyle {
        self.style
    }
    fn wrap(&self) -> Option<WrapStyle> {
        Some(self.wrap)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(self.deco);
    }
}

/// A waterfall layout with equal-width columns and variable item heights.
///
/// Items are appended to the shortest column, keeping their natural height.
/// This is useful for card feeds and galleries where rows should not share a
/// common height. Absolute children are excluded from the flow.
///
/// ```
/// use sequoia_view::prelude::*;
///
/// let feed = Masonry::new(2)
///     .gap(12.0)
///     .padding(16.0)
///     .child(Panel::new().height(120.0))
///     .child(Panel::new().height(80.0))
///     .child(Panel::new().height(160.0));
/// # let _ = feed;
/// ```
pub struct Masonry {
    children: Vec<AnyView>,
    style: LayoutStyle,
    masonry: MasonryStyle,
    deco: BoxStyle,
}

impl Masonry {
    /// Creates a masonry container with the given number of columns.
    pub fn new(columns: usize) -> Self {
        Self {
            children: Vec::new(),
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fit,
                ..LayoutStyle::default()
            },
            masonry: MasonryStyle::new(columns),
            deco: BoxStyle::new(),
        }
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.masonry = self.masonry.columns(columns);
        self
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        self.children.push(AnyView::new(child));
        self
    }

    pub fn children(mut self, children: impl IntoChildren) -> Self {
        self.children.extend(children.into_children());
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.masonry = self.masonry.gap(gap);
        self
    }

    pub fn column_gap(mut self, gap: f32) -> Self {
        self.masonry = self.masonry.column_gap(gap);
        self
    }

    pub fn row_gap(mut self, gap: f32) -> Self {
        self.masonry = self.masonry.row_gap(gap);
        self
    }

    pub fn align_items(mut self, align: Align) -> Self {
        self.masonry = self.masonry.align_items(align);
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.style = self.style.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.style = self.style.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.style = self.style.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.style = self.style.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.style = self.style.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.style = self.style.padding_bottom(v);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into()).height(height.into());
        self
    }

    pub fn square(mut self, side: impl Into<Length>) -> Self {
        let side = side.into();
        self.style = self.style.width(side).height(side);
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.style = self.style.overflow(overflow);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_left(width, color);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_right(width, color);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_top(width, color);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_bottom(width, color);
        self
    }

    pub fn border_x(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_x(width, color);
        self
    }

    pub fn border_y(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_y(width, color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.deco = self.deco.radius(radius);
        self
    }

    pub fn box_style(mut self, style: BoxStyle) -> Self {
        self.deco = style;
        self
    }

    pub fn style(self, style: BoxStyle) -> Self {
        self.box_style(style)
    }
}

impl Default for Masonry {
    fn default() -> Self {
        Self::new(2)
    }
}

impl_style_extras!(Masonry);

impl View for Masonry {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(self.children.clone())
    }
    fn layout_style(&self) -> LayoutStyle {
        self.style
    }
    fn masonry(&self) -> Option<MasonryStyle> {
        Some(self.masonry)
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(self.deco);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Panel {
    color: Option<Color>,
    radius: f32,
    style: LayoutStyle,
    borders: BoxStyle,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            color: None,
            radius: 0.0,
            style: LayoutStyle::default(),
            borders: BoxStyle::new(),
        }
    }
}

impl Panel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn background(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.borders = self.borders.border(width, color);
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.borders = self.borders.border_left(width, color);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.borders = self.borders.border_right(width, color);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.borders = self.borders.border_top(width, color);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.borders = self.borders.border_bottom(width, color);
        self
    }

    pub fn border_x(mut self, width: f32, color: Color) -> Self {
        self.borders = self.borders.border_x(width, color);
        self
    }

    pub fn border_y(mut self, width: f32, color: Color) -> Self {
        self.borders = self.borders.border_y(width, color);
        self
    }

    pub fn box_style(mut self, style: BoxStyle) -> Self {
        if style.background.a > 0.0 {
            self.color = Some(style.background);
        } else {
            self.color = None;
        }
        self.radius = style.radius;
        let mut borders = style;
        borders.background = Color::TRANSPARENT;
        borders.radius = 0.0;
        self.borders = borders;
        self
    }

    pub fn style(self, style: BoxStyle) -> Self {
        self.box_style(style)
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.style = self.style.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.style = self.style.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.style = self.style.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.style = self.style.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.style = self.style.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.style = self.style.padding_bottom(v);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into()).height(height.into());
        self
    }

    pub fn square(mut self, side: impl Into<Length>) -> Self {
        let side = side.into();
        self.style = self.style.width(side).height(side);
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.style = self.style.overflow(overflow);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }
}

impl_style_extras!(Panel);

impl View for Panel {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        self.style
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let theme = ctx.theme();
        let color = self.color.unwrap_or(theme.surface);
        let radius = if self.radius > 0.0 {
            self.radius
        } else {
            theme.radius
        };
        let p = self.style.padding;
        let rect = ctx.rect;
        let quad = Rect::from_xywh(
            rect.left() + p.left,
            rect.top() + p.top,
            (rect.size.width - p.horizontal_sum()).max(0.0),
            (rect.size.height - p.vertical_sum()).max(0.0),
        );

        let mut deco = self.borders;
        deco.background = color;
        deco.radius = radius;
        ctx.paint_box_in(deco, quad);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Spacer {
    style: LayoutStyle,
}

impl Spacer {
    pub fn flex(weight: f32) -> Self {
        let w = if weight.is_finite() {
            weight.max(0.0)
        } else {
            0.0
        };
        Self {
            style: LayoutStyle {
                width: Length::Fill(w),
                height: Length::Fit,
                ..LayoutStyle::default()
            },
        }
    }

    pub fn vertical(weight: f32) -> Self {
        let w = if weight.is_finite() {
            weight.max(0.0)
        } else {
            0.0
        };
        Self {
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fill(w),
                ..LayoutStyle::default()
            },
        }
    }

    pub fn fixed(size: f32) -> Self {
        let s = if size.is_finite() { size.max(0.0) } else { 0.0 };
        Self {
            style: LayoutStyle {
                width: Length::Px(s),
                height: Length::Px(s),
                ..LayoutStyle::default()
            },
        }
    }

    pub fn fixed_width(w: f32) -> Self {
        let w = if w.is_finite() { w.max(0.0) } else { 0.0 };
        Self {
            style: LayoutStyle {
                width: Length::Px(w),
                height: Length::Px(0.0),
                ..LayoutStyle::default()
            },
        }
    }

    pub fn fixed_height(h: f32) -> Self {
        let h = if h.is_finite() { h.max(0.0) } else { 0.0 };
        Self {
            style: LayoutStyle {
                width: Length::Px(0.0),
                height: Length::Px(h),
                ..LayoutStyle::default()
            },
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn expand(mut self) -> Self {
        self.style = self.style.flex(1.0);
        self
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::flex(1.0)
    }
}

impl_style_extras!(Spacer);

impl_child_helpers!(Fragment);
impl_child_helpers!(FocusScope);
impl_child_helpers!(Row);
impl_child_helpers!(Column);
impl_child_helpers!(Stack);
impl_child_helpers!(Grid);
impl_child_helpers!(Wrap);
impl_child_helpers!(Masonry);

impl View for Spacer {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        self.style
    }
}

fn aspect_contain(content: Size, ratio: f32) -> (Size, f32, f32) {
    let (w, h) = (content.width.max(0.0), content.height.max(0.0));
    if !(ratio.is_finite() && ratio > 0.0) || w <= 0.0 || h <= 0.0 {
        return (Size::new(w, h), 0.0, 0.0);
    }
    if !w.is_finite() || !h.is_finite() {
        return (Size::new(w, h), 0.0, 0.0);
    }
    let box_w = (h * ratio).min(w);
    let box_h = box_w / ratio;
    (
        Size::new(box_w, box_h),
        (w - box_w) * 0.5,
        (h - box_h) * 0.5,
    )
}

fn sanitize_aspect_ratio(ratio: f32) -> f32 {
    if ratio.is_finite() && ratio > 0.0 {
        ratio
    } else {
        1.0
    }
}

#[derive(Clone)]
pub struct AspectRatio {
    ratio: f32,
    child: Option<AnyView>,
    style: LayoutStyle,
    deco: BoxStyle,
    clip: bool,
}

impl AspectRatio {
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: sanitize_aspect_ratio(ratio),
            child: None,
            style: LayoutStyle {
                width: Length::Fit,
                height: Length::Fit,
                ..LayoutStyle::default()
            },
            deco: BoxStyle::new(),
            clip: false,
        }
    }

    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = sanitize_aspect_ratio(ratio);
        self
    }

    pub fn aspect(self, ratio: f32) -> Self {
        self.ratio(ratio)
    }

    pub fn child<V: View + 'static>(mut self, child: V) -> Self {
        debug_assert!(
            self.child.is_none(),
            "AspectRatio takes a single child - a second `.child()` would overwrite the first. Use Row/Column for many."
        );
        self.child = Some(AnyView::new(child));
        self
    }

    pub fn child_if<V: View + 'static>(mut self, cond: bool, make: impl FnOnce() -> V) -> Self {
        if cond {
            self.child = Some(AnyView::new(make()));
        }
        self
    }

    pub fn child_opt<V: View + 'static>(mut self, child: Option<V>) -> Self {
        if let Some(v) = child {
            self.child = Some(AnyView::new(v));
        }
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_left(width, color);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_right(width, color);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_top(width, color);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_bottom(width, color);
        self
    }

    pub fn border_x(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_x(width, color);
        self
    }

    pub fn border_y(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border_y(width, color);
        self
    }

    pub fn box_style(mut self, style: BoxStyle) -> Self {
        self.deco = style;
        self
    }

    pub fn style(self, style: BoxStyle) -> Self {
        self.box_style(style)
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.deco = self.deco.radius(radius);
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.padding_xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.style = self.style.padding_x(x);
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.style = self.style.padding_y(y);
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.style = self.style.padding_left(v);
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.style = self.style.padding_right(v);
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.style = self.style.padding_top(v);
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.style = self.style.padding_bottom(v);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into()).height(height.into());
        self
    }

    pub fn square(mut self, side: impl Into<Length>) -> Self {
        let side = side.into();
        self.style = self.style.width(side).height(side);
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.style = self.style.overflow(overflow);
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        self.style = self.style.margin(margin);
        self
    }

    pub fn margin_xy(mut self, x: f32, y: f32) -> Self {
        self.style = self.style.margin_xy(x, y);
        self
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        self.style = self.style.margin_x(x);
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        self.style = self.style.margin_y(y);
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.style = self.style.margin_left(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.style = self.style.margin_right(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.style = self.style.margin_top(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.style = self.style.margin_bottom(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.style = self.style.flex_shrink(shrink);
        self
    }

    pub fn flex_basis(mut self, basis: Option<Length>) -> Self {
        self.style = self.style.flex_basis(basis);
        self
    }

    pub fn align_self(mut self, align: Option<Align>) -> Self {
        self.style = self.style.align_self(align);
        self
    }

    pub fn position(mut self, position: Position) -> Self {
        self.style = self.style.position(position);
        self
    }

    pub fn absolute(mut self) -> Self {
        self.style = self.style.absolute();
        self
    }

    pub fn absolute_at(mut self, left: f32, top: f32) -> Self {
        self.style = self.style.absolute_at(left, top);
        self
    }

    pub fn absolute_inset(mut self, inset: Inset) -> Self {
        self.style = self.style.absolute_inset(inset);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }

    pub fn fill_size(self) -> Self {
        self.fill_width().fill_height()
    }

    pub fn fit(mut self) -> Self {
        self.style = self.style.fit();
        self
    }

    pub fn hit_testable(mut self, on: bool) -> Self {
        self.style = self.style.hit_testable(on);
        self
    }

    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    pub fn clip_children(self) -> Self {
        self.clip(true)
    }
}

impl View for AspectRatio {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn children_override(&self) -> Option<Vec<AnyView>> {
        self.child.clone().map(|c| vec![c])
    }
    fn clip_self(&self) -> bool {
        self.clip
    }
    fn clip_radius(&self) -> f32 {
        if self.clip { self.deco.radius } else { 0.0 }
    }
    fn layout_style(&self) -> LayoutStyle {
        let mut style = self.style;
        style.aspect_ratio = Some(self.ratio);
        style
    }
    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        let children = rt.children(el);
        let Some(&child) = children.first() else {
            return true;
        };

        if matches!(self.style.width, Length::Fit) || matches!(self.style.height, Length::Fit) {
            return false;
        }
        let pad = self.style.padding;
        let content = Rect::from_xywh(
            rect.left() + pad.left,
            rect.top() + pad.top,
            (rect.size.width - pad.horizontal_sum()).max(0.0),
            (rect.size.height - pad.vertical_sum()).max(0.0),
        );
        if !content.size.width.is_finite() || !content.size.height.is_finite() {
            return false;
        }
        let (boxed, dx, dy) = aspect_contain(content.size, self.ratio);
        let cs = rt.child_layout_style(child);
        rt.arrange_node(
            child,
            Rect::from_xywh(
                content.left() + dx + cs.margin.left,
                content.top() + dy + cs.margin.top,
                (boxed.width - cs.margin.horizontal_sum()).max(0.0),
                (boxed.height - cs.margin.vertical_sum()).max(0.0),
            ),
            ctx,
        );
        true
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.paint_box(self.deco);
    }
}

impl crate::styleable::Styleable for AspectRatio {
    fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }
    fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }
    fn fit(mut self) -> Self {
        self.style = self.style.fit();
        self
    }
    fn hit_testable(mut self, on: bool) -> Self {
        self.style = self.style.hit_testable(on);
        self
    }
    fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }
}

impl crate::styleable::Decoratable for AspectRatio {
    fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }
    fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }
    fn radius(mut self, radius: f32) -> Self {
        self.deco = self.deco.radius(radius);
        self
    }
    fn box_style(mut self, style: BoxStyle) -> Self {
        self.deco = style;
        self
    }
}

impl crate::styleable::Styleable for Row {
    fn fill_width(mut self) -> Self {
        self.0.style = self.0.style.width(Length::Fill(1.0));
        self
    }
    fn fill_height(mut self) -> Self {
        self.0.style = self.0.style.height(Length::Fill(1.0));
        self
    }
    fn fit(mut self) -> Self {
        self.0.style = self.0.style.fit();
        self
    }
    fn hit_testable(mut self, on: bool) -> Self {
        self.0.style = self.0.style.hit_testable(on);
        self
    }
    fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.0.style = self.0.style.padding(padding);
        self
    }
}

impl crate::styleable::Decoratable for Row {
    fn background(mut self, color: Color) -> Self {
        self.0.deco = self.0.deco.background(color);
        self
    }
    fn border(mut self, width: f32, color: Color) -> Self {
        self.0.deco = self.0.deco.border(width, color);
        self
    }
    fn radius(mut self, radius: f32) -> Self {
        self.0.deco = self.0.deco.radius(radius);
        self
    }
    fn box_style(mut self, style: BoxStyle) -> Self {
        self.0.deco = style;
        self
    }
}

impl crate::styleable::Styleable for Column {
    fn fill_width(mut self) -> Self {
        self.0.style = self.0.style.width(Length::Fill(1.0));
        self
    }
    fn fill_height(mut self) -> Self {
        self.0.style = self.0.style.height(Length::Fill(1.0));
        self
    }
    fn fit(mut self) -> Self {
        self.0.style = self.0.style.fit();
        self
    }
    fn hit_testable(mut self, on: bool) -> Self {
        self.0.style = self.0.style.hit_testable(on);
        self
    }
    fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.0.style = self.0.style.padding(padding);
        self
    }
}

impl crate::styleable::Decoratable for Column {
    fn background(mut self, color: Color) -> Self {
        self.0.deco = self.0.deco.background(color);
        self
    }
    fn border(mut self, width: f32, color: Color) -> Self {
        self.0.deco = self.0.deco.border(width, color);
        self
    }
    fn radius(mut self, radius: f32) -> Self {
        self.0.deco = self.0.deco.radius(radius);
        self
    }
    fn box_style(mut self, style: BoxStyle) -> Self {
        self.0.deco = style;
        self
    }
}

impl crate::styleable::Styleable for Stack {
    fn fill_width(mut self) -> Self {
        self.style = self.style.width(Length::Fill(1.0));
        self
    }
    fn fill_height(mut self) -> Self {
        self.style = self.style.height(Length::Fill(1.0));
        self
    }
    fn fit(mut self) -> Self {
        self.style = self.style.fit();
        self
    }
    fn hit_testable(mut self, on: bool) -> Self {
        self.style = self.style.hit_testable(on);
        self
    }
    fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.style = self.style.padding(padding);
        self
    }
}

impl crate::styleable::Decoratable for Stack {
    fn background(mut self, color: Color) -> Self {
        self.deco = self.deco.background(color);
        self
    }
    fn border(mut self, width: f32, color: Color) -> Self {
        self.deco = self.deco.border(width, color);
        self
    }
    fn radius(mut self, radius: f32) -> Self {
        self.deco = self.deco.radius(radius);
        self
    }
    fn box_style(mut self, style: BoxStyle) -> Self {
        self.deco = style;
        self
    }
}

#[cfg(test)]
mod aspect_ratio_tests {
    use super::*;

    #[test]
    fn invalid_ratio_falls_back_to_square() {
        for bad in [0.0, -2.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let v = AspectRatio::new(bad);
            assert_eq!(v.layout_style().aspect_ratio, Some(1.0), "ratio {bad}");
            let v = AspectRatio::new(2.0).ratio(bad);
            assert_eq!(v.layout_style().aspect_ratio, Some(1.0), "ratio {bad}");
        }
    }

    #[test]
    fn layout_style_carries_ratio() {
        let v = AspectRatio::new(16.0 / 9.0).width(320.0);
        let style = v.layout_style();
        assert_eq!(style.aspect_ratio, Some(16.0 / 9.0));
        assert_eq!(style.width, Length::Px(320.0));
    }

    #[test]
    fn contain_fits_wide_content() {
        let (boxed, dx, dy) = aspect_contain(Size::new(800.0, 600.0), 16.0 / 9.0);
        assert!((boxed.width - 800.0).abs() < 1e-3, "{boxed:?}");
        assert!((boxed.height - 450.0).abs() < 1e-3, "{boxed:?}");
        assert!(dx.abs() < 1e-3, "{dx}");
        assert!((dy - 75.0).abs() < 1e-3, "{dy}");
    }

    #[test]
    fn contain_fits_tall_content() {
        let (boxed, dx, dy) = aspect_contain(Size::new(100.0, 200.0), 2.0);
        assert!((boxed.width - 100.0).abs() < 1e-3, "{boxed:?}");
        assert!((boxed.height - 50.0).abs() < 1e-3, "{boxed:?}");
        assert!(dx.abs() < 1e-3, "{dx}");
        assert!((dy - 75.0).abs() < 1e-3, "{dy}");
    }

    #[test]
    fn contain_matching_ratio_is_identity() {
        let (boxed, dx, dy) = aspect_contain(Size::new(200.0, 100.0), 2.0);
        assert!((boxed.width - 200.0).abs() < 1e-3, "{boxed:?}");
        assert!((boxed.height - 100.0).abs() < 1e-3, "{boxed:?}");
        assert!(dx.abs() < 1e-3);
        assert!(dy.abs() < 1e-3);
    }

    #[test]
    fn contain_degenerate_returns_content() {
        let (boxed, dx, dy) = aspect_contain(Size::ZERO, 16.0 / 9.0);
        assert_eq!(boxed, Size::ZERO);
        assert_eq!((dx, dy), (0.0, 0.0));
        let (boxed, dx, dy) = aspect_contain(Size::new(100.0, 100.0), 0.0);
        assert_eq!(boxed, Size::new(100.0, 100.0));
        assert_eq!((dx, dy), (0.0, 0.0));
    }
}
