//! Parser for a subset of Svelte → AST. Handles only what `to_svelte` generates.
//!
//! Behaviors round-trip through the `<!-- vd-behavior: {…} -->` comments
//! (design doc §6.3): the editor stays the source of truth, so rules and
//! bindings are read from the payload, never re-parsed from the code.

use sequoia_ui::prelude::{Color, Edges};

use crate::document::json::{self, VdPayload};
use crate::model::svelte_ast::*;

pub fn from_svelte(src: &str) -> Result<Option<SvelteNode>, String> {
    let (body, style_src) = split_blocks(src);
    let css = parse_css(&style_src);
    let mut parser = BodyParser {
        chars: body.chars().collect(),
        pos: 0,
        next_id: 1,
        css: &css,
        pending_fill: std::collections::HashMap::new(),
        pending: None,
    };
    let mut children = Vec::new();
    while let Some(n) = parser.parse_node()? {
        children.push(n);
    }
    if children.is_empty() {
        return Ok(None);
    }
    if children.len() == 1 {
        return Ok(Some(children.remove(0)));
    }
    Ok(Some(SvelteNode::Element(Element {
        id: parser.next_id,
        name: "Root".into(),
        tag: Tag::Div,
        style: Style::default(),
        children,
        visible: true,
        locked: false,
        component: false,
        instance: false,
        interface: None,
        overrides: Vec::new(),
        slot_content: Vec::new(),
        behavior: None,
        binding: None,
        hover: None,
        prototype: None,
    })))
}

fn split_blocks(src: &str) -> (String, String) {
    let mut body = String::new();
    let mut style = String::new();
    let mut chars = src.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '<' {
            let rest: String = chars.clone().collect();
            if rest.starts_with("<script") {
                if let Some(end) = rest.find("</script>") {
                    for _ in 0..(end + "</script>".len()) {
                        chars.next();
                    }
                    continue;
                }
            }
            if rest.starts_with("<style") {
                if let Some(gt) = rest.find('>') {
                    let content_start = gt + 1;
                    let after: String = chars.clone().skip(content_start).collect();
                    if let Some(end) = after.find("</style>") {
                        style.push_str(&after[..end]);
                        for _ in 0..(content_start + end + "</style>".len()) {
                            chars.next();
                        }
                        continue;
                    }
                }
            }
        }
        body.push(c);
        chars.next();
    }
    (body, style)
}

#[derive(Clone, Default)]
struct CssRule {
    props: Vec<(String, String)>,
}

fn parse_css(src: &str) -> std::collections::HashMap<String, CssRule> {
    let mut map = std::collections::HashMap::new();
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '.' {
            let mut name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    name.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            // skip whitespace
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
            if chars.peek() != Some(&'{') {
                continue;
            }
            chars.next();
            let mut body = String::new();
            let mut depth = 1;
            while let Some(c) = chars.next() {
                if c == '{' {
                    depth += 1;
                }
                if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                body.push(c);
            }
            let mut rule = CssRule::default();
            for decl in body.split(';') {
                let decl = decl.trim();
                if decl.is_empty() {
                    continue;
                }
                if let Some(idx) = decl.find(':') {
                    let k = decl[..idx].trim().to_string();
                    let v = decl[idx + 1..].trim().to_string();
                    rule.props.push((k, v));
                }
            }
            map.insert(name, rule);
        }
    }
    map
}

fn style_from_css(rule: Option<&CssRule>, fill: &mut FillFlags) -> Style {
    let mut s = Style::default();
    let mut gap_single: Option<f32> = None;
    let mut gap_pair: Option<(f32, f32)> = None;
    let Some(rule) = rule else { return s };
    for (k, v) in &rule.props {
        let px = || parse_px(v).unwrap_or(0.0);
        match k.as_str() {
            "display" => match v.as_str() {
                "flex" => s.display = Display::Flex,
                "grid" => s.display = Display::Grid,
                _ => s.display = Display::None,
            },
            "flex-direction" => {
                s.flex_dir = if v == "column" {
                    FlexDir::Col
                } else {
                    FlexDir::Row
                }
            }
            "flex-wrap" => {
                s.flex_wrap = if v.trim() == "wrap" {
                    Wrap::Wrap
                } else {
                    Wrap::NoWrap
                }
            }
            "align-content" => s.align_content = Align::from_content(v),
            "flex" => {
                // `flex: 1 0 0` — the grow factor marks a fill on the main axis.
                let grow = v
                    .split_whitespace()
                    .next()
                    .and_then(|g| g.parse::<f32>().ok())
                    .unwrap_or(0.0);
                if grow > 0.0 {
                    fill.main = true;
                }
            }
            "align-self" => {
                s.align_self = Align::from_self(v);
                if s.align_self == Some(Align::Stretch) {
                    fill.cross = true;
                }
            }
            "gap" => {
                let parts: Vec<f32> = v.split_whitespace().filter_map(parse_px).collect();
                match parts.as_slice() {
                    [g] => gap_single = Some(*g),
                    [r, c, ..] => gap_pair = Some((*r, *c)),
                    _ => {}
                }
            }
            "padding" => {
                let parts: Vec<f32> = v.split_whitespace().filter_map(parse_px).collect();
                s.padding = match parts.len() {
                    1 => Edges::all(parts[0]),
                    2 => Edges::xy(parts[1], parts[0]),
                    4 => Edges::only(parts[3], parts[0], parts[1], parts[2]),
                    _ => Edges::all(0.0),
                };
            }
            "align-items" => s.align_items = Align::from_items(v),
            "justify-content" => s.justify_content = Align::from_content(v),
            "grid-template-columns" => s.grid_cols = parse_grid_tracks(v),
            "grid-template-rows" => s.grid_rows = parse_grid_tracks(v),
            "position" if v == "absolute" => s.position = Position::Absolute,
            "top" => s.top = parse_px(v),
            "right" => s.right = parse_px(v),
            "bottom" => s.bottom = parse_px(v),
            "left" => s.left = parse_px(v),
            "width" => {
                s.width = if v == "100%" {
                    Dimension::Fill
                } else if v == "auto" {
                    Dimension::Auto
                } else {
                    Dimension::Fixed(parse_px(v).unwrap_or(0.0))
                }
            }
            "height" => {
                s.height = if v == "100%" {
                    Dimension::Fill
                } else if v == "auto" {
                    Dimension::Auto
                } else {
                    Dimension::Fixed(parse_px(v).unwrap_or(0.0))
                }
            }
            "background" | "background-color" => {
                if let Some(g) = parse_gradient_css(v) {
                    s.gradient = Some(g);
                } else if let Some(c) = parse_color(v) {
                    s.fill = Some(c);
                    s.fill_hex = format!("#{:06X}", c.hex_rgb());
                }
            }
            "box-shadow" => s.effects = parse_shadow_css(v),
            "filter" => {
                if let Some(px) = parse_blur_css(v) {
                    s.blur = px;
                }
            }
            "border-radius" => {
                if v == "50%" {
                    s.radius = 9999.0;
                } else {
                    s.radius = px();
                }
            }
            "opacity" => s.opacity = v.parse::<f32>().unwrap_or(1.0) * 100.0,
            _ => {}
        }
    }
    if s.display == Display::Flex {
        let (row, col) = gap_pair
            .or(gap_single.map(|g| (g, g)))
            .unwrap_or((s.gap, s.gap));
        if s.flex_dir == FlexDir::Row {
            s.gap = col;
            s.row_gap = row;
        } else {
            s.gap = row;
            s.row_gap = col;
        }
    } else if s.display == Display::Grid {
        let (row, col) = gap_pair
            .or(gap_single.map(|g| (g, g)))
            .unwrap_or((s.gap, s.row_gap));
        s.gap = col;
        s.row_gap = row;
    }
    s
}

/// `repeat(3, 1fr)` or a track list (`1fr 1fr`) → explicit track count.
fn parse_grid_tracks(v: &str) -> Option<u16> {
    let v = v.trim();
    if v.is_empty() || v == "none" {
        return None;
    }
    if let Some(inner) = v.strip_prefix("repeat(").and_then(|s| s.strip_suffix(')')) {
        return inner
            .split(',')
            .next()
            .and_then(|n| n.trim().parse::<u16>().ok())
            .map(|n| n.max(1));
    }
    let n = v
        .split_whitespace()
        .filter(|t| *t != "none" && *t != "auto")
        .count();
    (n > 0).then_some(n as u16)
}

#[derive(Default, Clone, Copy)]
struct FillFlags {
    main: bool,
    cross: bool,
}

/// `flex: 1` and `align-self: stretch` only mean "fill" relative to a flex
/// parent, so the axes are resolved once the parent's direction is known.
fn resolve_child_fills(
    parent: &Style,
    children: &mut [SvelteNode],
    pending: &mut std::collections::HashMap<u64, FillFlags>,
) {
    let row = parent.display == Display::Flex && parent.flex_dir == FlexDir::Row;
    let col = parent.display == Display::Flex && parent.flex_dir == FlexDir::Col;
    if !row && !col {
        return;
    }
    for c in children.iter_mut() {
        if let SvelteNode::Element(e) = c
            && let Some(flags) = pending.remove(&e.id)
        {
            if row {
                if flags.main {
                    e.style.width = Dimension::Fill;
                }
                if flags.cross {
                    e.style.height = Dimension::Fill;
                }
            } else {
                if flags.main {
                    e.style.height = Dimension::Fill;
                }
                if flags.cross {
                    e.style.width = Dimension::Fill;
                }
            }
        }
    }
}

struct TextPlacement {
    x: f32,
    y: f32,
    absolute: bool,
}

fn text_style_from_css(
    rule: Option<&CssRule>,
    w: &mut Option<f32>,
    h: &mut Option<f32>,
    place: &mut TextPlacement,
) -> TextStyle {
    let mut t = TextStyle::default();
    let Some(rule) = rule else { return t };
    for (k, v) in &rule.props {
        match k.as_str() {
            "width" => *w = parse_px(v),
            "height" => *h = parse_px(v),
            "position" => place.absolute = v.trim() == "absolute",
            "left" => place.x = parse_px(v).unwrap_or(0.0),
            "top" => place.y = parse_px(v).unwrap_or(0.0),
            "font-size" => t.font_size = parse_px(v).unwrap_or(14.0),
            "font-weight" => t.font_weight = v.parse::<u16>().unwrap_or(400),
            "opacity" => t.opacity = parse_opacity(v),
            "color" => {
                if let Some(c) = parse_color(v) {
                    t.color = c;
                    t.color_hex = format!("#{:06X}", c.hex_rgb());
                }
            }
            "text-align" => {
                t.align = match v.as_str() {
                    "center" => Align::Center,
                    "right" => Align::End,
                    _ => Align::Start,
                }
            }
            _ => {}
        }
    }
    t
}

fn parse_px(v: &str) -> Option<f32> {
    let v = v.trim().trim_end_matches("px");
    v.parse::<f32>().ok()
}

/// CSS opacity is 0..1; the editor stores percent.
fn parse_opacity(v: &str) -> f32 {
    let v = v.trim();
    if let Some(n) = v.strip_suffix('%') {
        n.trim().parse::<f32>().unwrap_or(100.0).clamp(0.0, 100.0)
    } else {
        v.parse::<f32>()
            .map(|n| (n * 100.0).clamp(0.0, 100.0))
            .unwrap_or(100.0)
    }
}

/// Splits a CSS value on top-level commas (gradient stops, shadow lists).
fn split_top_level(src: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for c in src.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c == sep && depth == 0 {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

fn parse_gradient_css(v: &str) -> Option<GradientDef> {
    let v = v.trim();
    let (kind, body) = if let Some(b) = v.strip_prefix("linear-gradient(") {
        (GradientKind::Linear, b.strip_suffix(')')?)
    } else if let Some(b) = v.strip_prefix("radial-gradient(") {
        (GradientKind::Radial, b.strip_suffix(')')?)
    } else {
        return None;
    };
    let parts = split_top_level(body, ',');
    let mut angle = 90.0;
    let mut start = 0;
    if kind == GradientKind::Linear
        && let Some(deg) = parts
            .first()
            .and_then(|p| p.trim().strip_suffix("deg"))
            .and_then(|n| n.trim().parse::<f32>().ok())
    {
        angle = deg;
        start = 1;
    }
    let mut stops = Vec::new();
    for part in &parts[start..] {
        let p = part.trim();
        if p.is_empty() || p.starts_with("circle") || p.starts_with("ellipse") {
            continue;
        }
        let tokens: Vec<&str> = p.split_whitespace().collect();
        let Some(color) = tokens.first().and_then(|t| parse_color(t)) else {
            continue;
        };
        let at = tokens
            .iter()
            .skip(1)
            .find_map(|t| t.strip_suffix('%'))
            .and_then(|n| n.trim().parse::<f32>().ok())
            .map(|p| (p / 100.0).clamp(0.0, 1.0))
            .unwrap_or(if stops.is_empty() { 0.0 } else { 1.0 });
        stops.push(GradientStopDef {
            color,
            hex: format!("#{:06X}", color.hex_rgb()),
            at,
        });
    }
    if stops.len() < 2 {
        return None;
    }
    Some(GradientDef { kind, angle, stops })
}

fn parse_shadow_css(v: &str) -> Vec<ShadowEffect> {
    split_top_level(v, ',')
        .iter()
        .filter_map(|part| {
            let tokens: Vec<&str> = part.split_whitespace().collect();
            let color = tokens.iter().rev().find_map(|t| parse_color(t))?;
            let nums: Vec<f32> = tokens.iter().filter_map(|t| parse_px(t)).collect();
            if nums.is_empty() {
                return None;
            }
            Some(ShadowEffect {
                color,
                hex: format!("#{:06X}", color.hex_rgb()),
                x: nums.first().copied().unwrap_or(0.0),
                y: nums.get(1).copied().unwrap_or(0.0),
                blur: nums.get(2).copied().unwrap_or(0.0),
                spread: nums.get(3).copied().unwrap_or(0.0),
                enabled: true,
            })
        })
        .collect()
}

fn parse_blur_css(v: &str) -> Option<f32> {
    v.trim()
        .strip_prefix("blur(")?
        .strip_suffix(')')?
        .trim()
        .trim_end_matches("px")
        .parse::<f32>()
        .ok()
}

fn parse_color(v: &str) -> Option<Color> {
    let v = v.trim();
    if let Some(h) = v.strip_prefix('#') {
        let h = h.trim();
        let n = u32::from_str_radix(h, 16).ok()?;
        return Some(Color::hex(n));
    }
    if v.starts_with("rgba(") || v.starts_with("rgb(") {
        let inner = v
            .trim_start_matches("rgba(")
            .trim_start_matches("rgb(")
            .trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 3 {
            let r = parts[0].parse::<u8>().unwrap_or(0) as u32;
            let g = parts[1].parse::<u8>().unwrap_or(0) as u32;
            let b = parts[2].parse::<u8>().unwrap_or(0) as u32;
            let a = parts
                .get(3)
                .and_then(|p| p.parse::<f32>().ok())
                .map(|a| (a.clamp(0.0, 1.0) * 255.0).round() as u32)
                .unwrap_or(255);
            return Some(Color::hexa((r << 24) | (g << 16) | (b << 8) | a));
        }
    }
    None
}

struct BodyParser<'a> {
    chars: Vec<char>,
    pos: usize,
    next_id: u64,
    css: &'a std::collections::HashMap<String, CssRule>,
    /// Fill flags per element id, resolved once the parent's direction is known.
    pending_fill: std::collections::HashMap<u64, FillFlags>,
    /// Payload of a `vd-behavior` comment waiting for the next element.
    pending: Option<VdPayload>,
}

impl<'a> BodyParser<'a> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    fn consume(&mut self, s: &str) -> bool {
        if self.chars[self.pos..]
            .iter()
            .collect::<String>()
            .starts_with(s)
        {
            self.pos += s.chars().count();
            true
        } else {
            false
        }
    }
    fn parse_node(&mut self) -> Result<Option<SvelteNode>, String> {
        self.skip_ws();
        if self.pos >= self.chars.len() {
            return Ok(None);
        }
        if self.consume("<!--") {
            // `<!-- vd-behavior: {…} -->` — editor state for the NEXT element.
            let rest: String = self.chars[self.pos..self.pos + 16.min(self.chars.len() - self.pos)]
                .iter()
                .collect();
            if rest.starts_with(" vd-behavior:") {
                self.pos += " vd-behavior:".chars().count();
                self.skip_ws();
                if let Some(src) = self.scan_balanced_json() {
                    match json::parse_json(&src) {
                        Ok(v) => {
                            let payload = json::vd_payload_from_json(&v);
                            if !payload.is_empty() {
                                self.pending = Some(payload);
                            }
                        }
                        Err(_) => {
                            // Unparsable payload — keep going, the code is
                            // still a valid visual document.
                        }
                    }
                }
            }
            while self.pos < self.chars.len() {
                if self.consume("-->") {
                    break;
                }
                self.pos += 1;
            }
            return self.parse_node();
        }
        if !self.consume("<") {
            // text content (unlikely at top level, skip)
            while let Some(c) = self.bump() {
                if c == '<' {
                    self.pos -= 1;
                    break;
                }
            }
            return self.parse_node();
        }
        // tag name
        let mut tag_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() {
                tag_str.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        let Some(tag) = Tag::from_str(&tag_str) else {
            return Err(format!("Unknown tag: {tag_str}"));
        };
        // A `vd-behavior` comment right before this element belongs to it —
        // claim the payload before any child can claim it.
        let my_vd = self.pending.take();
        // attributes
        let mut class: Option<String> = None;
        loop {
            self.skip_ws();
            if self.consume("/>") {
                // self-closing
                let id = self.next_id;
                self.next_id += 1;
                let mut fill = FillFlags::default();
                let style = style_from_css(class.as_ref().and_then(|c| self.css.get(c)), &mut fill);
                if fill.main || fill.cross {
                    self.pending_fill.insert(id, fill);
                }
                let (vb, vbd, vh, vp) = split_vd(my_vd);
                return Ok(Some(SvelteNode::Element(Element {
                    id,
                    name: tag_str.clone(),
                    tag,
                    style,
                    children: vec![],
                    visible: true,
                    locked: false,
                    component: false,
                    instance: false,
                    interface: None,
                    overrides: Vec::new(),
                    slot_content: Vec::new(),
                    behavior: vb,
                    binding: vbd,
                    hover: vh,
                    prototype: vp,
                })));
            }
            if self.consume(">") {
                break;
            }
            // attr
            let mut key = String::new();
            while let Some(c) = self.peek() {
                if c.is_whitespace() || c == '=' || c == '>' {
                    break;
                }
                key.push(c);
                self.pos += 1;
            }
            if self.consume("=") {
                // Svelte expression attribute (`onclick={() => open = !open}`)
                // — skipped; the model arrives via the vd-behavior payload.
                if self.peek() == Some('{') {
                    let _ = self.scan_braces();
                    continue;
                }
                let quote = self.bump().ok_or("missing quote")?;
                let mut val = String::new();
                while let Some(c) = self.bump() {
                    if c == quote {
                        break;
                    }
                    val.push(c);
                }
                if key == "class" {
                    class = Some(val);
                }
            } else if key.is_empty() {
                // Malformed input: no key, no `=` — skip one character so
                // the loop always makes progress.
                self.pos += 1;
                if self.pos >= self.chars.len() {
                    break;
                }
            }
        }
        // children
        let mut children = Vec::new();
        loop {
            self.skip_ws();
            if self.consume("</") {
                // close tag
                let mut close = String::new();
                while let Some(c) = self.peek() {
                    if c == '>' {
                        self.pos += 1;
                        break;
                    }
                    close.push(c);
                    self.pos += 1;
                }
                break;
            }
            if self.pos >= self.chars.len() {
                break;
            }
            if self.consume("<span") {
                // span with class
                let mut cls = String::new();
                while let Some(c) = self.peek() {
                    if c == '>' {
                        self.pos += 1;
                        break;
                    }
                    cls.push(c);
                    self.pos += 1;
                }
                let cls = cls
                    .trim()
                    .trim_start_matches("class=")
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                let mut content = String::new();
                while self.pos < self.chars.len() {
                    if self.chars[self.pos..]
                        .iter()
                        .collect::<String>()
                        .starts_with("</span>")
                    {
                        self.pos += "</span>".len();
                        break;
                    }
                    content.push(self.chars[self.pos]);
                    self.pos += 1;
                }
                let id = self.next_id;
                self.next_id += 1;
                let mut w = None;
                let mut h = None;
                let mut place = TextPlacement {
                    x: 0.0,
                    y: 0.0,
                    absolute: false,
                };
                let ts = text_style_from_css(self.css.get(&cls), &mut w, &mut h, &mut place);
                children.push(SvelteNode::Text(TextNode {
                    id,
                    name: cls.clone(),
                    content,
                    style: ts,
                    width: w,
                    height: h,
                    x: place.x,
                    y: place.y,
                    absolute: place.absolute,
                    visible: true,
                    locked: false,
                }));
                continue;
            }
            if self.consume("<") {
                self.pos -= 1;
                match self.parse_node()? {
                    Some(n) => children.push(n),
                    None => break,
                }
                continue;
            }
            // plain text between elements — skipped (our codegen always uses span)
            while let Some(c) = self.bump() {
                if c == '<' {
                    self.pos -= 1;
                    break;
                }
            }
        }

        let id = self.next_id;
        self.next_id += 1;
        let mut fill = FillFlags::default();
        let style = style_from_css(class.as_ref().and_then(|c| self.css.get(c)), &mut fill);
        if fill.main || fill.cross {
            self.pending_fill.insert(id, fill);
        }
        resolve_child_fills(&style, &mut children, &mut self.pending_fill);
        let (vb, vbd, vh, vp) = split_vd(my_vd);
        Ok(Some(SvelteNode::Element(Element {
            id,
            name: class.unwrap_or_else(|| format!("{}-{}", tag_str, id)),
            tag,
            style,
            children,
            visible: true,
            locked: false,
            component: false,
            instance: false,
            interface: None,
            overrides: Vec::new(),
            slot_content: Vec::new(),
            behavior: vb,
            binding: vbd,
            hover: vh,
            prototype: vp,
        })))
    }

    /// Skips a balanced `{…}` expression starting at the current position
    /// (Svelte attribute value). Understands nested braces and string
    /// literals, so `onclick={() => e.key === '}'}` survives.
    fn scan_braces(&mut self) -> usize {
        let mut depth = 0usize;
        while let Some(c) = self.peek() {
            match c {
                '{' => {
                    depth += 1;
                    self.pos += 1;
                }
                '}' => {
                    depth -= 1;
                    self.pos += 1;
                    if depth == 0 {
                        break;
                    }
                }
                '\'' | '"' => {
                    let quote = c;
                    self.pos += 1;
                    while let Some(s) = self.peek() {
                        self.pos += 1;
                        if s == quote {
                            break;
                        }
                        if s == '\\' {
                            self.pos += 1;
                        }
                    }
                }
                _ => self.pos += 1,
            }
        }
        depth
    }

    /// Reads the balanced JSON object after `vd-behavior:` (robust against
    /// `-->` inside strings — the closing brace is found by depth, not by
    /// searching for the comment end).
    fn scan_balanced_json(&mut self) -> Option<String> {
        if self.peek() != Some('{') {
            return None;
        }
        let start = self.pos;
        self.scan_braces();
        // Depth returned 0 (or the input ran out) — take what was scanned.
        let end = self.pos;
        Some(self.chars[start..end].iter().collect())
    }
}

trait HexRgb {
    fn hex_rgb(self) -> u32;
}
impl HexRgb for Color {
    fn hex_rgb(self) -> u32 {
        let (r, g, b, _) = self.to_rgba8();
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}

fn split_vd(
    payload: Option<VdPayload>,
) -> (
    Option<crate::model::behavior::Behavior>,
    Option<crate::model::behavior::Binding>,
    Option<crate::model::behavior::HoverStyle>,
    Option<PrototypeLink>,
) {
    let Some(p) = payload else {
        return (None, None, None, None);
    };
    (p.behavior, p.binding, p.hover, p.prototype)
}
