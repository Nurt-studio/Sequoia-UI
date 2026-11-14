//! Parser for Seq code → AST using `sequoia_seq`.

use crate::model::svelte_ast::*;

pub fn from_seq(src: &str) -> Result<Option<SvelteNode>, String> {
    let app = sequoia_seq::parse("design.seq", src).map_err(|e| e.to_string())?;
    let window = app.windows.first().ok_or_else(|| "Brak okna w .seq".to_string())?;
    let mut children = Vec::new();
    let mut next_id = 1;
    for node in &window.nodes {
        children.push(seq_node_to_svelte(node, &mut next_id));
    }
    if children.is_empty() {
        return Ok(None);
    }
    if children.len() == 1 {
        return Ok(Some(children.remove(0)));
    }
    Ok(Some(SvelteNode::Element(Element {
        id: next_id,
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

fn seq_node_to_svelte(node: &sequoia_seq::SeqNode, next_id: &mut u64) -> SvelteNode {
    let id = *next_id;
    *next_id += 1;
    
    if node.kind == "Text" {
        let content = node.label.clone().unwrap_or_else(|| {
            node.args.iter().find(|a| a.name.is_empty() || a.name == "content").and_then(|a| match &a.value {
                sequoia_seq::SeqValue::Str(s) => Some(s.clone()),
                _ => None,
            }).unwrap_or_else(|| "Text".to_string())
        });
        return SvelteNode::Text(TextNode {
            id,
            name: content.clone(),
            content,
            width: None,
            height: None,
            x: 0.0,
            y: 0.0,
            absolute: false,
            visible: true,
            locked: false,
            style: TextStyle::default(),
        });
    }

    let (display, flex_dir, tag) = match node.kind.as_str() {
        "Column" => (Display::Flex, FlexDir::Col, Tag::Div),
        "Row" => (Display::Flex, FlexDir::Row, Tag::Div),
        "Grid" => (Display::Grid, FlexDir::Row, Tag::Div),
        "Button" => (Display::Flex, FlexDir::Row, Tag::Button),
        _ => (Display::Flex, FlexDir::Row, Tag::Div),
    };

    let mut style = Style::default();
    style.display = display;
    style.flex_dir = flex_dir;

    for arg in &node.args {
        match arg.name.as_str() {
            "gap" => {
                if let sequoia_seq::SeqValue::Num(n) = arg.value {
                    style.gap = n;
                }
            }
            "padding" => {
                if let sequoia_seq::SeqValue::Num(n) = arg.value {
                    style.padding = Edges::all(n);
                }
            }
            "width" => {
                if let sequoia_seq::SeqValue::Num(n) = arg.value {
                    style.width = Dimension::Fixed(n);
                }
            }
            "height" => {
                if let sequoia_seq::SeqValue::Num(n) = arg.value {
                    style.height = Dimension::Fixed(n);
                }
            }
            "radius" => {
                if let sequoia_seq::SeqValue::Num(n) = arg.value {
                    style.radius = n;
                }
            }
            "fill" => {
                if let sequoia_seq::SeqValue::Str(s) | sequoia_seq::SeqValue::Color(s) = &arg.value {
                    if let Some(color) = parse_color(s) {
                        style.fill = Some(color);
                    }
                }
            }
            _ => {}
        }
    }

    let mut children = Vec::new();
    for c in &node.children {
        children.push(seq_node_to_svelte(c, next_id));
    }

    SvelteNode::Element(Element {
        id,
        name: node.label.clone().unwrap_or(node.kind.clone()),
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
        behavior: None,
        binding: None,
        hover: None,
        prototype: None,
    })
}

fn parse_color(s: &str) -> Option<sequoia_ui::prelude::Color> {
    if s.starts_with('#') {
        let hex_str = &s[1..];
        if let Ok(v) = u32::from_str_radix(hex_str, 16) {
            if hex_str.len() == 6 {
                return Some(sequoia_ui::prelude::Color::hex(v));
            } else if hex_str.len() == 8 {
                return Some(sequoia_ui::prelude::Color::hexa(v));
            }
        }
    }
    None
}
