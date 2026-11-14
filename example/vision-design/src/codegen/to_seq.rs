//! AST → Seq code. Deterministic and parseable back by `sequoia_seq`.

use crate::model::svelte_ast::*;

pub fn to_seq(root: &Option<SvelteNode>) -> String {
    let mut out = String::new();
    out.push_str("app VisionDesign {\n");
    out.push_str("  title: \"Vision Design\"\n");
    let (w, h) = root.as_ref().map(|r| match r {
        SvelteNode::Element(e) => (e.style.width.value().unwrap_or(800.0), e.style.height.value().unwrap_or(600.0)),
        SvelteNode::Text(_) => (800.0, 600.0),
    }).unwrap_or((800.0, 600.0));
    out.push_str(&format!("  size: ({:.0}, {:.0})\n", w, h));
    out.push_str("  theme: dark\n\n");
    out.push_str("  window Main {\n");
    match root {
        Some(SvelteNode::Element(e)) => {
            for child in &e.children {
                out.push_str(&render_seq_node(child, 4));
            }
        }
        Some(SvelteNode::Text(t)) => {
            out.push_str(&format!("    Text({:?})\n", t.content));
        }
        None => {
            out.push_str("    // empty\n");
        }
    }
    out.push_str("  }\n");
    out.push_str("}\n");
    out
}

fn render_seq_node(node: &SvelteNode, indent: usize) -> String {
    let spaces = " ".repeat(indent);
    match node {
        SvelteNode::Element(e) => {
            let kind = match (e.style.display, e.style.flex_dir) {
                (Display::Flex, FlexDir::Col) => "Column",
                (Display::Flex, FlexDir::Row) => "Row",
                (Display::Grid, _) => "Grid",
                _ => match e.tag {
                    Tag::Button => "Button",
                    Tag::Span | Tag::P => "Text",
                    _ => "Container",
                },
            };
            let mut args = Vec::new();
            if e.style.gap > 0.0 {
                args.push(format!("gap: {:.0}", e.style.gap));
            }
            if e.style.padding.top > 0.0 || e.style.padding.left > 0.0 {
                args.push(format!("padding: {:.0}", e.style.padding.top));
            }
            if let Some(v) = e.style.width.value() {
                args.push(format!("width: {:.0}", v));
            }
            if let Some(v) = e.style.height.value() {
                args.push(format!("height: {:.0}", v));
            }
            if let Some(f) = e.style.fill {
                args.push(format!("fill: {}", fmt_color(f)));
            }
            if e.style.radius > 0.0 {
                args.push(format!("radius: {:.0}", e.style.radius));
            }

            let args_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };

            if e.children.is_empty() {
                if kind == "Text" {
                    format!("{spaces}Text(\"{}\"{})\n", e.name, if args.is_empty() { "".to_string() } else { format!(", {}", args.join(", ")) })
                } else {
                    format!("{spaces}{kind}{args_str} {{}}\n")
                }
            } else {
                let mut out = format!("{spaces}{kind}{args_str} {{\n");
                for c in &e.children {
                    out.push_str(&render_seq_node(c, indent + 2));
                }
                out.push_str(&format!("{spaces}}}\n"));
                out
            }
        }
        SvelteNode::Text(t) => {
            format!("{spaces}Text({:?})\n", t.content)
        }
    }
}

fn fmt_color(c: sequoia_ui::prelude::Color) -> String {
    let rgba = c.to_rgba8();
    format!("#{:02X}{:02X}{:02X}", rgba.0, rgba.1, rgba.2)
}
