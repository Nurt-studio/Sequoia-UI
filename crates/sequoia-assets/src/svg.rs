#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SvgPoint {
    pub x: f32,

    pub y: f32,
}

impl SvgPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FillRule {
    #[default]
    NonZero,

    EvenOdd,
}

impl FillRule {
    fn parse(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("evenodd") {
            Self::EvenOdd
        } else {
            Self::NonZero
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Seg {
    Line {
        to: SvgPoint,
    },

    Quad {
        ctrl: SvgPoint,

        to: SvgPoint,
    },

    Cubic {
        c1: SvgPoint,

        c2: SvgPoint,

        to: SvgPoint,
    },

    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubPath {
    pub start: SvgPoint,

    pub segs: Vec<Seg>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum StrokeLineCap {
    #[default]
    Butt,

    Round,

    Square,
}

impl StrokeLineCap {
    fn parse(value: &str) -> Option<Self> {
        if value.trim().eq_ignore_ascii_case("round") {
            Some(Self::Round)
        } else if value.trim().eq_ignore_ascii_case("square") {
            Some(Self::Square)
        } else if value.trim().eq_ignore_ascii_case("butt") {
            Some(Self::Butt)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum StrokeLineJoin {
    #[default]
    Miter,

    Round,

    Bevel,
}

impl StrokeLineJoin {
    fn parse(value: &str) -> Option<Self> {
        if value.trim().eq_ignore_ascii_case("round") || value.trim().eq_ignore_ascii_case("arcs") {
            Some(Self::Round)
        } else if value.trim().eq_ignore_ascii_case("bevel")
            || value.trim().eq_ignore_ascii_case("miter-clip")
        {
            Some(Self::Bevel)
        } else if value.trim().eq_ignore_ascii_case("miter") {
            Some(Self::Miter)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub width: f32,

    pub cap: StrokeLineCap,

    pub join: StrokeLineJoin,

    pub miter_limit: f32,

    pub dash_array: Vec<f32>,

    pub dash_offset: f32,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            width: 1.0,
            cap: StrokeLineCap::default(),
            join: StrokeLineJoin::default(),
            miter_limit: 4.0,
            dash_array: Vec::new(),
            dash_offset: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub subpaths: Vec<SubPath>,

    pub fill_rule: FillRule,

    pub fill: bool,

    pub stroke: Option<Stroke>,
}

impl Shape {
    pub fn filled(subpaths: Vec<SubPath>, fill_rule: FillRule) -> Self {
        Self {
            subpaths,
            fill_rule,
            fill: true,
            stroke: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SvgIcon {
    pub viewbox: [f32; 4],

    pub shapes: Vec<Shape>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SvgError {
    MissingViewBox,

    BadViewBox,

    BadPath(usize, String),
}

impl std::fmt::Display for SvgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingViewBox => write!(f, "missing viewBox in <svg>"),
            Self::BadViewBox => write!(f, "viewBox needs 4 numbers"),
            Self::BadPath(pos, msg) => write!(f, "path error at {pos}: {msg}"),
        }
    }
}

impl std::error::Error for SvgError {}

pub fn parse_svg(src: &str) -> Result<SvgIcon, SvgError> {
    let mut parser = Parser { src, i: 0 };
    parser.run()
}

enum Tok {
    Open {
        name: String,
        attrs: Attrs,
        self_closing: bool,
    },

    Close(String),
}

#[derive(Default)]
struct Attrs(Vec<(String, String)>);

impl Attrs {
    fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn len(&self, name: &str) -> Option<f32> {
        self.get(name).and_then(parse_len)
    }

    fn f32(&self, name: &str) -> Option<f32> {
        self.len(name)
    }

    fn own_fill_rule(&self) -> Option<FillRule> {
        if let Some(v) = self.get("fill-rule") {
            return Some(FillRule::parse(v));
        }
        style_decl(self.get("style"), "fill-rule").map(FillRule::parse)
    }

    fn own_none_fill(&self) -> Option<bool> {
        if let Some(f) = self.get("fill") {
            return Some(f.trim().eq_ignore_ascii_case("none"));
        }
        style_decl(self.get("style"), "fill").map(|v| v.trim().eq_ignore_ascii_case("none"))
    }

    fn stroke_prop(&self, name: &str) -> Option<&str> {
        if let Some(v) = self.get(name) {
            return Some(v);
        }
        style_decl(self.get("style"), name)
    }

    fn own_stroke_paint(&self) -> Option<bool> {
        let v = self.stroke_prop("stroke")?;
        let t = v.trim();
        if t.is_empty() {
            return None;
        }
        Some(!t.eq_ignore_ascii_case("none"))
    }

    fn own_stroke_width(&self) -> Option<f32> {
        self.stroke_prop("stroke-width").and_then(parse_len)
    }

    fn own_linecap(&self) -> Option<StrokeLineCap> {
        self.stroke_prop("stroke-linecap")
            .and_then(StrokeLineCap::parse)
    }

    fn own_linejoin(&self) -> Option<StrokeLineJoin> {
        self.stroke_prop("stroke-linejoin")
            .and_then(StrokeLineJoin::parse)
    }

    fn own_miter_limit(&self) -> Option<f32> {
        let v = self.stroke_prop("stroke-miterlimit")?;
        v.trim().parse().ok()
    }

    fn own_dash_array(&self) -> Option<Vec<f32>> {
        let v = self.stroke_prop("stroke-dasharray")?;
        let t = v.trim();
        if t.is_empty() {
            return None;
        }
        if t.eq_ignore_ascii_case("none") {
            return Some(Vec::new());
        }
        let mut out = Vec::new();
        for part in t.split(|c: char| c.is_whitespace() || c == ',') {
            if part.is_empty() {
                continue;
            }

            let n = parse_len(part)?;
            if !n.is_finite() || n < 0.0 {
                return None;
            }
            out.push(n);
        }
        Some(out)
    }

    fn own_dash_offset(&self) -> Option<f32> {
        self.stroke_prop("stroke-dashoffset").and_then(parse_len)
    }

    fn own_nonscaling_stroke(&self) -> bool {
        for src in [
            self.get("vector-effect"),
            style_decl(self.get("style"), "vector-effect"),
        ] {
            if let Some(v) = src
                && v.to_ascii_lowercase().contains("non-scaling-stroke")
            {
                return true;
            }
        }
        false
    }

    fn own_transform(&self) -> Transform {
        self.get("transform")
            .map(parse_transform)
            .unwrap_or_default()
    }
}

fn parse_len(raw: &str) -> Option<f32> {
    let s = raw.trim();
    if s.is_empty() || s.ends_with('%') {
        return None;
    }

    let mut end = s.len();
    while end > 0 && s.as_bytes()[end - 1].is_ascii_alphabetic() {
        end -= 1;
    }
    s[..end].trim().parse().ok()
}

fn style_decl<'a>(style: Option<&'a str>, prop: &str) -> Option<&'a str> {
    let style = style?;
    for decl in style.split(';') {
        let mut parts = decl.splitn(2, ':');
        let (Some(k), Some(v)) = (parts.next(), parts.next()) else {
            continue;
        };
        if k.trim().eq_ignore_ascii_case(prop) {
            return Some(v.trim());
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Transform {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    const fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    fn is_identity(self) -> bool {
        self == Self::identity()
    }

    fn then(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    fn scale_factor(self) -> f32 {
        let sx = (self.a * self.a + self.b * self.b).sqrt();
        let sy = (self.c * self.c + self.d * self.d).sqrt();
        let s = 0.5 * (sx + sy);
        if s.is_finite() { s } else { 1.0 }
    }

    fn apply(self, p: SvgPoint) -> SvgPoint {
        SvgPoint::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    fn apply_subpath(self, sp: &SubPath) -> SubPath {
        if self.is_identity() {
            return sp.clone();
        }
        SubPath {
            start: self.apply(sp.start),
            segs: sp
                .segs
                .iter()
                .map(|seg| match *seg {
                    Seg::Line { to } => Seg::Line { to: self.apply(to) },
                    Seg::Quad { ctrl, to } => Seg::Quad {
                        ctrl: self.apply(ctrl),
                        to: self.apply(to),
                    },
                    Seg::Cubic { c1, c2, to } => Seg::Cubic {
                        c1: self.apply(c1),
                        c2: self.apply(c2),
                        to: self.apply(to),
                    },
                    Seg::Close => Seg::Close,
                })
                .collect(),
        }
    }
}

fn parse_transform(raw: &str) -> Transform {
    let mut acc = Transform::identity();
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
            i += 1;
        }
        if start == i {
            break;
        }
        let name = &raw[start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if bytes.get(i) != Some(&b'(') {
            continue;
        }
        i += 1;
        let close = raw[i..].find(')').map(|o| i + o).unwrap_or(raw.len());
        let nums: Vec<f32> = raw[i..close]
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        i = (close + 1).min(raw.len());
        let local = match name {
            "translate" => {
                let (tx, ty) = match nums.as_slice() {
                    [x, y] => (*x, *y),
                    [x] => (*x, 0.0),
                    _ => continue,
                };
                Transform {
                    a: 1.0,
                    b: 0.0,
                    c: 0.0,
                    d: 1.0,
                    e: tx,
                    f: ty,
                }
            }
            "scale" => {
                let (sx, sy) = match nums.as_slice() {
                    [x, y] => (*x, *y),
                    [x] => (*x, *x),
                    _ => continue,
                };
                Transform {
                    a: sx,
                    b: 0.0,
                    c: 0.0,
                    d: sy,
                    e: 0.0,
                    f: 0.0,
                }
            }
            "rotate" => {
                let (deg, cx, cy) = match nums.as_slice() {
                    [a, x, y] => (*a, *x, *y),
                    [a] => (*a, 0.0, 0.0),
                    _ => continue,
                };
                let (s, c) = deg.to_radians().sin_cos();
                let r = Transform {
                    a: c,
                    b: s,
                    c: -s,
                    d: c,
                    e: 0.0,
                    f: 0.0,
                };
                if cx == 0.0 && cy == 0.0 {
                    r
                } else {
                    let to = Transform {
                        a: 1.0,
                        b: 0.0,
                        c: 0.0,
                        d: 1.0,
                        e: cx,
                        f: cy,
                    };
                    let from = Transform {
                        a: 1.0,
                        b: 0.0,
                        c: 0.0,
                        d: 1.0,
                        e: -cx,
                        f: -cy,
                    };
                    to.then(r).then(from)
                }
            }
            "skewX" => {
                let [a] = nums.as_slice() else { continue };
                Transform {
                    a: 1.0,
                    b: 0.0,
                    c: a.to_radians().tan(),
                    d: 1.0,
                    e: 0.0,
                    f: 0.0,
                }
            }
            "skewY" => {
                let [a] = nums.as_slice() else { continue };
                Transform {
                    a: 1.0,
                    b: a.to_radians().tan(),
                    c: 0.0,
                    d: 1.0,
                    e: 0.0,
                    f: 0.0,
                }
            }
            "matrix" => {
                let [a, b, c, d, e, f] = nums.as_slice() else {
                    continue;
                };
                Transform {
                    a: *a,
                    b: *b,
                    c: *c,
                    d: *d,
                    e: *e,
                    f: *f,
                }
            }
            _ => continue,
        };
        acc = acc.then(local);
    }
    acc
}

fn next_token(src: &str, i: &mut usize) -> Option<Tok> {
    loop {
        let off = src[*i..].find('<')?;
        *i += off;
        let rest = &src[*i..];
        if rest.starts_with("<!--") {
            let end = rest.find("-->")? + 3;
            *i += end;
            continue;
        }
        if rest.starts_with("<?") || rest.starts_with("<!") {
            let end = rest.find('>')? + 1;
            *i += end;
            continue;
        }
        if let Some(inner) = rest.strip_prefix("</") {
            let end = inner.find('>')?;
            let name = inner[..end].trim().to_ascii_lowercase();
            *i += 2 + end + 1;
            return Some(Tok::Close(name));
        }
        let inner = &rest[1..];
        let name_end = inner
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(inner.len());
        let name = inner[..name_end].to_ascii_lowercase();
        *i += 1 + name_end;
        let (attrs, self_closing) = parse_attrs(src, i)?;
        return Some(Tok::Open {
            name,
            attrs,
            self_closing,
        });
    }
}

fn parse_attrs(src: &str, i: &mut usize) -> Option<(Attrs, bool)> {
    let mut attrs = Attrs::default();
    let bytes = src.as_bytes();
    loop {
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        match bytes.get(*i) {
            Some(b'>') => {
                *i += 1;
                return Some((attrs, false));
            }
            Some(b'/') if bytes.get(*i + 1) == Some(&b'>') => {
                *i += 2;
                return Some((attrs, true));
            }

            Some(b'/') => return None,
            None => return Some((attrs, false)),
            _ => {}
        }

        let start = *i;
        while *i < bytes.len()
            && !bytes[*i].is_ascii_whitespace()
            && bytes[*i] != b'='
            && bytes[*i] != b'>'
            && bytes[*i] != b'/'
        {
            *i += 1;
        }
        let name = src[start..*i].to_ascii_lowercase();
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if bytes.get(*i) != Some(&b'=') {
            if name.is_empty() {
                return None;
            }
            continue;
        }
        *i += 1;
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        let quote = match bytes.get(*i) {
            Some(b'"') | Some(b'\'') => bytes[*i],
            _ => 0,
        };
        let value = if quote != 0 {
            let start = *i + 1;
            let end = src[start..]
                .find(quote as char)
                .map(|o| start + o)
                .unwrap_or(bytes.len());
            let v = src[start..end.min(bytes.len())].to_string();
            *i = (end + 1).min(bytes.len());
            v
        } else {
            let start = *i;
            while *i < bytes.len() && !bytes[*i].is_ascii_whitespace() && bytes[*i] != b'>' {
                *i += 1;
            }
            src[start..*i].to_string()
        };
        attrs.0.push((name, value));
    }
}

fn skip_element(src: &str, i: &mut usize, name: &str) {
    let mut depth = 1usize;
    while let Some(tok) = next_token(src, i) {
        match tok {
            Tok::Open {
                name: n,
                self_closing: false,
                ..
            } if n == name => depth += 1,
            Tok::Open {
                self_closing: true, ..
            } => {}
            Tok::Close(n) if n == name => {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
            _ => {}
        }
    }
}

struct Parser<'a> {
    src: &'a str,
    i: usize,
}

struct ResolvedPaint {
    fill_rule: FillRule,
    has_fill: bool,
    stroke: Option<Stroke>,
    tx: Transform,
}

impl Parser<'_> {
    #[allow(clippy::too_many_arguments)]
    fn resolve_paint(
        attrs: &Attrs,
        fill_stack: &[FillRule],
        none_stack: &[bool],
        stroke_paint: &[bool],
        stroke_width: &[f32],
        stroke_cap: &[StrokeLineCap],
        stroke_join: &[StrokeLineJoin],
        stroke_miter: &[f32],
        stroke_dash: &[Vec<f32>],
        stroke_offset: &[f32],
        tx_stack: &[Transform],
    ) -> ResolvedPaint {
        let fill_rule = attrs
            .own_fill_rule()
            .unwrap_or(fill_stack.last().copied().unwrap_or_default());
        let none = attrs
            .own_none_fill()
            .unwrap_or(none_stack.last().copied().unwrap_or(false));
        let has_fill = !none;

        let paint = attrs
            .own_stroke_paint()
            .unwrap_or(stroke_paint.last().copied().unwrap_or(false));
        let local_width = attrs
            .own_stroke_width()
            .filter(|v| v.is_finite())
            .unwrap_or(stroke_width.last().copied().unwrap_or(1.0));
        let cap = attrs
            .own_linecap()
            .unwrap_or(stroke_cap.last().copied().unwrap_or_default());
        let join = attrs
            .own_linejoin()
            .unwrap_or(stroke_join.last().copied().unwrap_or_default());
        let miter = attrs
            .own_miter_limit()
            .filter(|v| v.is_finite())
            .unwrap_or(stroke_miter.last().copied().unwrap_or(4.0));
        let dash = attrs
            .own_dash_array()
            .unwrap_or_else(|| stroke_dash.last().cloned().unwrap_or_default());
        let offset = attrs
            .own_dash_offset()
            .filter(|v| v.is_finite())
            .unwrap_or(stroke_offset.last().copied().unwrap_or(0.0));

        let tx = tx_stack
            .last()
            .copied()
            .unwrap_or_else(Transform::identity)
            .then(attrs.own_transform());
        let stroke = if paint && local_width.is_finite() && local_width > 0.0 {
            let scale = if attrs.own_nonscaling_stroke() {
                1.0
            } else {
                tx.scale_factor()
            };
            let scale = if scale.is_finite() && scale > 0.0 {
                scale
            } else {
                1.0
            };
            let width = local_width * scale;
            if width.is_finite() && width > 0.0 {
                Some(Stroke {
                    width,
                    cap,
                    join,
                    miter_limit: miter.clamp(1.0, 100.0),
                    dash_array: dash.into_iter().map(|v| v * scale).collect(),
                    dash_offset: offset * scale,
                })
            } else {
                None
            }
        } else {
            None
        };

        ResolvedPaint {
            fill_rule,
            has_fill,
            stroke,
            tx,
        }
    }

    fn push_group(
        attrs: &Attrs,
        fill_stack: &mut Vec<FillRule>,
        none_stack: &mut Vec<bool>,
        stroke_paint: &mut Vec<bool>,
        stroke_width: &mut Vec<f32>,
        stroke_cap: &mut Vec<StrokeLineCap>,
        stroke_join: &mut Vec<StrokeLineJoin>,
        stroke_miter: &mut Vec<f32>,
        stroke_dash: &mut Vec<Vec<f32>>,
        stroke_offset: &mut Vec<f32>,
        tx_stack: &mut Vec<Transform>,
    ) {
        let rule = attrs
            .own_fill_rule()
            .unwrap_or(fill_stack.last().copied().unwrap_or_default());
        let none = attrs
            .own_none_fill()
            .unwrap_or(none_stack.last().copied().unwrap_or(false));
        let paint = attrs
            .own_stroke_paint()
            .unwrap_or(stroke_paint.last().copied().unwrap_or(false));
        let width = attrs
            .own_stroke_width()
            .filter(|v| v.is_finite())
            .unwrap_or(stroke_width.last().copied().unwrap_or(1.0));
        let cap = attrs
            .own_linecap()
            .unwrap_or(stroke_cap.last().copied().unwrap_or_default());
        let join = attrs
            .own_linejoin()
            .unwrap_or(stroke_join.last().copied().unwrap_or_default());
        let miter = attrs
            .own_miter_limit()
            .filter(|v| v.is_finite())
            .unwrap_or(stroke_miter.last().copied().unwrap_or(4.0));
        let dash = attrs
            .own_dash_array()
            .unwrap_or_else(|| stroke_dash.last().cloned().unwrap_or_default());
        let offset = attrs
            .own_dash_offset()
            .filter(|v| v.is_finite())
            .unwrap_or(stroke_offset.last().copied().unwrap_or(0.0));
        fill_stack.push(rule);
        none_stack.push(none);
        stroke_paint.push(paint);
        stroke_width.push(width);
        stroke_cap.push(cap);
        stroke_join.push(join);
        stroke_miter.push(miter);
        stroke_dash.push(dash);
        stroke_offset.push(offset);
        let parent_tx = tx_stack.last().copied().unwrap_or_else(Transform::identity);
        tx_stack.push(parent_tx.then(attrs.own_transform()));
    }

    fn pop_group(
        fill_stack: &mut Vec<FillRule>,
        none_stack: &mut Vec<bool>,
        stroke_paint: &mut Vec<bool>,
        stroke_width: &mut Vec<f32>,
        stroke_cap: &mut Vec<StrokeLineCap>,
        stroke_join: &mut Vec<StrokeLineJoin>,
        stroke_miter: &mut Vec<f32>,
        stroke_dash: &mut Vec<Vec<f32>>,
        stroke_offset: &mut Vec<f32>,
        tx_stack: &mut Vec<Transform>,
    ) {
        if fill_stack.len() > 1 {
            fill_stack.pop();
        }
        if none_stack.len() > 1 {
            none_stack.pop();
        }
        if stroke_paint.len() > 1 {
            stroke_paint.pop();
        }
        if stroke_width.len() > 1 {
            stroke_width.pop();
        }
        if stroke_cap.len() > 1 {
            stroke_cap.pop();
        }
        if stroke_join.len() > 1 {
            stroke_join.pop();
        }
        if stroke_miter.len() > 1 {
            stroke_miter.pop();
        }
        if stroke_dash.len() > 1 {
            stroke_dash.pop();
        }
        if stroke_offset.len() > 1 {
            stroke_offset.pop();
        }
        if tx_stack.len() > 1 {
            tx_stack.pop();
        }
    }

    fn run(&mut self) -> Result<SvgIcon, SvgError> {
        let mut viewbox: Option<[f32; 4]> = None;
        let mut shapes: Vec<Shape> = Vec::new();

        let mut fill_stack: Vec<FillRule> = vec![FillRule::default()];
        let mut none_stack: Vec<bool> = vec![false];
        let mut stroke_paint: Vec<bool> = vec![false];
        let mut stroke_width: Vec<f32> = vec![1.0];
        let mut stroke_cap: Vec<StrokeLineCap> = vec![StrokeLineCap::default()];
        let mut stroke_join: Vec<StrokeLineJoin> = vec![StrokeLineJoin::default()];
        let mut stroke_miter: Vec<f32> = vec![4.0];
        let mut stroke_dash: Vec<Vec<f32>> = vec![Vec::new()];
        let mut stroke_offset: Vec<f32> = vec![0.0];
        let mut tx_stack: Vec<Transform> = vec![Transform::identity()];

        while let Some(tok) = next_token(self.src, &mut self.i) {
            let Tok::Open {
                name,
                attrs,
                self_closing,
            } = tok
            else {
                if name_close_is_group(&tok) {
                    Self::pop_group(
                        &mut fill_stack,
                        &mut none_stack,
                        &mut stroke_paint,
                        &mut stroke_width,
                        &mut stroke_cap,
                        &mut stroke_join,
                        &mut stroke_miter,
                        &mut stroke_dash,
                        &mut stroke_offset,
                        &mut tx_stack,
                    );
                }
                continue;
            };
            match name.as_str() {
                "svg" => {
                    if viewbox.is_none() {
                        viewbox = parse_viewbox(&attrs);
                    }
                    if !self_closing {
                        Self::push_group(
                            &attrs,
                            &mut fill_stack,
                            &mut none_stack,
                            &mut stroke_paint,
                            &mut stroke_width,
                            &mut stroke_cap,
                            &mut stroke_join,
                            &mut stroke_miter,
                            &mut stroke_dash,
                            &mut stroke_offset,
                            &mut tx_stack,
                        );
                    }
                }
                "g" => {
                    if !self_closing {
                        Self::push_group(
                            &attrs,
                            &mut fill_stack,
                            &mut none_stack,
                            &mut stroke_paint,
                            &mut stroke_width,
                            &mut stroke_cap,
                            &mut stroke_join,
                            &mut stroke_miter,
                            &mut stroke_dash,
                            &mut stroke_offset,
                            &mut tx_stack,
                        );
                    }
                }
                "path" => {
                    let paint = Self::resolve_paint(
                        &attrs,
                        &fill_stack,
                        &none_stack,
                        &stroke_paint,
                        &stroke_width,
                        &stroke_cap,
                        &stroke_join,
                        &stroke_miter,
                        &stroke_dash,
                        &stroke_offset,
                        &tx_stack,
                    );
                    if (paint.has_fill || paint.stroke.is_some())
                        && let Some(d) = attrs.get("d")
                    {
                        let subpaths =
                            parse_path_data(d).map_err(|(pos, msg)| SvgError::BadPath(pos, msg))?;
                        if !subpaths.is_empty() {
                            let subpaths: Vec<SubPath> = subpaths
                                .iter()
                                .map(|sp| paint.tx.apply_subpath(sp))
                                .collect();
                            shapes.push(Shape {
                                subpaths,
                                fill_rule: paint.fill_rule,
                                fill: paint.has_fill,
                                stroke: paint.stroke,
                            });
                        }
                    }
                    if !self_closing {
                        skip_element(self.src, &mut self.i, "path");
                    }
                }
                "rect" => {
                    let paint = Self::resolve_paint(
                        &attrs,
                        &fill_stack,
                        &none_stack,
                        &stroke_paint,
                        &stroke_width,
                        &stroke_cap,
                        &stroke_join,
                        &stroke_miter,
                        &stroke_dash,
                        &stroke_offset,
                        &tx_stack,
                    );
                    if (paint.has_fill || paint.stroke.is_some())
                        && let Some(shape) = shape_rect(&attrs)
                    {
                        shapes.push(Shape {
                            subpaths: vec![paint.tx.apply_subpath(&shape)],
                            fill_rule: paint.fill_rule,
                            fill: paint.has_fill,
                            stroke: paint.stroke,
                        });
                    }
                    if !self_closing {
                        skip_element(self.src, &mut self.i, "rect");
                    }
                }
                "circle" | "ellipse" => {
                    let paint = Self::resolve_paint(
                        &attrs,
                        &fill_stack,
                        &none_stack,
                        &stroke_paint,
                        &stroke_width,
                        &stroke_cap,
                        &stroke_join,
                        &stroke_miter,
                        &stroke_dash,
                        &stroke_offset,
                        &tx_stack,
                    );
                    if (paint.has_fill || paint.stroke.is_some())
                        && let Some(shape) = shape_ellipse(&attrs)
                    {
                        shapes.push(Shape {
                            subpaths: vec![paint.tx.apply_subpath(&shape)],
                            fill_rule: paint.fill_rule,
                            fill: paint.has_fill,
                            stroke: paint.stroke,
                        });
                    }
                    if !self_closing {
                        skip_element(self.src, &mut self.i, &name);
                    }
                }
                "line" => {
                    let paint = Self::resolve_paint(
                        &attrs,
                        &fill_stack,
                        &none_stack,
                        &stroke_paint,
                        &stroke_width,
                        &stroke_cap,
                        &stroke_join,
                        &stroke_miter,
                        &stroke_dash,
                        &stroke_offset,
                        &tx_stack,
                    );
                    if (paint.has_fill || paint.stroke.is_some())
                        && let Some(shape) = shape_line(&attrs)
                    {
                        shapes.push(Shape {
                            subpaths: vec![paint.tx.apply_subpath(&shape)],
                            fill_rule: paint.fill_rule,
                            fill: paint.has_fill,
                            stroke: paint.stroke,
                        });
                    }
                    if !self_closing {
                        skip_element(self.src, &mut self.i, &name);
                    }
                }
                "polyline" | "polygon" => {
                    let paint = Self::resolve_paint(
                        &attrs,
                        &fill_stack,
                        &none_stack,
                        &stroke_paint,
                        &stroke_width,
                        &stroke_cap,
                        &stroke_join,
                        &stroke_miter,
                        &stroke_dash,
                        &stroke_offset,
                        &tx_stack,
                    );
                    if paint.has_fill || paint.stroke.is_some() {
                        let close = name == "polygon";
                        if let Some(shape) = shape_points(&attrs, close) {
                            shapes.push(Shape {
                                subpaths: vec![paint.tx.apply_subpath(&shape)],
                                fill_rule: paint.fill_rule,
                                fill: paint.has_fill,
                                stroke: paint.stroke,
                            });
                        }
                    }
                    if !self_closing {
                        skip_element(self.src, &mut self.i, &name);
                    }
                }
                _ => {
                    if !self_closing {
                        skip_element(self.src, &mut self.i, &name);
                    }
                }
            }
        }

        let vb = viewbox.ok_or(SvgError::MissingViewBox)?;

        Ok(SvgIcon {
            viewbox: vb,
            shapes,
        })
    }
}

fn name_close_is_group(tok: &Tok) -> bool {
    matches!(tok, Tok::Close(n) if n == "g" || n == "svg")
}

fn parse_viewbox(attrs: &Attrs) -> Option<[f32; 4]> {
    if let Some(vb) = attrs.get("viewbox") {
        let nums: Vec<f32> = vb
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        return (nums.len() == 4).then_some([nums[0], nums[1], nums[2], nums[3]]);
    }
    let w = attrs.f32("width")?;
    let h = attrs.f32("height")?;
    Some([0.0, 0.0, w, h])
}

fn quarter_arc(c: SvgPoint, rx: f32, ry: f32, a0: f32, a1: f32) -> Seg {
    let k = 4.0 / 3.0 * ((a1 - a0) / 4.0).tan();
    let p0 = SvgPoint::new(c.x + rx * a0.cos(), c.y + ry * a0.sin());
    let p1 = SvgPoint::new(c.x + rx * a1.cos(), c.y + ry * a1.sin());
    let d0 = SvgPoint::new(-rx * a0.sin(), ry * a0.cos());
    let d1 = SvgPoint::new(-rx * a1.sin(), ry * a1.cos());
    Seg::Cubic {
        c1: SvgPoint::new(p0.x + k * d0.x, p0.y + k * d0.y),
        c2: SvgPoint::new(p1.x - k * d1.x, p1.y - k * d1.y),
        to: p1,
    }
}

fn shape_rect(attrs: &Attrs) -> Option<SubPath> {
    let x = attrs.f32("x").unwrap_or(0.0);
    let y = attrs.f32("y").unwrap_or(0.0);
    let w = attrs.f32("width")?;
    let h = attrs.f32("height")?;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let rx = attrs.f32("rx").unwrap_or(0.0).min(w / 2.0).max(0.0);
    let ry = attrs.f32("ry").unwrap_or(rx).min(h / 2.0).max(0.0);
    let mut sub = SubPath {
        start: SvgPoint::new(x + rx, y),
        segs: Vec::new(),
    };
    if rx <= 0.0 || ry <= 0.0 {
        sub.segs.push(Seg::Line {
            to: SvgPoint::new(x + w, y),
        });
        sub.segs.push(Seg::Line {
            to: SvgPoint::new(x + w, y + h),
        });
        sub.segs.push(Seg::Line {
            to: SvgPoint::new(x, y + h),
        });
    } else {
        sub.segs.push(Seg::Line {
            to: SvgPoint::new(x + w - rx, y),
        });
        sub.segs.push(quarter_arc(
            SvgPoint::new(x + w - rx, y + ry),
            rx,
            ry,
            -std::f32::consts::FRAC_PI_2,
            0.0,
        ));
        sub.segs.push(Seg::Line {
            to: SvgPoint::new(x + w, y + h - ry),
        });
        sub.segs.push(quarter_arc(
            SvgPoint::new(x + w - rx, y + h - ry),
            rx,
            ry,
            0.0,
            std::f32::consts::FRAC_PI_2,
        ));
        sub.segs.push(Seg::Line {
            to: SvgPoint::new(x + rx, y + h),
        });
        sub.segs.push(quarter_arc(
            SvgPoint::new(x + rx, y + h - ry),
            rx,
            ry,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
        ));
        sub.segs.push(Seg::Line {
            to: SvgPoint::new(x, y + ry),
        });
        sub.segs.push(quarter_arc(
            SvgPoint::new(x + rx, y + ry),
            rx,
            ry,
            std::f32::consts::PI,
            3.0 * std::f32::consts::FRAC_PI_2,
        ));
    }
    sub.segs.push(Seg::Close);
    Some(sub)
}

fn shape_ellipse(attrs: &Attrs) -> Option<SubPath> {
    let cx = attrs.f32("cx").unwrap_or(0.0);
    let cy = attrs.f32("cy").unwrap_or(0.0);
    let (rx, ry) = match attrs.f32("r") {
        Some(r) => (r, r),
        None => (attrs.f32("rx")?, attrs.f32("ry")?),
    };
    if rx <= 0.0 || ry <= 0.0 {
        return None;
    }
    let (rx, ry) = (rx.abs(), ry.abs());
    let hpi = std::f32::consts::FRAC_PI_2;
    let mut sub = SubPath {
        start: SvgPoint::new(cx, cy - ry),
        segs: Vec::new(),
    };
    for a0 in [-hpi, 0.0, hpi, std::f32::consts::PI] {
        sub.segs
            .push(quarter_arc(SvgPoint::new(cx, cy), rx, ry, a0, a0 + hpi));
    }
    sub.segs.push(Seg::Close);
    Some(sub)
}

fn shape_line(attrs: &Attrs) -> Option<SubPath> {
    let x1 = attrs.f32("x1").unwrap_or(0.0);
    let y1 = attrs.f32("y1").unwrap_or(0.0);
    let x2 = attrs.f32("x2").unwrap_or(0.0);
    let y2 = attrs.f32("y2").unwrap_or(0.0);
    Some(SubPath {
        start: SvgPoint::new(x1, y1),
        segs: vec![Seg::Line {
            to: SvgPoint::new(x2, y2),
        }],
    })
}

fn shape_points(attrs: &Attrs, close: bool) -> Option<SubPath> {
    let raw = attrs.get("points")?;
    let nums: Vec<f32> = raw
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() < 4 {
        return None;
    }
    let mut sub = SubPath {
        start: SvgPoint::new(nums[0], nums[1]),
        segs: Vec::new(),
    };
    for pair in nums[2..].chunks_exact(2) {
        sub.segs.push(Seg::Line {
            to: SvgPoint::new(pair[0], pair[1]),
        });
    }
    if close {
        sub.segs.push(Seg::Close);
    }
    Some(sub)
}

struct PathCursor<'a> {
    s: &'a [u8],
    i: usize,
}

impl PathCursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn skip_sep(&mut self) {
        while matches!(self.peek(), Some(b) if b.is_ascii_whitespace() || b == b',') {
            self.i += 1;
        }
    }

    fn number(&mut self) -> Option<f32> {
        self.skip_sep();
        let start = self.i;
        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.i += 1;
        }
        let mut seen_digit = false;
        while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
            self.i += 1;
            seen_digit = true;
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                self.i += 1;
                seen_digit = true;
            }
        }
        if seen_digit && matches!(self.peek(), Some(b'e') | Some(b'E')) {
            let save = self.i;
            self.i += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            if matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                    self.i += 1;
                }
            } else {
                self.i = save;
            }
        }
        if !seen_digit {
            self.i = start;
            return None;
        }
        std::str::from_utf8(&self.s[start..self.i])
            .ok()?
            .parse()
            .ok()
    }

    fn flag(&mut self) -> Option<bool> {
        self.skip_sep();
        match self.peek()? {
            b'0' => {
                self.i += 1;
                Some(false)
            }
            b'1' => {
                self.i += 1;
                Some(true)
            }
            _ => None,
        }
    }

    fn pair(&mut self) -> Option<(f32, f32)> {
        let x = self.number()?;
        let y = self.number()?;
        Some((x, y))
    }
}

fn implicit_command(prev: u8) -> u8 {
    match prev {
        b'M' => b'L',
        b'm' => b'l',
        other => other,
    }
}

pub(crate) fn parse_path_data(d: &str) -> Result<Vec<SubPath>, (usize, String)> {
    let mut c = PathCursor {
        s: d.as_bytes(),
        i: 0,
    };
    let mut subpaths: Vec<SubPath> = Vec::new();
    let mut cur: Option<SubPath> = None;
    let mut pos = SvgPoint::new(0.0, 0.0);
    let mut last_cubic_ctrl: Option<SvgPoint> = None;
    let mut last_quad_ctrl: Option<SvgPoint> = None;
    let mut prev_cmd: Option<u8> = None;

    macro_rules! flush {
        () => {
            if let Some(sp) = cur.take() {
                if !sp.segs.is_empty() {
                    subpaths.push(sp);
                }
            }
        };
    }

    macro_rules! ensure_open {
        ($p:expr) => {
            if cur.is_none() {
                cur = Some(SubPath {
                    start: $p,
                    segs: Vec::new(),
                });
            }
        };
    }

    let err = |c: &PathCursor, msg: &str| (c.i, msg.to_string());

    loop {
        c.skip_sep();
        let Some(b) = c.peek() else { break };
        let cmd = if b.is_ascii_alphabetic() {
            c.i += 1;
            b
        } else {
            match prev_cmd {
                Some(p) => implicit_command(p),
                None => return Err(err(&c, "expected a path command")),
            }
        };
        let rel = cmd.is_ascii_lowercase();

        match cmd {
            b'M' | b'm' => {
                let Some((x, y)) = c.pair() else {
                    return Err(err(&c, "M needs an x y pair"));
                };
                flush!();
                pos = if rel {
                    SvgPoint::new(pos.x + x, pos.y + y)
                } else {
                    SvgPoint::new(x, y)
                };
                cur = Some(SubPath {
                    start: pos,
                    segs: Vec::new(),
                });
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
                prev_cmd = Some(cmd);
            }
            b'L' | b'l' => {
                let Some((x, y)) = c.pair() else {
                    return Err(err(&c, "L needs an x y pair"));
                };
                pos = if rel {
                    SvgPoint::new(pos.x + x, pos.y + y)
                } else {
                    SvgPoint::new(x, y)
                };
                ensure_open!(pos);
                if let Some(sp) = &mut cur {
                    sp.segs.push(Seg::Line { to: pos });
                }
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
                prev_cmd = Some(cmd);
            }
            b'H' | b'h' => {
                let Some(x) = c.number() else {
                    return Err(err(&c, "H needs x"));
                };
                pos = SvgPoint::new(if rel { pos.x + x } else { x }, pos.y);
                ensure_open!(pos);
                if let Some(sp) = &mut cur {
                    sp.segs.push(Seg::Line { to: pos });
                }
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
                prev_cmd = Some(cmd);
            }
            b'V' | b'v' => {
                let Some(y) = c.number() else {
                    return Err(err(&c, "V needs y"));
                };
                pos = SvgPoint::new(pos.x, if rel { pos.y + y } else { y });
                ensure_open!(pos);
                if let Some(sp) = &mut cur {
                    sp.segs.push(Seg::Line { to: pos });
                }
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
                prev_cmd = Some(cmd);
            }
            b'C' | b'c' => {
                let (Some((x1, y1)), Some((x2, y2)), Some((x, y))) = (c.pair(), c.pair(), c.pair())
                else {
                    return Err(err(&c, "C needs three pairs"));
                };
                let to_abs = |a: (f32, f32)| SvgPoint::new(pos.x + a.0, pos.y + a.1);
                let (c1, c2, to) = if rel {
                    (to_abs((x1, y1)), to_abs((x2, y2)), to_abs((x, y)))
                } else {
                    (
                        SvgPoint::new(x1, y1),
                        SvgPoint::new(x2, y2),
                        SvgPoint::new(x, y),
                    )
                };
                ensure_open!(pos);
                if let Some(sp) = &mut cur {
                    sp.segs.push(Seg::Cubic { c1, c2, to });
                }
                last_cubic_ctrl = Some(c2);
                last_quad_ctrl = None;
                pos = to;
                prev_cmd = Some(cmd);
            }
            b'S' | b's' => {
                let (Some((x2, y2)), Some((x, y))) = (c.pair(), c.pair()) else {
                    return Err(err(&c, "S needs two pairs"));
                };
                let c1 = reflect(last_cubic_ctrl, pos);
                let to_abs = |a: (f32, f32)| SvgPoint::new(pos.x + a.0, pos.y + a.1);
                let (c2, to) = if rel {
                    (to_abs((x2, y2)), to_abs((x, y)))
                } else {
                    (SvgPoint::new(x2, y2), SvgPoint::new(x, y))
                };
                ensure_open!(pos);
                if let Some(sp) = &mut cur {
                    sp.segs.push(Seg::Cubic { c1, c2, to });
                }
                last_cubic_ctrl = Some(c2);
                last_quad_ctrl = None;
                pos = to;
                prev_cmd = Some(cmd);
            }
            b'Q' | b'q' => {
                let (Some((x1, y1)), Some((x, y))) = (c.pair(), c.pair()) else {
                    return Err(err(&c, "Q needs two pairs"));
                };
                let to_abs = |a: (f32, f32)| SvgPoint::new(pos.x + a.0, pos.y + a.1);
                let (ctrl, to) = if rel {
                    (to_abs((x1, y1)), to_abs((x, y)))
                } else {
                    (SvgPoint::new(x1, y1), SvgPoint::new(x, y))
                };
                ensure_open!(pos);
                if let Some(sp) = &mut cur {
                    sp.segs.push(Seg::Quad { ctrl, to });
                }
                last_cubic_ctrl = None;
                last_quad_ctrl = Some(ctrl);
                pos = to;
                prev_cmd = Some(cmd);
            }
            b'T' | b't' => {
                let Some((x, y)) = c.pair() else {
                    return Err(err(&c, "T needs an x y pair"));
                };
                let ctrl = reflect(last_quad_ctrl, pos);
                let to = if rel {
                    SvgPoint::new(pos.x + x, pos.y + y)
                } else {
                    SvgPoint::new(x, y)
                };
                ensure_open!(pos);
                if let Some(sp) = &mut cur {
                    sp.segs.push(Seg::Quad { ctrl, to });
                }
                last_cubic_ctrl = None;
                last_quad_ctrl = Some(ctrl);
                pos = to;
                prev_cmd = Some(cmd);
            }
            b'A' | b'a' => {
                let (Some(rx), Some(ry), Some(rot), Some(large), Some(sweep), Some((x, y))) = (
                    c.number(),
                    c.number(),
                    c.number(),
                    c.flag(),
                    c.flag(),
                    c.pair(),
                ) else {
                    return Err(err(&c, "A needs rx ry rot flags x y"));
                };
                let to = if rel {
                    SvgPoint::new(pos.x + x, pos.y + y)
                } else {
                    SvgPoint::new(x, y)
                };
                let segs = arc_to_cubics(pos, rx, ry, rot, large, sweep, to);
                ensure_open!(pos);
                if let Some(sp) = &mut cur {
                    sp.segs.extend(segs);
                }
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
                pos = to;
                prev_cmd = Some(cmd);
            }
            b'Z' | b'z' => {
                if let Some(sp) = &mut cur {
                    sp.segs.push(Seg::Close);
                }
                flush!();
                if let Some(last) = subpaths.last() {
                    pos = last.start;
                }
                prev_cmd = Some(cmd);
            }
            other => {
                return Err(err(&c, &format!("nieznana komenda `{}`", other as char)));
            }
        }
    }
    flush!();
    Ok(subpaths)
}

fn reflect(ctrl: Option<SvgPoint>, around: SvgPoint) -> SvgPoint {
    match ctrl {
        Some(c) => SvgPoint::new(2.0 * around.x - c.x, 2.0 * around.y - c.y),
        None => around,
    }
}

fn arc_to_cubics(
    p0: SvgPoint,
    rx: f32,
    ry: f32,
    x_rot_deg: f32,
    large: bool,
    sweep: bool,
    p1: SvgPoint,
) -> Vec<Seg> {
    const TAU: f32 = std::f32::consts::TAU;
    if (p0.x - p1.x).abs() < 1e-6 && (p0.y - p1.y).abs() < 1e-6 {
        return Vec::new();
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-6 || ry < 1e-6 {
        return vec![Seg::Line { to: p1 }];
    }
    let phi = x_rot_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();

    let dx2 = (p0.x - p1.x) / 2.0;
    let dy2 = (p0.y - p1.y) / 2.0;
    let x1 = cos_phi * dx2 + sin_phi * dy2;
    let y1 = -sin_phi * dx2 + cos_phi * dy2;

    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let rx2 = rx * rx;
    let ry2 = ry * ry;
    let x1sq = x1 * x1;
    let y1sq = y1 * y1;
    let num = (rx2 * ry2 - rx2 * y1sq - ry2 * x1sq).max(0.0);
    let den = rx2 * y1sq + ry2 * x1sq;
    let sign = if large != sweep { 1.0 } else { -1.0 };
    let coef = sign * (num / den.max(f32::MIN_POSITIVE)).sqrt();
    let cxp = coef * rx * y1 / ry;
    let cyp = -coef * ry * x1 / rx;
    let cx = cos_phi * cxp - sin_phi * cyp + (p0.x + p1.x) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (p0.y + p1.y) / 2.0;

    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| -> f32 {
        let dot = ux * vx + uy * vy;
        let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        let mut a = if len > 0.0 {
            (dot / len).clamp(-1.0, 1.0).acos()
        } else {
            0.0
        };
        if ux * vy - uy * vx < 0.0 {
            a = -a;
        }
        a
    };
    let theta1 = angle(1.0, 0.0, (x1 - cxp) / rx, (y1 - cyp) / ry);
    let mut dtheta = angle(
        (x1 - cxp) / rx,
        (y1 - cyp) / ry,
        (-x1 - cxp) / rx,
        (-y1 - cyp) / ry,
    );
    if !sweep && dtheta > 0.0 {
        dtheta -= TAU;
    } else if sweep && dtheta < 0.0 {
        dtheta += TAU;
    }

    let n = ((dtheta.abs() / std::f32::consts::FRAC_PI_2).ceil() as usize).max(1);
    let delta = dtheta / n as f32;
    let k = 4.0 / 3.0 * (delta / 4.0).tan();
    let mut segs = Vec::with_capacity(n);
    let mut t = theta1;
    for _ in 0..n {
        let t2 = t + delta;
        let (st, ct) = t.sin_cos();
        let (st2, ct2) = t2.sin_cos();

        let map = |ex: f32, ey: f32| {
            SvgPoint::new(
                cos_phi * rx * ex - sin_phi * ry * ey + cx,
                sin_phi * rx * ex + cos_phi * ry * ey + cy,
            )
        };
        let pa = map(ct, st);
        let pb = map(ct2, st2);

        let d1 = SvgPoint::new(
            cos_phi * rx * (-st) * k - sin_phi * ry * ct * k,
            sin_phi * rx * (-st) * k + cos_phi * ry * ct * k,
        );
        let d2 = SvgPoint::new(
            cos_phi * rx * (-st2) * k - sin_phi * ry * ct2 * k,
            sin_phi * rx * (-st2) * k + cos_phi * ry * ct2 * k,
        );
        segs.push(Seg::Cubic {
            c1: SvgPoint::new(pa.x + d1.x, pa.y + d1.y),
            c2: SvgPoint::new(pb.x - d2.x, pb.y - d2.y),
            to: pb,
        });
        t = t2;
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flatten(icon: &SvgIcon) -> Vec<SvgPoint> {
        let mut pts = Vec::new();
        for s in &icon.shapes {
            for sp in &s.subpaths {
                pts.push(sp.start);
                for seg in &sp.segs {
                    match seg {
                        Seg::Line { to } => pts.push(*to),
                        Seg::Quad { to, .. } | Seg::Cubic { to, .. } => pts.push(*to),
                        Seg::Close => {}
                    }
                }
            }
        }
        pts
    }

    #[test]
    fn rect_and_viewbox() {
        let icon =
            parse_svg(r#"<svg viewBox="0 0 10 10"><rect x="1" y="2" width="4" height="3"/></svg>"#)
                .unwrap();
        assert_eq!(icon.viewbox, [0.0, 0.0, 10.0, 10.0]);
        assert_eq!(icon.shapes.len(), 1);
        let pts = flatten(&icon);
        assert!(pts.contains(&SvgPoint::new(1.0, 2.0)));
        assert!(pts.contains(&SvgPoint::new(5.0, 5.0)));
    }

    #[test]
    fn missing_viewbox_is_error() {
        assert_eq!(
            parse_svg("<svg><rect width='2' height='2'/></svg>"),
            Err(SvgError::MissingViewBox)
        );
    }

    #[test]
    fn stray_slash_does_not_hang() {
        assert_eq!(parse_svg("<svg / ></svg>"), Err(SvgError::MissingViewBox));
        assert_eq!(parse_svg("<svg /"), Err(SvgError::MissingViewBox));
        assert_eq!(parse_svg("<svg /x></svg>"), Err(SvgError::MissingViewBox));
    }

    #[test]
    fn stray_slash_after_valid_open_is_truncation_not_error() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><rect width="2" height="2" / ><circle cx="5" cy="5" r="1"/></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.viewbox, [0.0, 0.0, 10.0, 10.0]);
    }

    #[test]
    fn width_height_fallback_viewbox() {
        let icon = parse_svg(r#"<svg width="24" height="24"><circle cx="5" cy="5" r="4"/></svg>"#)
            .unwrap();
        assert_eq!(icon.viewbox, [0.0, 0.0, 24.0, 24.0]);
    }

    #[test]
    fn path_commands_mixed_relative_absolute() {
        let d = "M1 1 l2 0 H7 v2 C8 3 9 4 10 5 Q11 6 12 7 Z m1 1 h2";
        let subs = parse_path_data(d).unwrap();
        assert_eq!(subs.len(), 2, "Z ends the first, second from m");
        assert_eq!(subs[0].start, SvgPoint::new(1.0, 1.0));

        assert!(matches!(
            subs[0].segs[subs[0].segs.len() - 2],
            Seg::Quad {
                to: SvgPoint { x: 12.0, y: 7.0 },
                ..
            }
        ));
        assert!(matches!(subs[0].segs.last(), Some(Seg::Close)));
        assert_eq!(subs[1].start, SvgPoint::new(2.0, 2.0));
    }

    #[test]
    fn numbers_without_separators() {
        let subs = parse_path_data("M1.5.5 L-2e1 3").unwrap();
        assert_eq!(subs[0].start, SvgPoint::new(1.5, 0.5));
        assert!(matches!(&subs[0].segs[0], Seg::Line { to } if *to == SvgPoint::new(-20.0, 3.0)));
    }

    #[test]
    fn arc_full_circle_becomes_four_cubics() {
        let subs = parse_path_data("M4 11 a7 7 0 1 0 14 0 a7 7 0 1 0 -14 0 Z").unwrap();
        let cubics = subs[0]
            .segs
            .iter()
            .filter(|s| matches!(s, Seg::Cubic { .. }))
            .count();
        assert_eq!(cubics, 4, "full circle = two 180deg arcs of 2 quarters");
    }

    #[test]
    fn arc_endpoints_reached() {
        let subs = parse_path_data("M0 0 A5 5 0 0 1 10 0").unwrap();
        let last = match &subs[0].segs.last() {
            Some(Seg::Cubic { to, .. }) => *to,
            other => panic!("expected a curve: {other:?}"),
        };
        assert!((last.x - 10.0).abs() < 1e-4 && last.y.abs() < 1e-4);
    }

    #[test]
    fn smooth_commands_reflect_controls() {
        let subs = parse_path_data("M0 0 C1 1 2 1 3 0 S5 -1 6 0").unwrap();
        match &subs[0].segs[1] {
            Seg::Cubic { c1, .. } => assert_eq!(*c1, SvgPoint::new(4.0, -1.0)),
            other => panic!("expected C: {other:?}"),
        }
    }

    #[test]
    fn fill_rule_and_none_fill() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10">
                 <path fill="none" d="M0 0 H10"/>
                 <path fill-rule="evenodd" d="M1 1 H5 V5 Z"/>
                 <g fill-rule="nonzero"><rect width="2" height="2"/></g>
               </svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes.len(), 2, "fill=none pomijany");
        assert_eq!(icon.shapes[0].fill_rule, FillRule::EvenOdd);
        assert_eq!(icon.shapes[1].fill_rule, FillRule::NonZero);
    }

    #[test]
    fn comments_and_unknown_tags_skipped() {
        let icon = parse_svg(
            r#"<?xml version="1.0"?><!-- nota --><svg viewBox="0 0 8 8"><defs><style>.a{}</style></defs><rect width="8" height="8"/></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes.len(), 1);
    }

    #[test]
    fn rounded_rect_uses_arcs() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 20 20"><rect x="2" y="2" width="16" height="16" rx="4"/></svg>"#,
        )
        .unwrap();
        let cubics = flatten(&icon).len();
        assert!(cubics >= 8, "roundings give curves");
    }

    #[test]
    fn bad_path_reports_position() {
        let e = parse_path_data("M1 1 X2 2").unwrap_err();
        assert!(e.1.contains("nieznana"));
    }

    #[test]
    fn group_fill_rule_inherited() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><g fill-rule="evenodd"><path d="M1 1 H5 V5 Z"/></g></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes.len(), 1);
        assert_eq!(icon.shapes[0].fill_rule, FillRule::EvenOdd);
    }

    #[test]
    fn group_none_fill_hides_children_unless_overridden() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><g fill="none"><rect width="4" height="4"/><rect x="5" y="5" width="4" height="4" fill="red"/></g></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes.len(), 1, "only the overridden fill paints");
        assert_eq!(icon.shapes[0].subpaths[0].start, SvgPoint::new(5.0, 5.0));
    }

    #[test]
    fn style_attr_provides_fill_rule() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path style="fill-rule:evenodd" d="M1 1 H5 V5 Z"/></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes[0].fill_rule, FillRule::EvenOdd);
    }

    #[test]
    fn lengths_with_units_parse() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 24 24"><rect x="1px" y="2pt" width="4" height="3"/></svg>"#,
        )
        .unwrap();
        let pts = flatten(&icon);
        assert!(pts.contains(&SvgPoint::new(1.0, 2.0)));
    }

    #[test]
    fn line_uses_endpoints() {
        let icon =
            parse_svg(r#"<svg viewBox="0 0 10 10"><line x1="1" y1="2" x2="7" y2="8"/></svg>"#)
                .unwrap();
        assert_eq!(icon.shapes.len(), 1);
        assert_eq!(icon.shapes[0].subpaths[0].start, SvgPoint::new(1.0, 2.0));
    }

    #[test]
    fn group_translate_moves_shape() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><g transform="translate(5 0)"><rect width="2" height="2"/></g></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes[0].subpaths[0].start, SvgPoint::new(5.0, 0.0));
    }

    #[test]
    fn shape_scale_doubles_coords() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><rect width="2" height="2" transform="scale(2)"/></svg>"#,
        )
        .unwrap();
        let pts = flatten(&icon);
        assert!(pts.contains(&SvgPoint::new(4.0, 4.0)));
    }

    #[test]
    fn stroke_outline_path_is_kept_with_fill_none() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-width="2" d="M5 12 H19"/></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes.len(), 1);
        let s = &icon.shapes[0];
        assert!(!s.fill, "fill=none -> no fill");
        let st = s.stroke.as_ref().expect("outline kept");
        assert!((st.width - 2.0).abs() < 1e-6);
    }

    #[test]
    fn stroke_inherited_from_svg_root_feather_style() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12 H19"/></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes.len(), 1);
        let s = &icon.shapes[0];
        assert!(!s.fill);
        let st = s.stroke.as_ref().unwrap();
        assert_eq!(st.cap, StrokeLineCap::Round);
        assert_eq!(st.join, StrokeLineJoin::Round);
        assert!((st.width - 2.0).abs() < 1e-6);
    }

    #[test]
    fn stroke_none_means_no_stroke() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><rect width="4" height="4" stroke="none" stroke-width="5"/></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes.len(), 1);
        assert!(icon.shapes[0].fill);
        assert!(icon.shapes[0].stroke.is_none());
    }

    #[test]
    fn fill_none_without_stroke_gives_no_shapes() {
        let icon = parse_svg(r#"<svg viewBox="0 0 10 10"><path fill="none" d="M0 0 H10"/></svg>"#)
            .unwrap();
        assert!(
            icon.shapes.is_empty(),
            "fill=none alone without stroke is empty"
        );
    }

    #[test]
    fn line_with_stroke_is_kept() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><line x1="1" y1="2" x2="7" y2="8" stroke="black" stroke-width="1"/></svg>"#,
        )
        .unwrap();

        assert_eq!(icon.shapes.len(), 1);
        assert!(icon.shapes[0].stroke.is_some());
    }

    #[test]
    fn stroke_width_scales_with_transform() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><rect width="2" height="2" transform="scale(2)" stroke="black" stroke-width="1"/></svg>"#,
        )
        .unwrap();
        let st = icon.shapes[0].stroke.as_ref().unwrap();
        assert!(
            (st.width - 2.0).abs() < 1e-6,
            "skala ×2 → width {:?}, ",
            st.width
        );
    }

    #[test]
    fn nonscaling_stroke_ignores_transform() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><rect width="2" height="2" transform="scale(3)" stroke="black" stroke-width="2" vector-effect="non-scaling-stroke"/></svg>"#,
        )
        .unwrap();
        let st = icon.shapes[0].stroke.as_ref().unwrap();
        assert!(
            (st.width - 2.0).abs() < 1e-6,
            "non-scaling-stroke: {:?}",
            st.width
        );
    }

    #[test]
    fn stroke_dasharray_parsed() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path stroke="black" stroke-width="1" stroke-dasharray="3 1" d="M0 0 H10"/></svg>"#,
        )
        .unwrap();
        let st = icon.shapes[0].stroke.as_ref().unwrap();
        assert_eq!(st.dash_array.len(), 2);

        let icon2 = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path stroke="black" stroke-dasharray="2" d="M0 0 H10"/></svg>"#,
        )
        .unwrap();
        assert_eq!(
            icon2.shapes[0].stroke.as_ref().unwrap().dash_array,
            vec![2.0]
        );
        let icon3 = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path stroke="black" stroke-dasharray="none" d="M0 0 H10"/></svg>"#,
        )
        .unwrap();
        assert!(
            icon3.shapes[0]
                .stroke
                .as_ref()
                .unwrap()
                .dash_array
                .is_empty()
        );
    }

    #[test]
    fn stroke_caps_joins_and_miter_parsed() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path stroke="black" stroke-linecap="square" stroke-linejoin="bevel" stroke-miterlimit="2" d="M0 0 H10"/></svg>"#,
        )
        .unwrap();
        let st = icon.shapes[0].stroke.as_ref().unwrap();
        assert_eq!(st.cap, StrokeLineCap::Square);
        assert_eq!(st.join, StrokeLineJoin::Bevel);
        assert!((st.miter_limit - 2.0).abs() < 1e-6);
    }

    #[test]
    fn stroke_inherited_through_g() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><g stroke="red" stroke-width="1.5"><path d="M0 0 H5"/></g></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.shapes.len(), 1);

        assert!(icon.shapes[0].fill);
        let st = icon.shapes[0].stroke.as_ref().unwrap();
        assert!((st.width - 1.5).abs() < 1e-6);
    }

    #[test]
    fn style_attr_provides_stroke() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path style="fill:none;stroke:black;stroke-width:2" d="M0 0 H10"/></svg>"#,
        )
        .unwrap();
        assert!(!icon.shapes[0].fill);
        assert!((icon.shapes[0].stroke.as_ref().unwrap().width - 2.0).abs() < 1e-6);
    }
}
