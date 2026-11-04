//! Document (AST) ↔ JSON serialization — without external dependencies.
//!
//! The JSON format is versioned (`"version": 1`) and mirrors the `SvelteNode`
//! tree 1:1, so a document can be moved between projects or generated
//! externally (e.g. by a script) and loaded back.

use sequoia_ui::prelude::{Color, Edges};

use crate::model::svelte_ast::*;

// ──────────────────────────────────────────────────────────────── export

pub fn to_json(root: &Option<SvelteNode>) -> String {
    let mut out = String::new();
    out.push_str("{\n  \"version\": 1,\n  \"root\": ");
    match root {
        Some(r) => write_node(r, 2, &mut out),
        None => out.push_str("null"),
    }
    out.push_str("\n}\n");
    out
}

fn indent(n: usize, out: &mut String) {
    for _ in 0..n {
        out.push_str("  ");
    }
}

fn write_kv_str(key: &str, val: &str, first: bool, pad: usize, out: &mut String) {
    if !first {
        out.push(',');
    }
    out.push('\n');
    indent(pad, out);
    out.push_str(&format!("\"{}\": {}", key, json_str(val)));
}

fn write_kv_bool(key: &str, val: bool, first: bool, pad: usize, out: &mut String) {
    if !first {
        out.push(',');
    }
    out.push('\n');
    indent(pad, out);
    out.push_str(&format!("\"{}\": {}", key, val));
}

fn write_kv_num(key: &str, val: f32, first: bool, pad: usize, out: &mut String) {
    if !first {
        out.push(',');
    }
    out.push('\n');
    indent(pad, out);
    out.push_str(&format!("\"{}\": {}", key, fmt_num(val)));
}

fn write_kv_opt(key: &str, val: Option<f32>, first: bool, pad: usize, out: &mut String) {
    if !first {
        out.push(',');
    }
    out.push('\n');
    indent(pad, out);
    match val {
        Some(v) => out.push_str(&format!("\"{}\": {}", key, fmt_num(v))),
        None => out.push_str(&format!("\"{}\": null", key)),
    }
}

fn fmt_num(v: f32) -> String {
    if v.fract() == 0.0 && v.abs() < 1e9 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn write_dim(key: &str, d: Dimension, first: bool, pad: usize, out: &mut String) {
    if !first {
        out.push(',');
    }
    out.push('\n');
    indent(pad, out);
    match d {
        Dimension::Auto => out.push_str(&format!("\"{}\": \"auto\"", key)),
        Dimension::Fill => out.push_str(&format!("\"{}\": \"fill\"", key)),
        Dimension::Fixed(v) => out.push_str(&format!("\"{}\": {}", key, fmt_num(v))),
    }
}

fn write_edges(key: &str, e: &Edges, first: bool, pad: usize, out: &mut String) {
    if !first {
        out.push(',');
    }
    out.push('\n');
    indent(pad, out);
    out.push_str(&format!(
        "\"{}\": {{",
        key
    ));
    write_kv_num("top", e.top, true, pad + 1, out);
    write_kv_num("right", e.right, false, pad + 1, out);
    write_kv_num("bottom", e.bottom, false, pad + 1, out);
    write_kv_num("left", e.left, false, pad + 1, out);
    out.push_str("\n");
    indent(pad, out);
    out.push('}');
}

fn write_node(node: &SvelteNode, pad: usize, out: &mut String) {
    match node {
        SvelteNode::Element(e) => {
            out.push('{');
            write_kv_str("type", "element", true, pad + 1, out);
            write_kv_num("id", e.id as f32, false, pad + 1, out);
            write_kv_str("name", &e.name, false, pad + 1, out);
            write_kv_str("tag", e.tag.as_str(), false, pad + 1, out);
            write_kv_bool("visible", e.visible, false, pad + 1, out);
            write_kv_bool("locked", e.locked, false, pad + 1, out);
            write_kv_bool("component", e.component, false, pad + 1, out);
            write_kv_bool("instance", e.instance, false, pad + 1, out);
            // style
            out.push_str(",\n");
            indent(pad + 1, out);
            out.push_str("\"style\": {");
            let s = &e.style;
            write_kv_str(
                "display",
                match s.display {
                    Display::Flex => "flex",
                    Display::Grid => "grid",
                    Display::None => "none",
                },
                true,
                pad + 2,
                out,
            );
            write_kv_str(
                "flex_dir",
                if s.flex_dir == FlexDir::Row { "row" } else { "column" },
                false,
                pad + 2,
                out,
            );
            match s.grid_cols {
                Some(n) => write_kv_num("grid_cols", n as f32, false, pad + 2, out),
                None => {
                    out.push_str(",\n");
                    indent(pad + 2, out);
                    out.push_str("\"grid_cols\": null");
                }
            }
            write_kv_num("gap", s.gap, false, pad + 2, out);
            write_edges("padding", &s.padding, false, pad + 2, out);
            write_kv_str(
                "align_items",
                align_name(s.align_items),
                false,
                pad + 2,
                out,
            );
            write_kv_str(
                "justify_content",
                align_name(s.justify_content),
                false,
                pad + 2,
                out,
            );
            write_kv_str(
                "position",
                match s.position {
                    Position::Absolute => "absolute",
                    Position::Relative => "relative",
                    Position::Static => "static",
                },
                false,
                pad + 2,
                out,
            );
            write_kv_opt("top", s.top, false, pad + 2, out);
            write_kv_opt("right", s.right, false, pad + 2, out);
            write_kv_opt("bottom", s.bottom, false, pad + 2, out);
            write_kv_opt("left", s.left, false, pad + 2, out);
            write_dim("width", s.width, false, pad + 2, out);
            write_dim("height", s.height, false, pad + 2, out);
            write_kv_str("fill", &s.fill_hex, false, pad + 2, out);
            write_kv_str("stroke", &s.stroke_hex, false, pad + 2, out);
            write_kv_num("stroke_width", s.stroke_width, false, pad + 2, out);
            write_kv_num("radius", s.radius, false, pad + 2, out);
            write_kv_num("opacity", s.opacity, false, pad + 2, out);
            out.push_str("\n");
            indent(pad + 1, out);
            out.push('}');
            // children
            if e.children.is_empty() {
                out.push_str(",\n");
                indent(pad + 1, out);
                out.push_str("\"children\": []");
            } else {
                out.push_str(",\n");
                indent(pad + 1, out);
                out.push_str("\"children\": [");
                for c in &e.children {
                    out.push('\n');
                    indent(pad + 2, out);
                    write_node(c, pad + 2, out);
                    out.push(',');
                }
                if out.ends_with(',') {
                    out.pop();
                }
                out.push('\n');
                indent(pad + 1, out);
                out.push(']');
            }
            out.push('\n');
            indent(pad, out);
            out.push('}');
        }
        SvelteNode::Text(t) => {
            out.push('{');
            write_kv_str("type", "text", true, pad + 1, out);
            write_kv_num("id", t.id as f32, false, pad + 1, out);
            write_kv_str("name", &t.name, false, pad + 1, out);
            write_kv_str("content", &t.content, false, pad + 1, out);
            write_kv_opt("width", t.width, false, pad + 1, out);
            write_kv_opt("height", t.height, false, pad + 1, out);
            write_kv_bool("visible", t.visible, false, pad + 1, out);
            write_kv_bool("locked", t.locked, false, pad + 1, out);
            out.push_str(",\n");
            indent(pad + 1, out);
            out.push_str("\"style\": {");
            write_kv_num("font_size", t.style.font_size, true, pad + 2, out);
            write_kv_num("font_weight", t.style.font_weight as f32, false, pad + 2, out);
            write_kv_str("color", &t.style.color_hex, false, pad + 2, out);
            write_kv_str("align", align_name(t.style.align), false, pad + 2, out);
            out.push_str("\n");
            indent(pad + 1, out);
            out.push('}');
            out.push('\n');
            indent(pad, out);
            out.push('}');
        }
    }
}

fn align_name(a: Align) -> &'static str {
    match a {
        Align::Start => "start",
        Align::Center => "center",
        Align::End => "end",
        Align::Stretch => "stretch",
        Align::SpaceBetween => "space-between",
    }
}

// ──────────────────────────────────────────────────────────────── parser

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f32),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_num(&self) -> Option<f32> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
}

pub fn from_json(src: &str) -> Result<Option<SvelteNode>, String> {
    let mut p = JsonParser {
        chars: src.chars().collect(),
        pos: 0,
    };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.chars.len() {
        return Err(format!("Unexpected data after position {}", p.pos));
    }
    if value.get("version").and_then(|v| v.as_num()) != Some(1.0) {
        return Err("Missing \"version\": 1 field — this is not a Vision Design document".into());
    }
    match value.get("root") {
        None | Some(Json::Null) => Ok(None),
        Some(node) => Ok(Some(parse_node(node)?)),
    }
}

struct JsonParser {
    chars: Vec<char>,
    pos: usize,
}

impl JsonParser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
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
    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!(
                "Expected '{}', found {:?} (position {})",
                c,
                self.peek(),
                self.pos
            ))
        }
    }

    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.parse_obj(),
            Some('[') => self.parse_arr(),
            Some('"') => Ok(Json::Str(self.parse_string()?)),
            Some('t') => self.parse_lit("true", Json::Bool(true)),
            Some('f') => self.parse_lit("false", Json::Bool(false)),
            Some('n') => self.parse_lit("null", Json::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_num(),
            Some(c) => Err(format!("Unexpected character '{}' (position {})", c, self.pos)),
            None => Err("Unexpected end of input".into()),
        }
    }

    fn parse_lit(&mut self, lit: &str, val: Json) -> Result<Json, String> {
        for ch in lit.chars() {
            self.expect(ch)?;
        }
        Ok(val)
    }

    fn parse_num(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        s.parse::<f32>()
            .map(Json::Num)
            .map_err(|_| format!("Invalid number: '{s}'"))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err("Unexpected end of string".into()),
                Some('"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some('\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some('/') => out.push('/'),
                        Some('n') => out.push('\n'),
                        Some('r') => out.push('\r'),
                        Some('t') => out.push('\t'),
                        Some('b') => out.push('\u{8}'),
                        Some('f') => out.push('\u{c}'),
                        Some('u') => {
                            self.pos += 1;
                            let mut code = 0u32;
                            for _ in 0..4 {
                                let c = self.peek().ok_or("Incomplete \\u sequence")?;
                                code = code * 16
                                    + c.to_digit(16).ok_or("Invalid hexadecimal digit")?;
                                self.pos += 1;
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                            continue;
                        }
                        other => return Err(format!("Unknown escape sequence \\{other:?}")),
                    }
                    self.pos += 1;
                }
                Some(c) => {
                    out.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    fn parse_arr(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                }
                Some(']') => {
                    self.pos += 1;
                    return Ok(Json::Arr(items));
                }
                other => return Err(format!("Expected ',' or ']', found {other:?}")),
            }
        }
    }

    fn parse_obj(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(Json::Obj(pairs));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(':')?;
            let val = self.parse_value()?;
            pairs.push((key, val));
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                }
                Some('}') => {
                    self.pos += 1;
                    return Ok(Json::Obj(pairs));
                }
                other => return Err(format!("Expected ',' or '}}', found {other:?}")),
            }
        }
    }
}

// ──────────────────────────────────────────────────────── JSON → AST

fn parse_node(v: &Json) -> Result<SvelteNode, String> {
    let id = v
        .get("id")
        .and_then(|x| x.as_num())
        .ok_or("Missing \"id\" field")? as u64;
    let name = v
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or("Node")
        .to_string();
    match v.get("type").and_then(|x| x.as_str()) {
        Some("text") => {
            let style_v = v.get("style").ok_or("Text node is missing the \"style\" field")?;
            let color_hex = style_v
                .get("color")
                .and_then(|x| x.as_str())
                .unwrap_or("#FFFFFF")
                .to_string();
            let color = Color::parse_hex(&color_hex).unwrap_or(Color::WHITE);
            Ok(SvelteNode::Text(TextNode {
                id,
                name,
                content: v
                    .get("content")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                style: TextStyle {
                    font_size: style_v.get("font_size").and_then(|x| x.as_num()).unwrap_or(14.0),
                    font_weight: style_v
                        .get("font_weight")
                        .and_then(|x| x.as_num())
                        .unwrap_or(400.0) as u16,
                    color,
                    color_hex,
                    align: style_v
                        .get("align")
                        .and_then(|x| x.as_str())
                        .map(align_from_name)
                        .unwrap_or(Align::Start),
                },
                width: opt_num(v.get("width")),
                height: opt_num(v.get("height")),
                visible: v.get("visible").and_then(|x| x.as_bool()).unwrap_or(true),
                locked: v.get("locked").and_then(|x| x.as_bool()).unwrap_or(false),
            }))
        }
        Some("element") | None => {
            let tag = v
                .get("tag")
                .and_then(|x| x.as_str())
                .and_then(Tag::from_str)
                .unwrap_or(Tag::Div);
            let style = match v.get("style") {
                Some(sv) => parse_style(sv)?,
                None => Style::default(),
            };
            let mut children = Vec::new();
            if let Some(arr) = v.get("children").and_then(|x| x.as_arr()) {
                for c in arr {
                    children.push(parse_node(c)?);
                }
            }
            Ok(SvelteNode::Element(Element {
                id,
                name,
                tag,
                style,
                children,
                visible: v.get("visible").and_then(|x| x.as_bool()).unwrap_or(true),
                locked: v.get("locked").and_then(|x| x.as_bool()).unwrap_or(false),
                component: v.get("component").and_then(|x| x.as_bool()).unwrap_or(false),
                instance: v.get("instance").and_then(|x| x.as_bool()).unwrap_or(false),
            }))
        }
        Some(other) => Err(format!("Unknown node type: '{other}'")),
    }
}

fn opt_num(v: Option<&Json>) -> Option<f32> {
    match v {
        Some(Json::Num(n)) => Some(*n),
        _ => None,
    }
}

fn parse_dim(v: Option<&Json>) -> Dimension {
    match v {
        Some(Json::Str(s)) if s == "fill" => Dimension::Fill,
        Some(Json::Str(s)) if s == "auto" => Dimension::Auto,
        Some(Json::Num(n)) => Dimension::Fixed(*n),
        _ => Dimension::Auto,
    }
}

fn align_from_name(s: &str) -> Align {
    match s {
        "center" => Align::Center,
        "end" => Align::End,
        "stretch" => Align::Stretch,
        "space-between" => Align::SpaceBetween,
        _ => Align::Start,
    }
}

fn parse_color_opt(v: Option<&Json>) -> Option<Color> {
    let hex = v.and_then(|x| x.as_str())?;
    let color = Color::parse_hex(hex)?;
    // alpha 0 = no fill / stroke (matching the editor convention)
    if color.to_rgba8().3 == 0 {
        None
    } else {
        Some(color)
    }
}

fn parse_style(v: &Json) -> Result<Style, String> {
    let display = match v.get("display").and_then(|x| x.as_str()) {
        Some("flex") => Display::Flex,
        Some("grid") => Display::Grid,
        _ => Display::None,
    };
    let flex_dir = if v.get("flex_dir").and_then(|x| x.as_str()) == Some("column") {
        FlexDir::Col
    } else {
        FlexDir::Row
    };
    let padding_v = v.get("padding").cloned().unwrap_or(Json::Obj(Vec::new()));
    let edge = |k: &str| padding_v.get(k).and_then(|x| x.as_num()).unwrap_or(0.0);
    let fill_hex = v
        .get("fill")
        .and_then(|x| x.as_str())
        .unwrap_or("#00000000")
        .to_string();
    let stroke_hex = v
        .get("stroke")
        .and_then(|x| x.as_str())
        .unwrap_or("#00000000")
        .to_string();

    Ok(Style {
        display,
        flex_dir,
        grid_cols: v
            .get("grid_cols")
            .and_then(|x| x.as_num())
            .map(|n| n as u16),
        gap: v.get("gap").and_then(|x| x.as_num()).unwrap_or(0.0),
        padding: Edges::only(edge("left"), edge("top"), edge("right"), edge("bottom")),
        align_items: v
            .get("align_items")
            .and_then(|x| x.as_str())
            .map(align_from_name)
            .unwrap_or(Align::Start),
        justify_content: v
            .get("justify_content")
            .and_then(|x| x.as_str())
            .map(align_from_name)
            .unwrap_or(Align::Start),
        position: match v.get("position").and_then(|x| x.as_str()) {
            Some("absolute") => Position::Absolute,
            Some("static") => Position::Static,
            _ => Position::Relative,
        },
        top: opt_num(v.get("top")),
        right: opt_num(v.get("right")),
        bottom: opt_num(v.get("bottom")),
        left: opt_num(v.get("left")),
        width: parse_dim(v.get("width")),
        height: parse_dim(v.get("height")),
        fill: parse_color_opt(v.get("fill")),
        fill_hex,
        stroke: parse_color_opt(v.get("stroke")),
        stroke_hex,
        stroke_width: v.get("stroke_width").and_then(|x| x.as_num()).unwrap_or(0.0),
        radius: v.get("radius").and_then(|x| x.as_num()).unwrap_or(0.0),
        opacity: v.get("opacity").and_then(|x| x.as_num()).unwrap_or(100.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::demo::demo_tree;

    #[test]
    fn demo_tree_roundtrip() {
        let root = Some(demo_tree());
        let json = to_json(&root);
        let back = from_json(&json).expect("JSON parsing");
        assert_eq!(root, back, "AST must be identical after a JSON round trip");
    }

    #[test]
    fn empty_document_roundtrip() {
        let json = to_json(&None);
        assert!(from_json(&json).unwrap().is_none());
    }

    #[test]
    fn rejects_garbage() {
        assert!(from_json("not json").is_err());
        assert!(from_json("{}").is_err(), "missing version field");
    }

    #[test]
    fn escapes_special_chars() {
        assert_eq!(json_str("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    }
}
