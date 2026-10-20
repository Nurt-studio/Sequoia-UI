use sequoia_core::{Color, Edges, Length, Size};
use sequoia_layout::{LayoutCtx, LayoutStyle};
use sequoia_style::AvatarStyle;
use sequoia_view::{AssetRegistry, FontWeight, PaintCtx, Unit, View, icon_key};

const SIZE: f32 = 32.0;
const FONT_RATIO: f32 = 0.38;
const ICON_RATIO: f32 = 0.55;

pub fn initials_for(name: &str) -> String {
    let mut words = name.split_whitespace();
    let first = words.next().unwrap_or("");
    let last = words.last();
    let mut out = String::new();
    if let Some(c) = first.chars().next() {
        out.extend(c.to_uppercase());
    }
    if let Some(last) = last
        && let Some(c) = last.chars().next()
    {
        out.extend(c.to_uppercase());
    }
    if out.is_empty() { "?".to_string() } else { out }
}

pub fn avatar_color_for(name: &str) -> Color {
    const PALETTE: [Color; 8] = [
        Color::hex(0x356FD1),
        Color::hex(0x7A5ACD),
        Color::hex(0x2E8B57),
        Color::hex(0xB3541E),
        Color::hex(0xA03A5A),
        Color::hex(0x2A9D8F),
        Color::hex(0x8A6D1B),
        Color::hex(0x5A6E7F),
    ];
    let mut hash: u32 = 5381;
    for b in name.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }

    PALETTE[(hash as usize) % PALETTE.len()]
}

pub struct Avatar {
    name: String,
    initials: Option<String>,
    icon: Option<String>,
    size: f32,
    auto_color: bool,
    style: Option<AvatarStyle>,
    background: Option<Color>,
    text_color: Option<Color>,
    font_size: Option<f32>,
    radius: Option<f32>,
    margin: Edges,
}

impl Avatar {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            initials: None,
            icon: None,
            size: SIZE,
            auto_color: true,
            style: None,
            background: None,
            text_color: None,
            font_size: None,
            radius: None,
            margin: Edges::ZERO,
        }
    }

    pub fn icon(name: impl Into<String>) -> Self {
        Self {
            name: String::new(),
            initials: None,
            icon: Some(name.into()),
            size: SIZE,
            auto_color: true,
            style: None,
            background: None,
            text_color: None,
            font_size: None,
            radius: None,
            margin: Edges::ZERO,
        }
    }

    pub fn initials(mut self, initials: impl Into<String>) -> Self {
        self.initials = Some(initials.into());
        self
    }

    pub fn size(mut self, px: f32) -> Self {
        self.size = px.max(8.0);
        self
    }

    pub fn auto_color(mut self, on: bool) -> Self {
        self.auto_color = on;
        self
    }

    pub fn style(mut self, style: AvatarStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn margin(mut self, margin: impl Into<Edges>) -> Self {
        let m = margin.into();
        self.margin = Edges::only(
            sanitize_margin(m.left),
            sanitize_margin(m.top),
            sanitize_margin(m.right),
            sanitize_margin(m.bottom),
        );
        self
    }

    pub fn margin_xy(self, x: f32, y: f32) -> Self {
        self.margin(Edges::xy(x, y))
    }

    pub fn margin_x(mut self, x: f32) -> Self {
        let x = sanitize_margin(x);
        self.margin.left = x;
        self.margin.right = x;
        self
    }

    pub fn margin_y(mut self, y: f32) -> Self {
        let y = sanitize_margin(y);
        self.margin.top = y;
        self.margin.bottom = y;
        self
    }

    pub fn margin_left(mut self, v: f32) -> Self {
        self.margin.left = sanitize_margin(v);
        self
    }

    pub fn margin_right(mut self, v: f32) -> Self {
        self.margin.right = sanitize_margin(v);
        self
    }

    pub fn margin_top(mut self, v: f32) -> Self {
        self.margin.top = sanitize_margin(v);
        self
    }

    pub fn margin_bottom(mut self, v: f32) -> Self {
        self.margin.bottom = sanitize_margin(v);
        self
    }

    fn label(&self) -> String {
        self.initials
            .clone()
            .unwrap_or_else(|| initials_for(&self.name))
    }
}

pub(crate) fn sanitize_margin(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl View for Avatar {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn hit_testable(&self) -> bool {
        false
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Px(self.size),
            height: Length::Px(self.size),
            margin: self.margin,
            ..LayoutStyle::default().hit_testable(false)
        }
    }
    fn intrinsic_size(&self, _ctx: &mut LayoutCtx) -> Option<Size> {
        Some(Size::new(self.size, self.size))
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        let rect = ctx.rect;
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        let theme = ctx.theme();
        let style = self.style.unwrap_or_default();
        let side = rect.size.width.min(rect.size.height);
        let radius = self.radius.or(style.radius).unwrap_or(side * 0.5).max(0.0);
        let background = self.background.or(style.background).unwrap_or_else(|| {
            if self.auto_color && self.icon.is_none() {
                avatar_color_for(&self.name)
            } else {
                theme.surface
            }
        });
        ctx.quad(rect, background, radius);

        let text = self.text_color.or(style.text).unwrap_or(theme.text);
        if let Some(name) = &self.icon {
            let px = side * ICON_RATIO;
            if let Some(assets) = ctx.environment.get::<AssetRegistry>() {
                let raster_px = ((px * ctx.scale_factor()).round() as u32).max(1);
                if let Ok(sprite) = assets.rasterize_icon(name, raster_px) {
                    let x = rect.left() + (rect.size.width - px) * 0.5;
                    let y = rect.top() + (rect.size.height - px) * 0.5;
                    ctx.draw_sprite(
                        icon_key(name, raster_px),
                        sprite.width,
                        sprite.height,
                        &sprite.alpha,
                        sequoia_core::Rect::from_xywh(x, y, px, px),
                        text,
                    );
                    return;
                }
            }
        }
        let label = self.label();
        let size = self
            .font_size
            .or(style.font_size)
            .unwrap_or(side * FONT_RATIO);
        let w = ctx.text_width_weighted(&label, size, FontWeight::SEMIBOLD);
        let x = rect.left() + (rect.size.width - w) * 0.5;
        let y = rect.top() + rect.size.height * 0.5 + size * 0.35;
        ctx.draw_text_weighted(&label, (x, y), size, FontWeight::SEMIBOLD, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_take_first_and_last() {
        assert_eq!(initials_for("Ada Lovelace"), "AL");
        assert_eq!(initials_for("ada lovelace byron"), "AB");
        assert_eq!(initials_for("root"), "R");
        assert_eq!(initials_for("  "), "?");
        assert_eq!(initials_for(""), "?");
    }

    #[test]
    fn color_is_stable_and_spread() {
        assert_eq!(avatar_color_for("ada"), avatar_color_for("ada"));
        let mut seen = std::collections::HashSet::new();
        for name in [
            "ada", "bob", "ced", "dan", "eli", "fay", "gus", "hal", "ida", "jay",
        ] {
            seen.insert(avatar_color_for(name).to_rgb8());
        }
        assert!(seen.len() > 1, "palette differs: {seen:?}");
    }
}
