//! Parser for a subset of Svelte → AST. Handles only what `to_svelte` generates.

use sequoia_ui::prelude::{Color, Edges};

use crate::model::svelte_ast::*;

pub fn from_svelte(src: &str) -> Result<Option<SvelteNode>, String> {
    let (body, style_src) = split_blocks(src);
    let css = parse_css(&style_src);
    let mut parser = BodyParser {
        chars: body.chars().collect(),
        pos: 0,
        next_id: 1,
        css: &css,
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

fn style_from_css(rule: Option<&CssRule>) -> Style {
    let mut s = Style::default();
    let Some(rule) = rule else { return s };
    for (k, v) in &rule.props {
        let px = || parse_px(v).unwrap_or(0.0);
        match k.as_str() {
            "display" => match v.as_str() {
                "flex" => s.display = Display::Flex,
                "grid" => s.display = Display::Grid,
                _ => s.display = Display::None,
            },
            "flex-direction" => s.flex_dir = if v == "column" { FlexDir::Col } else { FlexDir::Row },
            "gap" => s.gap = px(),
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
            "position" if v == "absolute" => s.position = Position::Absolute,
            "top" => s.top = parse_px(v),
            "right" => s.right = parse_px(v),
            "bottom" => s.bottom = parse_px(v),
            "left" => s.left = parse_px(v),
            "width" => {
                s.width = if v == "100%" { Dimension::Fill }
                    else if v == "auto" { Dimension::Auto }
                    else { Dimension::Fixed(parse_px(v).unwrap_or(0.0)) }
            }
            "height" => {
                s.height = if v == "100%" { Dimension::Fill }
                    else if v == "auto" { Dimension::Auto }
                    else { Dimension::Fixed(parse_px(v).unwrap_or(0.0)) }
            }
            "background" | "background-color" => {
                if let Some(c) = parse_color(v) {
                    s.fill = Some(c);
                    s.fill_hex = format!("#{:06X}", c.hex_rgb());
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
    s
}

fn text_style_from_css(rule: Option<&CssRule>, w: &mut Option<f32>, h: &mut Option<f32>) -> TextStyle {
    let mut t = TextStyle::default();
    let Some(rule) = rule else { return t };
    for (k, v) in &rule.props {
        match k.as_str() {
            "width" => *w = parse_px(v),
            "height" => *h = parse_px(v),
            "font-size" => t.font_size = parse_px(v).unwrap_or(14.0),
            "font-weight" => t.font_weight = v.parse::<u16>().unwrap_or(400),
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

fn parse_color(v: &str) -> Option<Color> {
    let v = v.trim();
    if let Some(h) = v.strip_prefix('#') {
        let h = h.trim();
        let n = u32::from_str_radix(h, 16).ok()?;
        return Some(Color::hex(n));
    }
    if v.starts_with("rgba(") || v.starts_with("rgb(") {
        let inner = v.trim_start_matches("rgba(").trim_start_matches("rgb(").trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 3 {
            let r = parts[0].parse::<u8>().unwrap_or(0) as u32;
            let g = parts[1].parse::<u8>().unwrap_or(0) as u32;
            let b = parts[2].parse::<u8>().unwrap_or(0) as u32;
            return Some(Color::hex((r << 16) | (g << 8) | b));
        }
    }
    None
}

struct BodyParser<'a> {
    chars: Vec<char>,
    pos: usize,
    next_id: u64,
    css: &'a std::collections::HashMap<String, CssRule>,
}

impl<'a> BodyParser<'a> {
    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() { self.pos += 1; }
        c
    }
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() { self.pos += 1; } else { break; }
        }
    }
    fn consume(&mut self, s: &str) -> bool {
        if self.chars[self.pos..].iter().collect::<String>().starts_with(s) {
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
            while self.pos < self.chars.len() {
                if self.consume("-->") { break; }
                self.pos += 1;
            }
            return self.parse_node();
        }
        if !self.consume("<") {
            // text content (unlikely at top level, skip)
            while let Some(c) = self.bump() {
                if c == '<' { self.pos -= 1; break; }
            }
            return self.parse_node();
        }
        // tag name
        let mut tag_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() { tag_str.push(c); self.pos += 1; } else { break; }
        }
        let Some(tag) = Tag::from_str(&tag_str) else {
            return Err(format!("Unknown tag: {tag_str}"));
        };
        // attributes
        let mut class: Option<String> = None;
        loop {
            self.skip_ws();
            if self.consume("/>") {
                // self-closing
                let id = self.next_id; self.next_id += 1;
                let style = style_from_css(class.as_ref().and_then(|c| self.css.get(c)));
                return Ok(Some(SvelteNode::Element(Element {
                    id, name: tag_str.clone(), tag, style, children: vec![],
                    visible: true, locked: false, component: false, instance: false,
                })));
            }
            if self.consume(">") { break; }
            // attr
            let mut key = String::new();
            while let Some(c) = self.peek() {
                if c.is_whitespace() || c == '=' || c == '>' { break; }
                key.push(c); self.pos += 1;
            }
            if self.consume("=") {
                let quote = self.bump().ok_or("missing quote")?;
                let mut val = String::new();
                while let Some(c) = self.bump() {
                    if c == quote { break; }
                    val.push(c);
                }
                if key == "class" {
                    class = Some(val);
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
                    if c == '>' { self.pos += 1; break; }
                    close.push(c); self.pos += 1;
                }
                break;
            }
            if self.pos >= self.chars.len() { break; }
            if self.consume("<span") {
                // span with class
                let mut cls = String::new();
                while let Some(c) = self.peek() {
                    if c == '>' { self.pos += 1; break; }
                    cls.push(c); self.pos += 1;
                }
                let cls = cls.trim()
                    .trim_start_matches("class=").trim_matches('"').trim_matches('\'')
                    .to_string();
                let mut content = String::new();
                while self.pos < self.chars.len() {
                    if self.chars[self.pos..].iter().collect::<String>().starts_with("</span>") {
                        self.pos += "</span>".len();
                        break;
                    }
                    content.push(self.chars[self.pos]);
                    self.pos += 1;
                }
                let id = self.next_id; self.next_id += 1;
                let mut w = None; let mut h = None;
                let ts = text_style_from_css(self.css.get(&cls), &mut w, &mut h);
                children.push(SvelteNode::Text(TextNode {
                    id, name: cls.clone(), content, style: ts, width: w, height: h,
                    visible: true, locked: false,
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
                if c == '<' { self.pos -= 1; break; }
            }
        }

        let id = self.next_id; self.next_id += 1;
        let style = style_from_css(class.as_ref().and_then(|c| self.css.get(c)));
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
        })))
    }
}

trait HexRgb { fn hex_rgb(self) -> u32; }
impl HexRgb for Color {
    fn hex_rgb(self) -> u32 {
        let (r, g, b, _) = self.to_rgba8();
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}