use std::rc::Rc;

use crate::geometry::{Rect, Transform, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,

    pub g: f32,

    pub b: f32,

    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };

    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub const ALICEBLUE: Self = Self::hex(0xF0F8FF);

    pub const ANTIQUEWHITE: Self = Self::hex(0xFAEBD7);

    pub const AQUA: Self = Self::hex(0x00FFFF);

    pub const AQUAMARINE: Self = Self::hex(0x7FFFD4);

    pub const AZURE: Self = Self::hex(0xF0FFFF);

    pub const BEIGE: Self = Self::hex(0xF5F5DC);

    pub const BISQUE: Self = Self::hex(0xFFE4C4);

    pub const BLANCHEDALMOND: Self = Self::hex(0xFFEBCD);

    pub const BLUEVIOLET: Self = Self::hex(0x8A2BE2);

    pub const BROWN: Self = Self::hex(0xA52A2A);

    pub const BURLYWOOD: Self = Self::hex(0xDEB887);

    pub const CADETBLUE: Self = Self::hex(0x5F9EA0);

    pub const CHARTREUSE: Self = Self::hex(0x7FFF00);

    pub const CHOCOLATE: Self = Self::hex(0xD2691E);

    pub const CORAL: Self = Self::hex(0xFF7F50);

    pub const CORNFLOWERBLUE: Self = Self::hex(0x6495ED);

    pub const CORNSILK: Self = Self::hex(0xFFF8DC);

    pub const CRIMSON: Self = Self::hex(0xDC143C);

    pub const CYAN: Self = Self::hex(0x00FFFF);

    pub const DARKBLUE: Self = Self::hex(0x00008B);

    pub const DARKCYAN: Self = Self::hex(0x008B8B);

    pub const DARKGOLDENROD: Self = Self::hex(0xB8860B);

    pub const DARKGRAY: Self = Self::hex(0xA9A9A9);

    pub const DARKGREEN: Self = Self::hex(0x006400);

    pub const DARKGREY: Self = Self::hex(0xA9A9A9);

    pub const DARKKHAKI: Self = Self::hex(0xBDB76B);

    pub const DARKMAGENTA: Self = Self::hex(0x8B008B);

    pub const DARKOLIVEGREEN: Self = Self::hex(0x556B2F);

    pub const DARKORANGE: Self = Self::hex(0xFF8C00);

    pub const DARKORCHID: Self = Self::hex(0x9932CC);

    pub const DARKRED: Self = Self::hex(0x8B0000);

    pub const DARKSALMON: Self = Self::hex(0xE9967A);

    pub const DARKSEAGREEN: Self = Self::hex(0x8FBC8F);

    pub const DARKSLATEBLUE: Self = Self::hex(0x483D8B);

    pub const DARKSLATEGRAY: Self = Self::hex(0x2F4F4F);

    pub const DARKSLATEGREY: Self = Self::hex(0x2F4F4F);

    pub const DARKTURQUOISE: Self = Self::hex(0x00CED1);

    pub const DARKVIOLET: Self = Self::hex(0x9400D3);

    pub const DEEPPINK: Self = Self::hex(0xFF1493);

    pub const DEEPSKYBLUE: Self = Self::hex(0x00BFFF);

    pub const DIMGRAY: Self = Self::hex(0x696969);

    pub const DIMGREY: Self = Self::hex(0x696969);

    pub const DODGERBLUE: Self = Self::hex(0x1E90FF);

    pub const FIREBRICK: Self = Self::hex(0xB22222);

    pub const FLORALWHITE: Self = Self::hex(0xFFFAF0);

    pub const FORESTGREEN: Self = Self::hex(0x228B22);

    pub const FUCHSIA: Self = Self::hex(0xFF00FF);

    pub const GAINSBORO: Self = Self::hex(0xDCDCDC);

    pub const GHOSTWHITE: Self = Self::hex(0xF8F8FF);

    pub const GOLD: Self = Self::hex(0xFFD700);

    pub const GOLDENROD: Self = Self::hex(0xDAA520);

    pub const GRAY: Self = Self::hex(0x808080);

    pub const GREENYELLOW: Self = Self::hex(0xADFF2F);

    pub const GREY: Self = Self::hex(0x808080);

    pub const HONEYDEW: Self = Self::hex(0xF0FFF0);

    pub const HOTPINK: Self = Self::hex(0xFF69B4);

    pub const INDIANRED: Self = Self::hex(0xCD5C5C);

    pub const INDIGO: Self = Self::hex(0x4B0082);

    pub const IVORY: Self = Self::hex(0xFFFFF0);

    pub const KHAKI: Self = Self::hex(0xF0E68C);

    pub const LAVENDER: Self = Self::hex(0xE6E6FA);

    pub const LAVENDERBLUSH: Self = Self::hex(0xFFF0F5);

    pub const LAWNGREEN: Self = Self::hex(0x7CFC00);

    pub const LEMONCHIFFON: Self = Self::hex(0xFFFACD);

    pub const LIGHTBLUE: Self = Self::hex(0xADD8E6);

    pub const LIGHTCORAL: Self = Self::hex(0xF08080);

    pub const LIGHTCYAN: Self = Self::hex(0xE0FFFF);

    pub const LIGHTGOLDENRODYELLOW: Self = Self::hex(0xFAFAD2);

    pub const LIGHTGRAY: Self = Self::hex(0xD3D3D3);

    pub const LIGHTGREEN: Self = Self::hex(0x90EE90);

    pub const LIGHTGREY: Self = Self::hex(0xD3D3D3);

    pub const LIGHTPINK: Self = Self::hex(0xFFB6C1);

    pub const LIGHTSALMON: Self = Self::hex(0xFFA07A);

    pub const LIGHTSEAGREEN: Self = Self::hex(0x20B2AA);

    pub const LIGHTSKYBLUE: Self = Self::hex(0x87CEFA);

    pub const LIGHTSLATEGRAY: Self = Self::hex(0x778899);

    pub const LIGHTSLATEGREY: Self = Self::hex(0x778899);

    pub const LIGHTSTEELBLUE: Self = Self::hex(0xB0C4DE);

    pub const LIGHTYELLOW: Self = Self::hex(0xFFFFE0);

    pub const LIME: Self = Self::hex(0x00FF00);

    pub const LIMEGREEN: Self = Self::hex(0x32CD32);

    pub const LINEN: Self = Self::hex(0xFAF0E6);

    pub const MAGENTA: Self = Self::hex(0xFF00FF);

    pub const MAROON: Self = Self::hex(0x800000);

    pub const MEDIUMAQUAMARINE: Self = Self::hex(0x66CDAA);

    pub const MEDIUMBLUE: Self = Self::hex(0x0000CD);

    pub const MEDIUMORCHID: Self = Self::hex(0xBA55D3);

    pub const MEDIUMPURPLE: Self = Self::hex(0x9370DB);

    pub const MEDIUMSEAGREEN: Self = Self::hex(0x3CB371);

    pub const MEDIUMSLATEBLUE: Self = Self::hex(0x7B68EE);

    pub const MEDIUMSPRINGGREEN: Self = Self::hex(0x00FA9A);

    pub const MEDIUMTURQUOISE: Self = Self::hex(0x48D1CC);

    pub const MEDIUMVIOLETRED: Self = Self::hex(0xC71585);

    pub const MIDNIGHTBLUE: Self = Self::hex(0x191970);

    pub const MINTCREAM: Self = Self::hex(0xF5FFFA);

    pub const MISTYROSE: Self = Self::hex(0xFFE4E1);

    pub const MOCCASIN: Self = Self::hex(0xFFE4B5);

    pub const NAVAJOWHITE: Self = Self::hex(0xFFDEAD);

    pub const NAVY: Self = Self::hex(0x000080);

    pub const OLDLACE: Self = Self::hex(0xFDF5E6);

    pub const OLIVE: Self = Self::hex(0x808000);

    pub const OLIVEDRAB: Self = Self::hex(0x6B8E23);

    pub const ORANGE: Self = Self::hex(0xFFA500);

    pub const ORANGERED: Self = Self::hex(0xFF4500);

    pub const ORCHID: Self = Self::hex(0xDA70D6);

    pub const PALEGOLDENROD: Self = Self::hex(0xEEE8AA);

    pub const PALEGREEN: Self = Self::hex(0x98FB98);

    pub const PALETURQUOISE: Self = Self::hex(0xAFEEEE);

    pub const PALEVIOLETRED: Self = Self::hex(0xDB7093);

    pub const PAPAYAWHIP: Self = Self::hex(0xFFEFD5);

    pub const PEACHPUFF: Self = Self::hex(0xFFDAB9);

    pub const PERU: Self = Self::hex(0xCD853F);

    pub const PINK: Self = Self::hex(0xFFC0CB);

    pub const PLUM: Self = Self::hex(0xDDA0DD);

    pub const POWDERBLUE: Self = Self::hex(0xB0E0E6);

    pub const PURPLE: Self = Self::hex(0x800080);

    pub const REBECCAPURPLE: Self = Self::hex(0x663399);

    pub const ROSYBROWN: Self = Self::hex(0xBC8F8F);

    pub const ROYALBLUE: Self = Self::hex(0x4169E1);

    pub const SADDLEBROWN: Self = Self::hex(0x8B4513);

    pub const SALMON: Self = Self::hex(0xFA8072);

    pub const SANDYBROWN: Self = Self::hex(0xF4A460);

    pub const SEAGREEN: Self = Self::hex(0x2E8B57);

    pub const SEASHELL: Self = Self::hex(0xFFF5EE);

    pub const SIENNA: Self = Self::hex(0xA0522D);

    pub const SILVER: Self = Self::hex(0xC0C0C0);

    pub const SKYBLUE: Self = Self::hex(0x87CEEB);

    pub const SLATEBLUE: Self = Self::hex(0x6A5ACD);

    pub const SLATEGRAY: Self = Self::hex(0x708090);

    pub const SLATEGREY: Self = Self::hex(0x708090);

    pub const SNOW: Self = Self::hex(0xFFFAFA);

    pub const SPRINGGREEN: Self = Self::hex(0x00FF7F);

    pub const STEELBLUE: Self = Self::hex(0x4682B4);

    pub const TAN: Self = Self::hex(0xD2B48C);

    pub const TEAL: Self = Self::hex(0x008080);

    pub const THISTLE: Self = Self::hex(0xD8BFD8);

    pub const TOMATO: Self = Self::hex(0xFF6347);

    pub const TURQUOISE: Self = Self::hex(0x40E0D0);

    pub const VIOLET: Self = Self::hex(0xEE82EE);

    pub const WHEAT: Self = Self::hex(0xF5DEB3);

    pub const WHITESMOKE: Self = Self::hex(0xF5F5F5);

    pub const YELLOW: Self = Self::hex(0xFFFF00);

    pub const YELLOWGREEN: Self = Self::hex(0x9ACD32);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    pub const fn hex(hex: u32) -> Self {
        Self::rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
    }

    pub const fn hexa(hex: u32) -> Self {
        let mut c = Self::hex(hex >> 8);
        c.a = (hex as u8) as f32 / 255.0;
        c
    }

    pub fn to_linear(self) -> Self {
        fn channel(c: f32) -> f32 {
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        Self {
            r: channel(self.r),
            g: channel(self.g),
            b: channel(self.b),
            a: self.a,
        }
    }

    pub fn from_linear(self) -> Self {
        fn channel(c: f32) -> f32 {
            if c <= 0.003_130_8 {
                c * 12.92
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        }
        Self {
            r: channel(self.r),
            g: channel(self.g),
            b: channel(self.b),
            a: self.a,
        }
    }

    pub fn parse_hex(s: &str) -> Option<Self> {
        let h = s.strip_prefix('#').unwrap_or(s);
        let h = h.strip_prefix("0x").unwrap_or(h);
        let d = |c: u8| (c as char).to_digit(16);
        let bytes: [u8; 4] = match h.len() {
            3 | 4 => {
                let mut out = [0u8; 4];
                let mut it = h.bytes();
                for slot in out.iter_mut().take(h.len()) {
                    let v = d(it.next()?)? as u8;
                    *slot = v * 16 + v;
                }
                if h.len() == 3 {
                    out[3] = 0xFF;
                }
                out
            }
            6 | 8 => {
                let b = h.as_bytes();
                let pair =
                    |i: usize| d(b[i]).and_then(|hi| d(b[i + 1]).map(|lo| (hi * 16 + lo) as u8));
                let mut out = [pair(0)?, pair(2)?, pair(4)?, 0xFF];
                if h.len() == 8 {
                    out[3] = pair(6)?;
                }
                out
            }
            _ => return None,
        };
        Some(Self {
            r: bytes[0] as f32 / 255.0,
            g: bytes[1] as f32 / 255.0,
            b: bytes[2] as f32 / 255.0,
            a: bytes[3] as f32 / 255.0,
        })
    }

    pub fn to_rgb8(self) -> (u8, u8, u8) {
        let q = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        (q(self.r), q(self.g), q(self.b))
    }

    pub fn to_rgba8(self) -> (u8, u8, u8, u8) {
        let q = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        (q(self.r), q(self.g), q(self.b), q(self.a))
    }

    pub fn to_hex_string(self) -> String {
        let (r, g, b) = self.to_rgb8();
        format!("#{r:02X}{g:02X}{b:02X}")
    }

    pub fn to_hex8_string(self) -> String {
        let (r, g, b, a) = self.to_rgba8();
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }

    pub fn with_alpha(self, alpha: f32) -> Self {
        Self {
            a: alpha.clamp(0.0, 1.0),
            ..self
        }
    }

    pub fn to_hsva(self) -> (f32, f32, f32, f32) {
        let (r, g, b) = (
            self.r.clamp(0.0, 1.0),
            self.g.clamp(0.0, 1.0),
            self.b.clamp(0.0, 1.0),
        );
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let v = max;
        let s = if max <= f32::EPSILON { 0.0 } else { d / max };
        let h = if d <= f32::EPSILON {
            0.0
        } else if max == r {
            (((g - b) / d) % 6.0 + 6.0) % 6.0 / 6.0
        } else if max == g {
            ((b - r) / d + 2.0) / 6.0
        } else {
            ((r - g) / d + 4.0) / 6.0
        };
        (h, s, v, self.a.clamp(0.0, 1.0))
    }

    pub fn from_hsva(h: f32, s: f32, v: f32, a: f32) -> Self {
        let h = if h.is_finite() {
            h.rem_euclid(1.0)
        } else {
            0.0
        };
        let (s, v, a) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0), a.clamp(0.0, 1.0));
        if s <= f32::EPSILON {
            return Self {
                r: v,
                g: v,
                b: v,
                a,
            };
        }
        let sector = h * 6.0;
        let i = sector.floor() as i32;
        let f = sector - sector.floor();
        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));
        let (r, g, b) = match i.rem_euclid(6) {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Self { r, g, b, a }
    }

    pub fn from_hsl(h_deg: f32, s: f32, l: f32) -> Self {
        let h = if h_deg.is_finite() {
            h_deg.rem_euclid(360.0)
        } else {
            0.0
        };
        let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
        let a = s * l.min(1.0 - l);
        let f = |n: f32| {
            let k = (n + h / 30.0) % 12.0;
            l - a * (-1.0f32).max((k - 3.0).min(9.0 - k).min(1.0))
        };
        Self {
            r: f(0.0).clamp(0.0, 1.0),
            g: f(8.0).clamp(0.0, 1.0),
            b: f(4.0).clamp(0.0, 1.0),
            a: 1.0,
        }
    }

    pub fn to_hsl(self) -> (f32, f32, f32) {
        let (r, g, b) = (
            self.r.clamp(0.0, 1.0),
            self.g.clamp(0.0, 1.0),
            self.b.clamp(0.0, 1.0),
        );
        let mx = r.max(g).max(b);
        let mn = r.min(g).min(b);
        let d = mx - mn;
        let l = ((mx + mn) * 0.5).clamp(0.0, 1.0);
        if d <= f32::EPSILON {
            return (0.0, 0.0, l);
        }
        let s = (d / (1.0 - (2.0 * l - 1.0).abs())).clamp(0.0, 1.0);
        let h = if mx == r {
            ((g - b) / d) % 6.0
        } else if mx == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        } * 60.0;
        (h.rem_euclid(360.0), s, l)
    }

    pub fn to_hex6(self) -> String {
        let (r, g, b) = self.to_rgb8();
        format!("{r:02X}{g:02X}{b:02X}")
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub rect: Rect,

    pub color: Color,

    pub radius: f32,

    pub border_color: Color,

    pub border_width: f32,
}

impl Quad {
    pub fn has_border(self) -> bool {
        self.border_width > 0.0 && self.border_color.a > 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub rect: Rect,

    pub color: Color,

    pub radius: f32,

    pub blur: f32,

    pub spread: f32,

    pub offset: Vec2,

    pub inset: bool,
}

impl Shadow {
    pub fn is_visible(&self) -> bool {
        !self.rect.is_empty() && self.color.a > 0.0 && self.blur > 0.0 && self.blur.is_finite()
    }

    pub fn bounds(&self) -> Rect {
        if self.inset {
            return self.rect;
        }
        let margin = self.blur * 0.5 + self.spread.max(0.0);
        Rect::from_xywh(
            self.rect.left() + self.offset.x - margin,
            self.rect.top() + self.offset.y - margin,
            self.rect.size.width + margin * 2.0,
            self.rect.size.height + margin * 2.0,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphQuad {
    pub rect: Rect,

    pub uv: [f32; 4],

    pub color_glyph: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextDraw {
    pub color: Color,

    pub glyphs: Vec<GlyphQuad>,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ImageSource {
    File(String),

    Rgba {
        id: u64,

        width: u32,

        height: u32,

        data: Rc<[u8]>,
    },

    Dynamic {
        id: u64,

        width: u32,

        height: u32,

        data: Rc<[u8]>,

        version: u64,
    },
}

impl ImageSource {
    pub fn cache_key(&self) -> u64 {
        match self {
            Self::File(path) => {
                let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
                for byte in path.as_bytes() {
                    hash ^= u64::from(*byte);
                    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                }
                hash ^ 0x9e37_79b9_7f4a_7c15
            }
            Self::Rgba { id, .. } => *id,
            Self::Dynamic { id, .. } => *id,
        }
    }

    pub fn dynamic_version(&self) -> Option<u64> {
        match self {
            Self::Dynamic { version, .. } => Some(*version),
            Self::File(_) | Self::Rgba { .. } => None,
        }
    }

    pub fn extent(&self) -> Option<(u32, u32)> {
        match self {
            Self::File(_) => None,
            Self::Rgba { width, height, .. } | Self::Dynamic { width, height, .. } => {
                Some((*width, *height))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageDraw {
    pub rect: Rect,

    pub uv: [f32; 4],

    pub source: ImageSource,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaveformDraw {
    pub bars: Vec<Rect>,

    pub color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropDraw {
    pub rect: Rect,

    pub radius: f32,

    pub blur: f32,

    pub dim: Color,

    pub noise: f32,

    pub mask: SpanMask,
}

impl BackdropDraw {
    pub fn is_visible(self) -> bool {
        !self.rect.is_empty()
            && (self.blur > 0.0 && self.blur.is_finite() || self.dim.a > 0.0 || self.noise > 0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
#[derive(Default)]
pub enum SpanMask {
    #[default]
    None,

    Linear {
        from: f32,
        to: f32,
        angle_rad: f32,
    },

    Uniform(f32),
}

impl SpanMask {
    pub fn mask_vec(self) -> [f32; 4] {
        match self {
            SpanMask::None => [0.0, 0.0, 0.0, 0.0],
            SpanMask::Linear {
                from,
                to,
                angle_rad,
            } => {
                let angle = if angle_rad.is_finite() {
                    angle_rad
                } else {
                    0.0
                };
                [1.0, from.clamp(0.0, 1.0), to.clamp(0.0, 1.0), angle]
            }
            SpanMask::Uniform(v) => [2.0, v.clamp(0.0, 1.0), 0.0, 0.0],
        }
    }

    pub fn is_none(self) -> bool {
        matches!(self, SpanMask::None)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropSpan {
    pub at: usize,

    pub draw: BackdropDraw,

    pub clip: Option<RoundedClip>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilterDraw {
    pub rect: Rect,

    pub radius: f32,

    pub blur: f32,

    pub mask: SpanMask,
}

impl FilterDraw {
    pub fn is_visible(self) -> bool {
        !self.rect.is_empty() && self.blur > 0.0 && self.blur.is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilterSpan {
    pub start: usize,

    pub end: usize,

    pub draw: FilterDraw,

    pub clip: Option<RoundedClip>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientDraw {
    pub rect: Rect,

    pub radius: f32,

    pub kind: crate::brush::GradientKind,

    pub stops: crate::brush::GradientStops,
}

impl GradientDraw {
    pub fn is_visible(&self) -> bool {
        !self.rect.is_empty() && !self.stops.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum RenderItem {
    Quad(Quad),

    Text(TextDraw),

    Image(ImageDraw),

    Shadow(Shadow),

    Waveform(WaveformDraw),

    Gradient(GradientDraw),
}

pub const MAX_WAVEFORM_BARS: usize = 8192;

fn clip_uv(rect: Rect, uv: [f32; 4], clipped: Rect) -> [f32; 4] {
    let (w, h) = (rect.size.width, rect.size.height);
    if w <= 0.0 || h <= 0.0 {
        return uv;
    }
    let [u0, v0, u1, v1] = uv;
    let fx0 = (clipped.left() - rect.left()) / w;
    let fx1 = (clipped.right() - rect.left()) / w;
    let fy0 = (clipped.top() - rect.top()) / h;
    let fy1 = (clipped.bottom() - rect.top()) / h;
    [
        u0 + fx0 * (u1 - u0),
        v0 + fy0 * (v1 - v0),
        u0 + fx1 * (u1 - u0),
        v0 + fy1 * (v1 - v0),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundedClip {
    pub rect: Rect,

    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ClipEntry {
    clip: RoundedClip,

    under: Transform,
}

impl RoundedClip {
    pub fn new(rect: Rect, radius: f32) -> Self {
        let radius = if radius.is_finite() && radius > 0.0 {
            radius
        } else {
            0.0
        };
        Self { rect, radius }
    }

    pub fn is_rounded(self) -> bool {
        self.radius > 0.0 && !self.rect.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderList {
    items: Vec<RenderItem>,
    clip_stack: Vec<ClipEntry>,
    opacity_stack: Vec<f32>,
    transform_stack: Vec<Transform>,

    item_clips: Vec<Option<RoundedClip>>,
    item_transforms: Vec<Transform>,

    backdrops: Vec<BackdropSpan>,

    filters: Vec<FilterSpan>,
}

impl RenderList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.clip_stack.clear();
        self.opacity_stack.clear();
        self.transform_stack.clear();
        self.item_clips.clear();
        self.item_transforms.clear();
        self.backdrops.clear();
        self.filters.clear();
    }

    pub fn push_opacity(&mut self, opacity: f32) {
        let o = if opacity.is_finite() {
            opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.opacity_stack.push(o);
    }

    pub fn pop_opacity(&mut self) {
        self.opacity_stack.pop();
    }

    pub fn opacity_depth(&self) -> usize {
        self.opacity_stack.len()
    }

    pub fn truncate_opacity(&mut self, depth: usize) {
        while self.opacity_stack.len() > depth {
            self.opacity_stack.pop();
        }
    }

    pub fn active_opacity(&self) -> f32 {
        self.opacity_stack.iter().copied().fold(1.0, |a, b| a * b)
    }

    pub fn push_transform(&mut self, t: Transform) {
        let active = self.active_transform().then(t);
        self.transform_stack.push(active);
    }

    pub fn pop_transform(&mut self) {
        self.transform_stack.pop();
    }

    pub fn transform_depth(&self) -> usize {
        self.transform_stack.len()
    }

    pub fn truncate_transforms(&mut self, depth: usize) {
        while self.transform_stack.len() > depth {
            self.transform_stack.pop();
        }
    }

    pub fn active_transform(&self) -> Transform {
        self.transform_stack.last().copied().unwrap_or_default()
    }

    pub fn item_transform(&self, index: usize) -> Transform {
        self.item_transforms.get(index).copied().unwrap_or_default()
    }

    pub fn item_transforms(&self) -> &[Transform] {
        &self.item_transforms
    }

    fn modulated(&self, color: Color) -> Color {
        let o = self.active_opacity();
        if o >= 1.0 {
            color
        } else {
            Color {
                a: color.a * o,
                ..color
            }
        }
    }

    pub fn push_clip(&mut self, rect: Rect) {
        self.clip_stack.push(ClipEntry {
            clip: RoundedClip::new(rect, 0.0),
            under: self.active_transform(),
        });
    }

    pub fn push_rounded_clip(&mut self, rect: Rect, radius: f32) {
        self.clip_stack.push(ClipEntry {
            clip: RoundedClip::new(rect, radius),
            under: self.active_transform(),
        });
    }

    pub fn clip_depth(&self) -> usize {
        self.clip_stack.len()
    }

    pub fn active_clip(&self) -> Option<Rect> {
        let mut acc: Option<Rect> = None;
        for c in &self.clip_stack {
            acc = Some(match acc {
                Some(a) => a.intersect(c.clip.rect),
                None => c.clip.rect,
            });
        }
        acc
    }

    pub fn active_clip_for(&self, t_item: Transform) -> Option<Rect> {
        if self.clip_stack.is_empty() {
            return None;
        }
        let inv = t_item.inverse()?;
        let mut acc: Option<Rect> = None;
        for c in &self.clip_stack {
            let mapped = c.under.then(inv).apply_rect_aabb(c.clip.rect);
            acc = Some(match acc {
                Some(a) => a.intersect(mapped),
                None => mapped,
            });
        }
        acc
    }

    pub fn active_rounded_clip(&self) -> Option<RoundedClip> {
        self.clip_stack
            .iter()
            .rev()
            .find(|c| c.clip.is_rounded())
            .map(|c| c.clip)
    }

    pub fn item_rounded_clip(&self, index: usize) -> Option<RoundedClip> {
        self.item_clips.get(index).copied().flatten()
    }

    pub fn truncate_clips(&mut self, depth: usize) {
        while self.clip_stack.len() > depth {
            self.clip_stack.pop();
        }
    }

    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    fn clip_rect(&self, mut r: Rect) -> Option<Rect> {
        for c in &self.clip_stack {
            r = r.intersect(c.clip.rect);
            if r.is_empty() {
                return None;
            }
        }
        Some(r)
    }

    pub fn push_quad(&mut self, rect: Rect, color: Color, radius: f32) {
        self.push_bordered_quad(rect, color, radius, 0.0, Color::TRANSPARENT);
    }

    pub fn push_bordered_quad(
        &mut self,
        rect: Rect,
        color: Color,
        radius: f32,
        border_width: f32,
        border_color: Color,
    ) {
        let t = self.active_transform();

        if !t.is_identity() {
            if let Some(local) = self.active_clip_for(t)
                && !rect.intersects(local)
            {
                return;
            }
            let color = self.modulated(color);
            let border_color = self.modulated(border_color);
            let has_border = border_width > 0.0 && border_color.a > 0.0;
            if rect.is_empty() || (color.a <= 0.0 && !has_border) {
                return;
            }
            let rounded = self.active_rounded_clip();
            self.items.push(RenderItem::Quad(Quad {
                rect,
                color,
                radius,
                border_color,
                border_width: if has_border { border_width } else { 0.0 },
            }));
            self.item_clips.push(rounded);
            self.item_transforms.push(t);
            return;
        }
        let Some(rect) = self.clip_rect(rect) else {
            return;
        };
        let color = self.modulated(color);
        let border_color = self.modulated(border_color);
        let has_border = border_width > 0.0 && border_color.a > 0.0;
        if rect.is_empty() || (color.a <= 0.0 && !has_border) {
            return;
        }
        let rounded = self.active_rounded_clip();
        self.items.push(RenderItem::Quad(Quad {
            rect,
            color,
            radius,
            border_color,
            border_width: if has_border { border_width } else { 0.0 },
        }));
        self.item_clips.push(rounded);
        self.item_transforms.push(t);
    }

    pub fn push_shadow(&mut self, shadow: Shadow) {
        let t = self.active_transform();
        let shadow = Shadow {
            color: self.modulated(shadow.color),
            ..shadow
        };
        if !shadow.is_visible() {
            return;
        }
        if !t.is_identity() {
            if let Some(local) = self.active_clip_for(t)
                && !shadow.bounds().intersects(local)
            {
                return;
            }
            let radius = self.active_rounded_clip().map(|c| c.radius).unwrap_or(0.0);
            let clip = self
                .active_clip()
                .map(|clip_rect| RoundedClip::new(clip_rect, radius));
            self.items.push(RenderItem::Shadow(shadow));
            self.item_clips.push(clip);
            self.item_transforms.push(t);
            return;
        }
        let Some(clip_rect) = self.active_clip() else {
            self.items.push(RenderItem::Shadow(shadow));
            self.item_clips.push(None);
            self.item_transforms.push(t);
            return;
        };
        if clip_rect.is_empty() {
            return;
        }

        if !shadow.bounds().intersects(clip_rect) {
            return;
        }

        let radius = self.active_rounded_clip().map(|c| c.radius).unwrap_or(0.0);
        self.items.push(RenderItem::Shadow(shadow));
        self.item_clips
            .push(Some(RoundedClip::new(clip_rect, radius)));
        self.item_transforms.push(self.active_transform());
    }

    pub fn push_text(&mut self, mut draw: TextDraw) {
        let t = self.active_transform();
        draw.color = self.modulated(draw.color);
        if draw.color.a <= 0.0 {
            return;
        }
        if !t.is_identity() {
            if let Some(local) = self.active_clip_for(t) {
                let keep = draw.glyphs.iter().any(|g| g.rect.intersects(local));
                if !keep {
                    return;
                }
            }
            draw.glyphs.retain(|g| !g.rect.is_empty());
            if !draw.glyphs.is_empty() {
                let rounded = self.active_rounded_clip();
                self.items.push(RenderItem::Text(draw));
                self.item_clips.push(rounded);
                self.item_transforms.push(t);
            }
            return;
        }
        draw.glyphs.retain_mut(|g| {
            let Some(cr) = self.clip_rect(g.rect) else {
                return false;
            };
            g.uv = clip_uv(g.rect, g.uv, cr);
            g.rect = cr;
            true
        });
        if !draw.glyphs.is_empty() {
            let rounded = self.active_rounded_clip();
            self.items.push(RenderItem::Text(draw));
            self.item_clips.push(rounded);
            self.item_transforms.push(t);
        }
    }

    pub fn push_image(&mut self, rect: Rect, uv: [f32; 4], source: ImageSource) {
        let t = self.active_transform();
        if !t.is_identity() {
            if let Some(local) = self.active_clip_for(t)
                && !rect.intersects(local)
            {
                return;
            }
            if rect.is_empty() {
                return;
            }
            let rounded = self.active_rounded_clip();
            self.items
                .push(RenderItem::Image(ImageDraw { rect, uv, source }));
            self.item_clips.push(rounded);
            self.item_transforms.push(t);
            return;
        }
        let Some(cr) = self.clip_rect(rect) else {
            return;
        };
        if cr.is_empty() {
            return;
        }
        let uv = clip_uv(rect, uv, cr);
        let rounded = self.active_rounded_clip();
        self.items.push(RenderItem::Image(ImageDraw {
            rect: cr,
            uv,
            source,
        }));
        self.item_clips.push(rounded);
        self.item_transforms.push(self.active_transform());
    }

    pub fn push_waveform(&mut self, bars: &[Rect], color: Color) {
        let t = self.active_transform();
        let color = self.modulated(color);
        if bars.is_empty() || color.a <= 0.0 {
            return;
        }
        if !t.is_identity() {
            if let Some(local) = self.active_clip_for(t) {
                let keep = bars.iter().any(|r| !r.is_empty() && r.intersects(local));
                if !keep {
                    return;
                }
            }
            let clipped: Vec<Rect> = bars
                .iter()
                .copied()
                .filter(|r| !r.is_empty())
                .take(MAX_WAVEFORM_BARS)
                .collect();
            if clipped.is_empty() {
                return;
            }
            let rounded = self.active_rounded_clip();
            self.items.push(RenderItem::Waveform(WaveformDraw {
                bars: clipped,
                color,
            }));
            self.item_clips.push(rounded);
            self.item_transforms.push(t);
            return;
        }
        let clipped: Vec<Rect> = match self.active_clip() {
            None => bars
                .iter()
                .copied()
                .filter(|r| !r.is_empty())
                .take(MAX_WAVEFORM_BARS)
                .collect(),
            Some(clip) if clip.is_empty() => return,
            Some(clip) => bars
                .iter()
                .filter_map(|r| {
                    let c = r.intersect(clip);
                    if c.is_empty() { None } else { Some(c) }
                })
                .take(MAX_WAVEFORM_BARS)
                .collect(),
        };
        if clipped.is_empty() {
            return;
        }
        let rounded = self.active_rounded_clip();
        self.items.push(RenderItem::Waveform(WaveformDraw {
            bars: clipped,
            color,
        }));
        self.item_clips.push(rounded);
        self.item_transforms.push(self.active_transform());
    }

    pub fn items(&self) -> &[RenderItem] {
        &self.items
    }

    pub fn push_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        kind: crate::brush::GradientKind,
        stops: crate::brush::GradientStops,
    ) {
        use crate::brush::GradientStop;
        if rect.is_empty() || stops.is_empty() {
            return;
        }
        let t = self.active_transform();

        let opacity = self.active_opacity();
        let stops = if opacity >= 1.0 {
            stops
        } else {
            let modulated: Vec<GradientStop> = stops
                .stops()
                .iter()
                .map(|s| {
                    GradientStop::new(
                        s.at,
                        Color {
                            a: s.color.a * opacity,
                            ..s.color
                        },
                    )
                })
                .collect();
            crate::brush::GradientStops::new(modulated)
        };

        if !t.is_identity() {
            if let Some(local) = self.active_clip_for(t)
                && !rect.intersects(local)
            {
                return;
            }
        } else {
            let Some(clipped) = self.clip_rect(rect) else {
                return;
            };
            if clipped.is_empty() {
                return;
            }

            return self.push_gradient_inner(clipped, radius, kind, stops, t);
        }
        self.push_gradient_inner(rect, radius, kind, stops, t)
    }

    fn push_gradient_inner(
        &mut self,
        rect: Rect,
        radius: f32,
        kind: crate::brush::GradientKind,
        stops: crate::brush::GradientStops,
        t: Transform,
    ) {
        if rect.is_empty() || stops.is_empty() {
            return;
        }

        if stops.stops().iter().all(|s| s.color.a <= 0.0) {
            return;
        }
        let radius = if radius.is_finite() {
            radius
                .clamp(0.0, rect.size.width.min(rect.size.height) * 0.5)
                .max(0.0)
        } else {
            0.0
        };
        let rounded = self.active_rounded_clip();
        self.items.push(RenderItem::Gradient(GradientDraw {
            rect,
            radius,
            kind,
            stops,
        }));
        self.item_clips.push(rounded);
        self.item_transforms.push(t);
    }

    pub fn push_backdrop(
        &mut self,
        rect: Rect,
        radius: f32,
        blur: f32,
        dim: Color,
        noise: f32,
        mask: SpanMask,
    ) {
        let blur = if blur.is_finite() { blur.max(0.0) } else { 0.0 };
        let noise = if noise.is_finite() {
            noise.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let opacity = self.active_opacity();
        let dim = Color {
            a: dim.a * opacity,
            ..dim
        };
        if !(blur > 0.0 || dim.a > 0.0 || noise > 0.0) {
            return;
        }
        let Some(rect) = self.clip_rect(rect) else {
            return;
        };
        if rect.is_empty() {
            return;
        }
        let radius = if radius.is_finite() {
            radius.max(0.0)
        } else {
            0.0
        };
        let draw = BackdropDraw {
            rect,
            radius,
            blur,
            dim,
            noise: noise * opacity,
            mask,
        };
        if !draw.is_visible() {
            return;
        }
        self.backdrops.push(BackdropSpan {
            at: self.items.len(),
            draw,
            clip: self.active_rounded_clip(),
        });
    }

    pub fn backdrops(&self) -> &[BackdropSpan] {
        &self.backdrops
    }

    pub fn has_backdrops(&self) -> bool {
        !self.backdrops.is_empty()
    }

    pub fn push_filter_begin(
        &mut self,
        rect: Rect,
        radius: f32,
        blur: f32,
        mask: SpanMask,
    ) -> Option<usize> {
        let blur = if blur.is_finite() { blur.max(0.0) } else { 0.0 };
        if !(blur > 0.0) {
            return None;
        }
        let rect = self.clip_rect(rect)?;
        if rect.is_empty() {
            return None;
        }
        let radius = if radius.is_finite() {
            radius.max(0.0)
        } else {
            0.0
        };
        let draw = FilterDraw {
            rect,
            radius,
            blur,
            mask,
        };
        if !draw.is_visible() {
            return None;
        }
        let start = self.items.len();
        self.filters.push(FilterSpan {
            start,
            end: start,
            draw,
            clip: self.active_rounded_clip(),
        });
        Some(self.filters.len() - 1)
    }

    pub fn push_filter_end(&mut self, handle: usize) {
        let end = self.items.len();
        let Some(span) = self.filters.get_mut(handle) else {
            return;
        };
        span.end = end;
        if span.end <= span.start {
            self.filters.remove(handle);
        }
    }

    pub fn filter_spans(&self) -> &[FilterSpan] {
        &self.filters
    }

    pub fn has_filters(&self) -> bool {
        !self.filters.is_empty()
    }

    pub fn item_clips(&self) -> &[Option<RoundedClip>] {
        &self.item_clips
    }

    pub fn quads(&self) -> impl Iterator<Item = &Quad> {
        self.items.iter().filter_map(|i| match i {
            RenderItem::Quad(q) => Some(q),
            RenderItem::Text(_)
            | RenderItem::Image(_)
            | RenderItem::Shadow(_)
            | RenderItem::Waveform(_)
            | RenderItem::Gradient(_) => None,
        })
    }

    pub fn images(&self) -> impl Iterator<Item = &ImageDraw> {
        self.items.iter().filter_map(|i| match i {
            RenderItem::Image(d) => Some(d),
            RenderItem::Quad(_)
            | RenderItem::Text(_)
            | RenderItem::Shadow(_)
            | RenderItem::Waveform(_)
            | RenderItem::Gradient(_) => None,
        })
    }

    pub fn shadows(&self) -> impl Iterator<Item = &Shadow> {
        self.items.iter().filter_map(|i| match i {
            RenderItem::Shadow(s) => Some(s),
            RenderItem::Quad(_)
            | RenderItem::Text(_)
            | RenderItem::Image(_)
            | RenderItem::Waveform(_)
            | RenderItem::Gradient(_) => None,
        })
    }

    pub fn waveforms(&self) -> impl Iterator<Item = &WaveformDraw> {
        self.items.iter().filter_map(|i| match i {
            RenderItem::Waveform(w) => Some(w),
            RenderItem::Quad(_)
            | RenderItem::Text(_)
            | RenderItem::Image(_)
            | RenderItem::Shadow(_)
            | RenderItem::Gradient(_) => None,
        })
    }

    pub fn gradients(&self) -> impl Iterator<Item = &GradientDraw> {
        self.items.iter().filter_map(|i| match i {
            RenderItem::Gradient(g) => Some(g),
            RenderItem::Quad(_)
            | RenderItem::Text(_)
            | RenderItem::Image(_)
            | RenderItem::Shadow(_)
            | RenderItem::Waveform(_) => None,
        })
    }

    pub fn gradient_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i, RenderItem::Gradient(_)))
            .count()
    }

    pub fn waveform_bar_count(&self) -> usize {
        self.items
            .iter()
            .map(|i| match i {
                RenderItem::Waveform(w) => w.bars.len(),
                _ => 0,
            })
            .sum()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_construction() {
        assert_eq!(Color::rgb8(255, 0, 0), Color::RED);
        assert_eq!(Color::rgb(0.0, 1.0, 0.0).a, 1.0);
        assert_eq!(Color::TRANSPARENT.a, 0.0);
    }

    #[test]
    fn hsl_roundtrips_through_rgb() {
        for (r, g, b) in [
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (112, 143, 234),
            (185, 185, 185),
        ] {
            let c = Color::rgb8(r, g, b);
            let (h, s, l) = c.to_hsl();
            let back = Color::from_hsl(h, s, l).to_rgb8();
            assert!(
                (r as i16 - back.0 as i16).abs() <= 1
                    && (g as i16 - back.1 as i16).abs() <= 1
                    && (b as i16 - back.2 as i16).abs() <= 1,
                "#{r:02X}{g:02X}{b:02X} -> ({h:.1},{s:.3},{l:.3}) -> #{:02X}{:02X}{:02X}",
                back.0,
                back.1,
                back.2,
            );
        }
        assert_eq!(Color::from_hsl(0.0, 0.0, 0.5).to_rgb8(), (128, 128, 128));

        assert_eq!(
            Color::from_hsl(720.0, 1.0, 0.5),
            Color::from_hsl(0.0, 1.0, 0.5)
        );
        assert_eq!(Color::from_hsl(0.0, 0.5, 0.5).to_hex6(), "BF4040");
    }

    #[test]
    fn hex6_string_is_uppercase_without_hash() {
        assert_eq!(Color::rgb8(0x70, 0x8F, 0xEA).to_hex6(), "708FEA");
    }

    #[test]
    fn hex_number_forms() {
        assert_eq!(Color::hex(0x0FA4FA), Color::rgb8(15, 164, 250));
        assert_eq!(Color::hex(0xFF0000), Color::RED);
        assert_eq!(Color::hex(0x000000), Color::BLACK);

        assert_eq!(Color::hex(0x1_FF0000), Color::RED);
        let half = Color::hexa(0x0FA4FA80);
        assert_eq!(half.r, Color::hex(0x0FA4FA).r);
        assert_eq!(half.a, 128.0 / 255.0);
        assert_eq!(Color::hexa(0xFFFFFFFF).a, 1.0);
    }

    #[test]
    fn parse_hex_forms() {
        assert_eq!(Color::parse_hex("#0FA4FA"), Some(Color::rgb8(15, 164, 250)));
        assert_eq!(Color::parse_hex("0fa4fa"), Some(Color::rgb8(15, 164, 250)));
        assert_eq!(
            Color::parse_hex("0x0FA4FA"),
            Some(Color::rgb8(15, 164, 250))
        );
        assert_eq!(Color::parse_hex("#fff"), Some(Color::WHITE));
        assert_eq!(Color::parse_hex("#0000"), Some(Color::TRANSPARENT));
        assert_eq!(Color::parse_hex("#0FA4FA80"), Some(Color::hexa(0x0FA4FA80)));

        assert_eq!(Color::parse_hex("#0af"), Color::parse_hex("#00aaff"));
    }

    #[test]
    fn parse_hex_rejects_garbage() {
        assert_eq!(Color::parse_hex(""), None);
        assert_eq!(Color::parse_hex("#"), None);
        assert_eq!(Color::parse_hex("bogus"), None);
        assert_eq!(Color::parse_hex("#12"), None);
        assert_eq!(Color::parse_hex("#12345"), None);
        assert_eq!(Color::parse_hex("#123456789"), None);
        assert_eq!(Color::parse_hex("#gggggg"), None);
        assert_eq!(Color::parse_hex("  #fff"), None, "no trimming");
    }

    #[test]
    fn clip_balance_helpers() {
        let mut list = RenderList::new();
        assert_eq!(list.clip_depth(), 0);
        list.push_clip(Rect::from_xywh(0.0, 0.0, 10.0, 10.0));
        list.push_clip(Rect::from_xywh(0.0, 0.0, 5.0, 5.0));
        assert_eq!(list.clip_depth(), 2);
        list.truncate_clips(1);
        assert_eq!(list.clip_depth(), 1);
        list.truncate_clips(0);
        assert_eq!(list.clip_depth(), 0);
    }

    #[test]
    fn opacity_stack_modulates_quad_alpha() {
        let mut list = RenderList::new();
        assert_eq!(list.active_opacity(), 1.0);
        list.push_opacity(0.5);
        list.push_opacity(0.5);
        assert_eq!(list.active_opacity(), 0.25, "nestings multiply");
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        let q = list.quads().next().expect("quad po push");
        assert!(
            (q.color.a - 0.25).abs() < 1e-6,
            "alfa 1 * 0.5 * 0.5: {}",
            q.color.a
        );
        assert_eq!(q.color.r, 1.0, "rgb untouched");
        list.pop_opacity();
        list.push_quad(Rect::from_xywh(20.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        let qs: Vec<_> = list.quads().collect();
        assert!((qs[1].color.a - 0.5).abs() < 1e-6);

        list.push_opacity(0.0);
        list.push_quad(Rect::from_xywh(40.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        assert_eq!(list.quads().count(), 2, "quad przy opacity 0 nie wchodzi");
        list.truncate_opacity(0);
        assert_eq!(list.opacity_depth(), 0);
        assert_eq!(list.active_opacity(), 1.0);

        list.push_opacity(f32::NAN);
        list.push_quad(Rect::from_xywh(60.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        let qs: Vec<_> = list.quads().collect();
        assert_eq!(qs[2].color.a, 1.0);
        list.clear();
        assert_eq!(list.opacity_depth(), 0, "clear also resets opacity");
    }

    #[test]
    fn list_push_and_clear() {
        let mut list = RenderList::new();
        assert!(list.is_empty());
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        assert_eq!(list.len(), 1);
        list.push_quad(Rect::ZERO, Color::RED, 0.0);
        list.push_quad(
            Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
            Color::TRANSPARENT,
            0.0,
        );
        assert_eq!(list.len(), 1, "empty-value filters");

        list.push_text(TextDraw::default());
        assert_eq!(list.len(), 1, "empty TextDraw skipped");

        list.push_text(TextDraw {
            color: Color::WHITE,
            glyphs: vec![GlyphQuad {
                rect: Rect::ZERO,
                uv: [0.0; 4],
                color_glyph: false,
            }],
        });
        assert_eq!(list.len(), 2);
        assert_eq!(list.quads().count(), 1);

        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn bordered_quad_filters_and_flags() {
        let r = Rect::from_xywh(0.0, 0.0, 100.0, 60.0);
        let mut list = RenderList::new();

        list.push_quad(r, Color::BLUE, 8.0);
        let q = list.quads().next().unwrap();
        assert!(!q.has_border());
        assert_eq!(q.border_width, 0.0);

        let bg = Color::rgba(154.0 / 255.0, 214.0 / 255.0, 248.0 / 255.0, 24.0 / 255.0);
        list.push_bordered_quad(r, bg, 8.0, 1.0, Color::RED);
        let qs: Vec<_> = list.quads().collect();
        assert_eq!(qs.len(), 2, "background + border is one quad, not two");
        assert_eq!(qs[1].color, bg);
        assert_eq!(qs[1].border_color, Color::RED);
        assert_eq!(qs[1].border_width, 1.0);
        assert!(qs[1].has_border());

        let mut list = RenderList::new();
        list.push_bordered_quad(r, Color::TRANSPARENT, 8.0, 2.0, Color::GREEN);
        assert_eq!(list.len(), 1);

        let mut list = RenderList::new();
        list.push_bordered_quad(r, Color::TRANSPARENT, 0.0, 0.0, Color::RED);
        list.push_bordered_quad(r, Color::TRANSPARENT, 0.0, 2.0, Color::TRANSPARENT);
        list.push_bordered_quad(Rect::ZERO, Color::RED, 0.0, 1.0, Color::RED);
        assert!(list.is_empty());
    }

    #[test]
    fn clip_intersects_and_filters() {
        let mut list = RenderList::new();
        list.push_clip(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        list.push_quad(Rect::from_xywh(50.0, 50.0, 100.0, 100.0), Color::RED, 0.0);
        list.push_quad(Rect::from_xywh(200.0, 200.0, 10.0, 10.0), Color::RED, 0.0);
        assert_eq!(list.len(), 1);
        let q = list.quads().next().unwrap();
        assert_eq!(q.rect, Rect::from_xywh(50.0, 50.0, 50.0, 50.0));

        list.pop_clip();
        list.push_quad(Rect::from_xywh(200.0, 200.0, 10.0, 10.0), Color::RED, 0.0);
        assert_eq!(list.len(), 2, "unclipped after pop_clip");
    }

    #[test]
    fn nested_clip_intersects() {
        let mut list = RenderList::new();
        list.push_clip(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        list.push_clip(Rect::from_xywh(50.0, 50.0, 100.0, 100.0));
        list.push_quad(Rect::from_xywh(0.0, 0.0, 200.0, 200.0), Color::RED, 0.0);
        assert_eq!(list.len(), 1);
        let q = list.quads().next().unwrap();
        assert_eq!(q.rect, Rect::from_xywh(50.0, 50.0, 50.0, 50.0));
    }

    #[test]
    fn rounded_clip_stored_per_item_for_shader() {
        let mut list = RenderList::new();
        let card = Rect::from_xywh(0.0, 0.0, 240.0, 280.0);
        list.push_rounded_clip(card, 8.0);
        list.push_quad(Rect::from_xywh(0.0, 0.0, 240.0, 20.0), Color::GREEN, 0.0);
        assert_eq!(list.len(), 1);
        let clip = list.item_rounded_clip(0).expect("rounded clip stored");
        assert_eq!(clip.rect, card);
        assert_eq!(clip.radius, 8.0);

        let mut list = RenderList::new();
        list.push_clip(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        list.push_quad(Rect::from_xywh(10.0, 10.0, 20.0, 20.0), Color::RED, 0.0);
        assert_eq!(list.item_rounded_clip(0), None);
    }

    #[test]
    fn rounded_clip_survives_rectangular_child_clip() {
        let mut list = RenderList::new();
        list.push_rounded_clip(Rect::from_xywh(0.0, 0.0, 240.0, 280.0), 8.0);
        list.push_clip(Rect::from_xywh(0.0, 0.0, 240.0, 280.0));
        list.push_quad(Rect::from_xywh(0.0, 0.0, 240.0, 20.0), Color::GREEN, 0.0);
        let clip = list.item_rounded_clip(0).expect("outer rounding kept");
        assert_eq!(clip.radius, 8.0);
        list.pop_clip();
        list.pop_clip();
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        assert_eq!(list.item_rounded_clip(1), None, "no clip after pops");
    }

    fn glyph(x: f32, y: f32, w: f32, h: f32) -> GlyphQuad {
        GlyphQuad {
            rect: Rect::from_xywh(x, y, w, h),
            uv: [0.0, 0.0, 1.0, 1.0],
            color_glyph: false,
        }
    }

    fn pushed_glyphs(clip: Rect, g: GlyphQuad) -> Vec<GlyphQuad> {
        let mut list = RenderList::new();
        list.push_clip(clip);
        list.push_text(TextDraw {
            color: Color::WHITE,
            glyphs: vec![g],
        });
        match list.items() {
            [RenderItem::Text(t)] => t.glyphs.clone(),
            _ => panic!("expected a single TextDraw"),
        }
    }

    #[test]
    fn clipped_glyph_keeps_proportional_uv() {
        let out = pushed_glyphs(
            Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            glyph(90.0, 10.0, 20.0, 10.0),
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rect, Rect::from_xywh(90.0, 10.0, 10.0, 10.0));
        assert_eq!(out[0].uv, [0.0, 0.0, 0.5, 1.0]);
    }

    #[test]
    fn fully_visible_glyph_keeps_uv() {
        let out = pushed_glyphs(
            Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            glyph(10.0, 10.0, 20.0, 10.0),
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rect, Rect::from_xywh(10.0, 10.0, 20.0, 10.0));
        assert_eq!(out[0].uv, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn clipped_glyph_uv_scales_with_atlas_subrect() {
        let mut g = glyph(0.0, 0.0, 20.0, 20.0);
        g.uv = [0.25, 0.5, 0.75, 1.0];
        let out = pushed_glyphs(Rect::from_xywh(10.0, 0.0, 100.0, 100.0), g);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rect, Rect::from_xywh(10.0, 0.0, 10.0, 20.0));
        assert_eq!(out[0].uv, [0.5, 0.5, 0.75, 1.0]);
    }

    fn file_image() -> ImageSource {
        ImageSource::File("foto.png".into())
    }

    #[test]
    fn image_push_clip_and_uv() {
        let mut list = RenderList::new();
        list.push_clip(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        list.push_image(
            Rect::from_xywh(90.0, 10.0, 20.0, 10.0),
            [0.0, 0.0, 1.0, 1.0],
            file_image(),
        );
        list.push_image(
            Rect::from_xywh(200.0, 200.0, 10.0, 10.0),
            [0.0, 0.0, 1.0, 1.0],
            file_image(),
        );
        let images: Vec<_> = list.images().collect();
        assert_eq!(images.len(), 1, "outside the clip is dropped");
        assert_eq!(images[0].rect, Rect::from_xywh(90.0, 10.0, 10.0, 10.0));
        assert_eq!(
            images[0].uv,
            [0.0, 0.0, 0.5, 1.0],
            "proportional UV, as for glyphs"
        );
    }

    #[test]
    fn image_cache_keys() {
        assert_eq!(file_image().cache_key(), file_image().cache_key());
        assert_ne!(
            file_image().cache_key(),
            ImageSource::File("inne.png".into()).cache_key()
        );
        let rgba = |id| ImageSource::Rgba {
            id,
            width: 2,
            height: 2,
            data: Rc::from(vec![0u8; 16].into_boxed_slice()),
        };
        assert_eq!(rgba(7).cache_key(), 7);
        assert_ne!(rgba(7).cache_key(), rgba(8).cache_key());
    }

    fn shadow_spec() -> Shadow {
        Shadow {
            rect: Rect::from_xywh(10.0, 10.0, 100.0, 60.0),
            color: Color::BLACK,
            radius: 6.0,
            blur: 8.0,
            spread: 0.0,
            offset: Vec2::new(0.0, 2.0),
            inset: false,
        }
    }

    #[test]
    fn shadow_push_filters_invisible_shadows() {
        let mut list = RenderList::new();
        list.push_shadow(Shadow {
            blur: 0.0,
            ..shadow_spec()
        });
        list.push_shadow(Shadow {
            blur: f32::NAN,
            ..shadow_spec()
        });
        list.push_shadow(Shadow {
            color: Color::TRANSPARENT,
            ..shadow_spec()
        });
        list.push_shadow(Shadow {
            rect: Rect::ZERO,
            ..shadow_spec()
        });
        assert!(list.is_empty(), "hidden shadows create no items");
    }

    #[test]
    fn shadow_keeps_shape_rect_and_rides_the_clip() {
        let mut list = RenderList::new();
        list.push_clip(Rect::from_xywh(0.0, 0.0, 50.0, 50.0));
        list.push_rounded_clip(Rect::from_xywh(0.0, 0.0, 200.0, 200.0), 12.0);
        list.push_shadow(shadow_spec());
        assert_eq!(list.len(), 1);
        assert_eq!(list.shadows().next().unwrap().rect, shadow_spec().rect);
        let clip = list.item_rounded_clip(0).expect("ancestor clip");
        assert_eq!(
            clip.rect,
            Rect::from_xywh(0.0, 0.0, 50.0, 50.0),
            "przeciecie stosu clipow"
        );
        assert_eq!(clip.radius, 12.0, "rounding from inner clip");
    }

    #[test]
    fn shadow_dropped_when_fully_clipped_out() {
        let mut list = RenderList::new();
        list.push_clip(Rect::from_xywh(500.0, 500.0, 10.0, 10.0));
        list.push_shadow(shadow_spec());
        assert!(list.is_empty(), "fully outside clip");
    }

    #[test]
    fn shadow_bounds_grow_with_blur_spread_and_offset() {
        let s = Shadow {
            blur: 10.0,
            spread: 4.0,
            offset: Vec2::new(2.0, -3.0),
            ..shadow_spec()
        };
        let b = s.bounds();
        assert!((b.left() - 3.0).abs() < 1e-4, "{b:?}");
        assert!((b.top() - (-2.0)).abs() < 1e-4, "{b:?}");
        assert!((b.size.width - 118.0).abs() < 1e-4, "{b:?}");
        let inset = Shadow { inset: true, ..s };
        assert_eq!(inset.bounds(), inset.rect, "inset stays inside the shape");
    }

    #[test]
    fn waveform_push_is_one_item_not_n() {
        let mut list = RenderList::new();
        let bars: Vec<Rect> = (0..100)
            .map(|i| Rect::from_xywh(i as f32 * 2.0, 0.0, 1.5, 20.0))
            .collect();
        list.push_waveform(&bars, Color::WHITE);
        assert_eq!(list.len(), 1, "whole waveform is 1 item");
        assert_eq!(list.waveform_bar_count(), 100);
        assert_eq!(list.waveforms().next().unwrap().bars.len(), 100);
    }

    #[test]
    fn waveform_push_clips_and_filters() {
        let mut list = RenderList::new();
        list.push_clip(Rect::from_xywh(0.0, 0.0, 10.0, 10.0));
        let bars = vec![
            Rect::from_xywh(0.0, 0.0, 5.0, 5.0),
            Rect::from_xywh(200.0, 200.0, 5.0, 5.0),
            Rect::ZERO,
        ];
        list.push_waveform(&bars, Color::WHITE);
        assert_eq!(list.len(), 1);
        assert_eq!(list.waveform_bar_count(), 1, "outside clip dropped");

        let mut list = RenderList::new();
        list.push_waveform(&[], Color::WHITE);
        list.push_waveform(&[Rect::from_xywh(0.0, 0.0, 5.0, 5.0)], Color::TRANSPARENT);
        assert!(list.is_empty());
    }

    #[test]
    fn dynamic_image_cache_key_is_id_and_versioned() {
        let a = ImageSource::Dynamic {
            id: 7,
            width: 4,
            height: 1,
            data: Rc::from(vec![0u8; 16].into_boxed_slice()),
            version: 1,
        };
        let b = ImageSource::Dynamic {
            id: 7,
            width: 4,
            height: 1,
            data: Rc::from(vec![255u8; 16].into_boxed_slice()),
            version: 2,
        };
        assert_eq!(a.cache_key(), 7);
        assert_eq!(a.cache_key(), b.cache_key(), "same texture slot");
        assert_ne!(a, b, "other version means other data");
        assert_eq!(a.dynamic_version(), Some(1));
        assert_eq!(
            ImageSource::Rgba {
                id: 7,
                width: 1,
                height: 1,
                data: Rc::from(vec![0u8; 4].into_boxed_slice()),
            }
            .dynamic_version(),
            None
        );
    }

    #[test]
    fn backdrop_side_channel_keeps_item_indices() {
        let mut list = RenderList::new();
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        list.push_quad(Rect::from_xywh(10.0, 0.0, 10.0, 10.0), Color::GREEN, 0.0);
        assert!(!list.has_backdrops());
        list.push_backdrop(
            Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            8.0,
            16.0,
            Color::TRANSPARENT,
            0.0,
            SpanMask::None,
        );
        assert!(list.has_backdrops());
        assert_eq!(list.len(), 2, "marker does not add items");
        let span = list.backdrops()[0];
        assert_eq!(span.at, 2, "rozmywa dwa quady spod spodu");
        assert_eq!(span.draw.rect, Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        assert_eq!(span.draw.blur, 16.0);
        list.push_quad(Rect::from_xywh(0.0, 0.0, 5.0, 5.0), Color::BLUE, 0.0);
        assert_eq!(list.backdrops()[0].at, 2, "at frozen in push");
    }

    #[test]
    fn filter_span_covers_subtree_range() {
        let mut list = RenderList::new();
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        let h = list
            .push_filter_begin(
                Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
                8.0,
                16.0,
                SpanMask::None,
            )
            .expect("widoczny filtr otwiera zakres");
        assert_eq!(list.len(), 1, "begin does not add items");
        list.push_quad(Rect::from_xywh(0.0, 0.0, 5.0, 5.0), Color::BLUE, 0.0);
        list.push_quad(Rect::from_xywh(5.0, 5.0, 5.0, 5.0), Color::GREEN, 0.0);
        list.push_filter_end(h);
        assert!(list.has_filters());
        let span = list.filter_spans()[0];
        assert_eq!((span.start, span.end), (1, 3), "tylko poddrzewo");
        assert_eq!(span.draw.blur, 16.0);
        assert_eq!(span.draw.radius, 8.0);
        list.clear();
        assert!(!list.has_filters(), "clear also drops filters");
    }

    #[test]
    fn filter_begin_rejects_invisible_and_end_drops_empty() {
        let mut list = RenderList::new();

        assert!(
            list.push_filter_begin(
                Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
                0.0,
                0.0,
                SpanMask::None
            )
            .is_none()
        );
        assert!(
            list.push_filter_begin(
                Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
                0.0,
                f32::NAN,
                SpanMask::None
            )
            .is_none()
        );
        assert!(
            list.push_filter_begin(
                Rect::from_xywh(0.0, 0.0, 0.0, 0.0),
                0.0,
                8.0,
                SpanMask::None
            )
            .is_none()
        );
        assert!(!list.has_filters());

        let h = list
            .push_filter_begin(
                Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
                0.0,
                8.0,
                SpanMask::None,
            )
            .expect("otwiera");
        list.push_filter_end(h);
        assert!(!list.has_filters(), "pusty zakres bez contentu");

        let mut clipped = RenderList::new();
        clipped.push_clip(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        let h = clipped
            .push_filter_begin(
                Rect::from_xywh(50.0, 50.0, 100.0, 100.0),
                4.0,
                12.0,
                SpanMask::None,
            )
            .expect("otwiera");
        clipped.push_quad(Rect::from_xywh(60.0, 60.0, 10.0, 10.0), Color::RED, 0.0);
        clipped.push_filter_end(h);
        assert_eq!(
            clipped.filter_spans()[0].draw.rect,
            Rect::from_xywh(50.0, 50.0, 50.0, 50.0)
        );
    }

    #[test]
    fn filter_spans_nest_like_subtrees() {
        let mut list = RenderList::new();
        let outer = list
            .push_filter_begin(
                Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
                0.0,
                8.0,
                SpanMask::None,
            )
            .expect("outer");
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        let inner = list
            .push_filter_begin(
                Rect::from_xywh(0.0, 0.0, 50.0, 50.0),
                0.0,
                4.0,
                SpanMask::None,
            )
            .expect("inner");
        list.push_quad(Rect::from_xywh(0.0, 0.0, 5.0, 5.0), Color::BLUE, 0.0);
        list.push_filter_end(inner);
        list.push_filter_end(outer);
        assert_eq!(list.filter_spans().len(), 2);
        assert_eq!(
            (list.filter_spans()[0].start, list.filter_spans()[0].end),
            (0, 2)
        );
        assert_eq!(
            (list.filter_spans()[1].start, list.filter_spans()[1].end),
            (1, 2)
        );
    }

    #[test]
    fn backdrop_filters_and_clips() {
        let mut list = RenderList::new();

        list.push_backdrop(
            Rect::from_xywh(0.0, 0.0, 50.0, 50.0),
            0.0,
            0.0,
            Color::TRANSPARENT,
            0.0,
            SpanMask::None,
        );
        assert!(!list.has_backdrops());

        list.push_backdrop(
            Rect::from_xywh(0.0, 0.0, 50.0, 50.0),
            0.0,
            0.0,
            Color::BLACK.with_alpha(0.25),
            0.0,
            SpanMask::None,
        );
        assert_eq!(list.backdrops().len(), 1);

        let mut clipped = RenderList::new();
        clipped.push_clip(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        clipped.push_backdrop(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0),
            4.0,
            12.0,
            Color::TRANSPARENT,
            0.0,
            SpanMask::None,
        );
        assert_eq!(
            clipped.backdrops()[0].draw.rect,
            Rect::from_xywh(50.0, 50.0, 50.0, 50.0)
        );

        let mut outside = RenderList::new();
        outside.push_clip(Rect::from_xywh(500.0, 500.0, 10.0, 10.0));
        outside.push_backdrop(
            Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
            0.0,
            12.0,
            Color::TRANSPARENT,
            0.0,
            SpanMask::None,
        );
        assert!(!outside.has_backdrops());

        let mut faded = RenderList::new();
        faded.push_opacity(0.5);
        faded.push_backdrop(
            Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
            0.0,
            0.0,
            Color::BLACK.with_alpha(0.4),
            0.5,
            SpanMask::None,
        );
        let d = faded.backdrops()[0].draw;
        assert!((d.dim.a - 0.2).abs() < 1e-6, "dim.a 0.4 * 0.5: {}", d.dim.a);
        assert!(
            (d.noise - 0.25).abs() < 1e-6,
            "noise 0.5 * 0.5: {}",
            d.noise
        );

        faded.clear();
        assert!(!faded.has_backdrops());
    }

    #[test]
    fn span_mask_encodes_for_gpu() {
        assert_eq!(SpanMask::None.mask_vec(), [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            SpanMask::Linear {
                from: 1.0,
                to: 0.0,
                angle_rad: std::f32::consts::FRAC_PI_2,
            }
            .mask_vec(),
            [1.0, 1.0, 0.0, std::f32::consts::FRAC_PI_2]
        );
        assert_eq!(SpanMask::Uniform(0.5).mask_vec(), [2.0, 0.5, 0.0, 0.0]);

        assert_eq!(
            SpanMask::Linear {
                from: 2.0,
                to: -1.0,
                angle_rad: f32::NAN,
            }
            .mask_vec()[1..3],
            [1.0, 0.0]
        );
        assert!(SpanMask::None.is_none());
        assert!(!SpanMask::Uniform(0.0).is_none());
    }

    #[test]
    fn filter_span_carries_mask() {
        let mut list = RenderList::new();
        let mask = SpanMask::Linear {
            from: 1.0,
            to: 0.0,
            angle_rad: std::f32::consts::FRAC_PI_2,
        };
        let h = list
            .push_filter_begin(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), 0.0, 8.0, mask)
            .expect("otwiera");
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        list.push_filter_end(h);
        assert_eq!(list.filter_spans()[0].draw.mask, mask);
        assert_eq!(
            list.filter_spans()[0].draw.mask.mask_vec(),
            [1.0, 1.0, 0.0, std::f32::consts::FRAC_PI_2]
        );
    }

    #[test]
    fn backdrop_marker_carries_mask() {
        let mut list = RenderList::new();
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        list.push_backdrop(
            Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            0.0,
            8.0,
            Color::TRANSPARENT,
            0.0,
            SpanMask::Uniform(0.5),
        );
        assert_eq!(
            list.backdrops()[0].draw.mask,
            SpanMask::Uniform(0.5),
            "maska w markerze t�'a"
        );
    }

    #[test]
    fn transform_stack_composes_and_snapshots() {
        use crate::geometry::Point;
        let mut list = RenderList::new();
        assert!(list.active_transform().is_identity());

        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        assert!(list.item_transform(0).is_identity());

        list.push_transform(Transform::rotate_around_degrees(90.0, Point::new(5.0, 5.0)));
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        assert!(!list.item_transform(1).is_identity());
        assert_eq!(list.item_transforms().len(), list.items().len());
        list.pop_transform();
        assert!(list.active_transform().is_identity());
        list.push_quad(Rect::from_xywh(0.0, 0.0, 10.0, 10.0), Color::RED, 0.0);
        assert!(list.item_transform(2).is_identity());
    }

    #[test]
    fn transform_aabb_covers_rotation() {
        use crate::geometry::Point;
        let r = Rect::from_xywh(0.0, 0.0, 10.0, 20.0);
        let t = Transform::rotate_around_degrees(90.0, Point::new(5.0, 10.0));
        let aabb = t.apply_rect_aabb(r);
        assert!(
            (aabb.size.width - 20.0).abs() < 0.01,
            "w={}",
            aabb.size.width
        );
        assert!(
            (aabb.size.height - 10.0).abs() < 0.01,
            "h={}",
            aabb.size.height
        );
        assert!(Transform::IDENTITY.apply_rect_aabb(r) == r);
    }

    #[test]
    fn rotated_text_survives_own_self_clip() {
        let tile = Rect::from_xywh(0.0, 0.0, 60.0, 40.0);
        let mut list = RenderList::new();
        list.push_transform(Transform::rotate_around_degrees(45.0, tile.center()));
        list.push_clip(tile);
        list.push_text(TextDraw {
            color: Color::WHITE,
            glyphs: vec![GlyphQuad {
                rect: Rect::from_xywh(4.0, 4.0, 12.0, 10.0),
                uv: [0.0, 0.0, 1.0, 1.0],
                color_glyph: false,
            }],
        });
        assert_eq!(list.len(), 1, "rotated glyph in custom clip remains");
    }

    #[test]
    fn ancestor_clip_still_culls_rotated_content() {
        let tile = Rect::from_xywh(0.0, 0.0, 60.0, 40.0);
        let mut list = RenderList::new();
        list.push_clip(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        list.push_transform(Transform::rotate_around_degrees(45.0, tile.center()));
        list.push_text(TextDraw {
            color: Color::WHITE,
            glyphs: vec![GlyphQuad {
                rect: Rect::from_xywh(4.0, 4.0, 12.0, 10.0),
                uv: [0.0, 0.0, 1.0, 1.0],
                color_glyph: false,
            }],
        });

        assert_eq!(list.len(), 1, "clip przodka nie zjada widocznego");

        let mut list2 = RenderList::new();
        list2.push_clip(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        list2.push_transform(Transform::translate(500.0, 500.0));
        list2.push_text(TextDraw {
            color: Color::WHITE,
            glyphs: vec![GlyphQuad {
                rect: Rect::from_xywh(4.0, 4.0, 12.0, 10.0),
                uv: [0.0, 0.0, 1.0, 1.0],
                color_glyph: false,
            }],
        });
        assert_eq!(list2.len(), 0, "clip przodka obcina poza ekranem");
    }
}
