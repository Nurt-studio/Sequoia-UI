//! Document (AST) ↔ JSON serialization — without external dependencies.
//!
//! The JSON format is versioned (`"version": 1`) and mirrors the `SvelteNode`
//! tree 1:1, so a document can be moved between projects or generated
//! externally (e.g. by a script) and loaded back.

use sequoia_ui::prelude::{Color, Edges};

use crate::model::behavior::{
    Action, Behavior, Binding, HoverStyle, Rule, Transition, Trigger, Value, ValueExpr, Var,
    VarType,
};
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
    out.push_str(&format!("\"{}\": {{", key));
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
            // Component contract — written only when present, so documents
            // without components stay byte-identical to the old format.
            if let Some(it) = &e.interface {
                out.push_str(",\n");
                indent(pad + 1, out);
                write_interface(it, pad + 1, out);
            }
            if !e.overrides.is_empty() {
                out.push_str(",\n");
                indent(pad + 1, out);
                out.push_str("\"overrides\": [");
                for o in &e.overrides {
                    out.push_str("\n");
                    indent(pad + 2, out);
                    out.push('{');
                    write_kv_str("prop", &o.prop, true, pad + 3, out);
                    write_kv_str("value", &o.value, false, pad + 3, out);
                    out.push('\n');
                    indent(pad + 2, out);
                    out.push('}');
                    out.push(',');
                }
                if out.ends_with(',') {
                    out.pop();
                }
                out.push('\n');
                indent(pad + 1, out);
                out.push(']');
            }
            // Phase 4 slot content — written only when present.
            if !e.slot_content.is_empty() {
                out.push_str(",\n");
                indent(pad + 1, out);
                out.push_str("\"slot_content\": [");
                for sc in &e.slot_content {
                    out.push_str("\n");
                    indent(pad + 2, out);
                    out.push('{');
                    write_kv_str("slot", &sc.slot, true, pad + 3, out);
                    out.push_str(",\n");
                    indent(pad + 3, out);
                    out.push_str("\"nodes\": [");
                    for n in &sc.nodes {
                        out.push_str("\n");
                        indent(pad + 4, out);
                        write_node(n, pad + 4, out);
                        out.push(',');
                    }
                    if out.ends_with(',') {
                        out.pop();
                    }
                    if !sc.nodes.is_empty() {
                        out.push('\n');
                        indent(pad + 3, out);
                    }
                    out.push(']');
                    out.push('\n');
                    indent(pad + 2, out);
                    out.push('}');
                    out.push(',');
                }
                if out.ends_with(',') {
                    out.pop();
                }
                out.push('\n');
                indent(pad + 1, out);
                out.push(']');
            }
            // Phase 2 behavior — written only when present, so documents
            // without behaviors stay byte-identical to the old format.
            if let Some(b) = &e.behavior {
                if !b.is_empty() {
                    out.push_str(",\n");
                    indent(pad + 1, out);
                    out.push_str("\"behavior\": ");
                    write_behavior(b, pad + 1, out);
                }
            }
            if let Some(b) = &e.binding {
                out.push_str(",\n");
                indent(pad + 1, out);
                out.push_str("\"binding\": ");
                write_binding(b, out);
            }
            if let Some(h) = &e.hover {
                if !h.is_empty() {
                    out.push_str(",\n");
                    indent(pad + 1, out);
                    out.push_str("\"hover\": ");
                    write_hover(h, out);
                }
            }
            if let Some(p) = &e.prototype {
                out.push_str(",\n");
                indent(pad + 1, out);
                out.push_str("\"prototype\": ");
                write_prototype(p, out);
            }
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
                if s.flex_dir == FlexDir::Row {
                    "row"
                } else {
                    "column"
                },
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
            match s.grid_rows {
                Some(n) => write_kv_num("grid_rows", n as f32, false, pad + 2, out),
                None => {
                    out.push_str(",\n");
                    indent(pad + 2, out);
                    out.push_str("\"grid_rows\": null");
                }
            }
            write_kv_num("gap", s.gap, false, pad + 2, out);
            write_kv_num("row_gap", s.row_gap, false, pad + 2, out);
            write_kv_str(
                "flex_wrap",
                match s.flex_wrap {
                    Wrap::Wrap => "wrap",
                    Wrap::NoWrap => "nowrap",
                },
                false,
                pad + 2,
                out,
            );
            write_kv_str(
                "align_content",
                align_name(s.align_content),
                false,
                pad + 2,
                out,
            );
            write_edges("padding", &s.padding, false, pad + 2, out);
            write_kv_str(
                "align_items",
                align_name(s.align_items),
                false,
                pad + 2,
                out,
            );
            match s.align_self {
                Some(a) => write_kv_str("align_self", align_name(a), false, pad + 2, out),
                None => {
                    out.push_str(",\n");
                    indent(pad + 2, out);
                    out.push_str("\"align_self\": null");
                }
            }
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
            if let Some(g) = &s.gradient {
                out.push_str(",\n");
                indent(pad + 2, out);
                out.push_str("\"gradient\": ");
                write_gradient(g, out);
            }
            if !s.effects.is_empty() {
                out.push_str(",\n");
                indent(pad + 2, out);
                out.push_str("\"effects\": [");
                for (i, e) in s.effects.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_shadow(e, out);
                }
                out.push(']');
            }
            if s.blur > 0.0 {
                write_kv_num("blur", s.blur, false, pad + 2, out);
            }
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
            write_kv_num("x", t.x, false, pad + 1, out);
            write_kv_num("y", t.y, false, pad + 1, out);
            write_kv_bool("absolute", t.absolute, false, pad + 1, out);
            write_kv_bool("visible", t.visible, false, pad + 1, out);
            write_kv_bool("locked", t.locked, false, pad + 1, out);
            out.push_str(",\n");
            indent(pad + 1, out);
            out.push_str("\"style\": {");
            write_kv_num("font_size", t.style.font_size, true, pad + 2, out);
            write_kv_num(
                "font_weight",
                t.style.font_weight as f32,
                false,
                pad + 2,
                out,
            );
            if t.style.opacity < 100.0 {
                write_kv_num("opacity", t.style.opacity, false, pad + 2, out);
            }
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

fn write_gradient(g: &GradientDef, out: &mut String) {
    out.push_str(&format!(
        "{{\"kind\":{},\"angle\":{},\"stops\":[",
        json_str(g.kind.as_str()),
        fmt_num(g.angle)
    ));
    for (i, s) in g.stops.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"color\":{},\"at\":{}}}",
            json_str(&s.hex),
            fmt_num(s.at)
        ));
    }
    out.push_str("]}");
}

fn write_shadow(e: &ShadowEffect, out: &mut String) {
    out.push_str(&format!(
        "{{\"color\":{},\"x\":{},\"y\":{},\"blur\":{},\"spread\":{},\"enabled\":{}}}",
        json_str(&e.hex),
        fmt_num(e.x),
        fmt_num(e.y),
        fmt_num(e.blur),
        fmt_num(e.spread),
        e.enabled
    ));
}

fn write_prototype(p: &PrototypeLink, out: &mut String) {
    out.push_str(&format!(
        "{{\"on\":{},\"action\":{},\"target\":{},\"transition\":{},\"ms\":{}}}",
        json_str(p.trigger.as_str()),
        json_str(p.action.as_str()),
        p.target
            .map(|t| t.to_string())
            .unwrap_or_else(|| "null".into()),
        json_str(p.transition.as_str()),
        p.duration_ms
    ));
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

/// `"interface": { props, events, slots }` — the master's contract.
fn write_interface(it: &ComponentInterface, pad: usize, out: &mut String) {
    out.push_str("\"interface\": {");
    // props
    out.push_str("\n");
    indent(pad + 1, out);
    out.push_str("\"props\": [");
    for p in &it.props {
        out.push_str("\n");
        indent(pad + 2, out);
        out.push('{');
        write_kv_str("name", &p.name, true, pad + 3, out);
        write_kv_str("default", &p.default, false, pad + 3, out);
        write_kv_str("kind", p.kind.as_str(), false, pad + 3, out);
        write_kv_num("target", p.target as f32, false, pad + 3, out);
        out.push('\n');
        indent(pad + 2, out);
        out.push('}');
        out.push(',');
    }
    if out.ends_with(',') {
        out.pop();
    }
    if !it.props.is_empty() {
        out.push('\n');
        indent(pad + 1, out);
    }
    out.push_str("],");
    // events
    out.push_str("\n");
    indent(pad + 1, out);
    out.push_str("\"events\": [");
    for (i, ev) in it.events.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&json_str(ev));
    }
    out.push_str("],");
    // slots
    out.push_str("\n");
    indent(pad + 1, out);
    out.push_str("\"slots\": [");
    for s in &it.slots {
        out.push_str("\n");
        indent(pad + 2, out);
        out.push('{');
        write_kv_str("name", &s.name, true, pad + 3, out);
        write_kv_num("container", s.container as f32, false, pad + 3, out);
        out.push('\n');
        indent(pad + 2, out);
        out.push('}');
        out.push(',');
    }
    if out.ends_with(',') {
        out.pop();
    }
    if !it.slots.is_empty() {
        out.push('\n');
        indent(pad + 1, out);
    }
    out.push(']');
    out.push('\n');
    indent(pad, out);
    out.push('}');
}

// ──────────────────────────────── Phase 2: behavior serialization
//
// One JSON shape serves both the document format (pretty) and the compact
// `vd-behavior` payload embedded in generated Svelte (design doc §6.3):
//   var       {"name":"open","ty":"bool","init":false}
//   trigger   {"on":"click"}   {"on":"key","key":"Enter"}
//   action    {"act":"toggle","var":"open"}   {"act":"show","target":"Panel"}
//   expr      {"expr":"value","val":3}  {"expr":"var","name":"x"}  {"expr":"payload"}

fn write_value(v: &Value, out: &mut String) {
    match v {
        Value::Bool(b) => out.push_str(&b.to_string()),
        Value::Number(n) => out.push_str(&fmt_num(*n)),
        Value::Str(s) => out.push_str(&json_str(s)),
    }
}

fn write_value_expr(e: &ValueExpr, out: &mut String) {
    out.push('{');
    match e {
        ValueExpr::Value(v) => {
            out.push_str("\"expr\":\"value\",\"val\":");
            write_value(v, out);
        }
        ValueExpr::Var(name) => {
            out.push_str("\"expr\":\"var\",\"name\":");
            out.push_str(&json_str(name));
        }
        ValueExpr::Payload => out.push_str("\"expr\":\"payload\""),
    }
    out.push('}');
}

fn write_transition(t: Transition, out: &mut String) {
    out.push_str(&format!(
        "{{\"kind\":{},\"ms\":{}}}",
        json_str(t.kind_name()),
        t.ms()
    ));
}

fn write_trigger(t: &Trigger, out: &mut String) {
    out.push_str(&format!("\"on\":{}", json_str(t.kind_name())));
    if let Some(p) = t.param() {
        let key = match t {
            Trigger::OnKey(_) => "key",
            Trigger::OnPropChanged(_) => "prop",
            _ => "event",
        };
        out.push_str(&format!(",\"{}\":{}", key, json_str(p)));
    }
}

fn write_action(a: &Action, out: &mut String) {
    out.push('{');
    match a {
        Action::ToggleVar(v) => {
            out.push_str(&format!("\"act\":\"toggle\",\"var\":{}", json_str(v)));
        }
        Action::SetVar(v, e) => {
            out.push_str(&format!(
                "\"act\":\"set\",\"var\":{},\"expr\":",
                json_str(v)
            ));
            write_value_expr(e, out);
        }
        Action::Show { target, transition } => {
            out.push_str(&format!("\"act\":\"show\",\"target\":{}", json_str(target)));
            if let Some(t) = transition {
                out.push_str(",\"transition\":");
                write_transition(*t, out);
            }
        }
        Action::Hide { target, transition } => {
            out.push_str(&format!("\"act\":\"hide\",\"target\":{}", json_str(target)));
            if let Some(t) = transition {
                out.push_str(",\"transition\":");
                write_transition(*t, out);
            }
        }
        Action::Emit(e) => {
            out.push_str(&format!("\"act\":\"emit\",\"event\":{}", json_str(e)));
        }
        Action::Delay(ms, actions) => {
            out.push_str(&format!("\"act\":\"delay\",\"ms\":{},\"do\":", ms));
            write_action_list(actions, out);
        }
    }
    out.push('}');
}

fn write_action_list(actions: &[Action], out: &mut String) {
    out.push('[');
    for (i, a) in actions.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_action(a, out);
    }
    out.push(']');
}

/// `"behavior": { … }` — pretty form used inside document JSON. Writes the
/// value only; the key and indentation are handled by the caller.
fn write_behavior(b: &Behavior, pad: usize, out: &mut String) {
    out.push('{');
    out.push('\n');
    indent(pad + 1, out);
    out.push_str("\"vars\": [");
    for v in &b.vars {
        out.push('\n');
        indent(pad + 2, out);
        out.push('{');
        out.push_str(&format!(
            "\"name\":{},\"ty\":{},\"init\":",
            json_str(&v.name),
            json_str(v.ty.as_str())
        ));
        write_value(&v.initial, out);
        out.push('}');
        out.push(',');
    }
    if out.ends_with(',') {
        out.pop();
    }
    if !b.vars.is_empty() {
        out.push('\n');
        indent(pad + 1, out);
    }
    out.push_str("],");
    out.push('\n');
    indent(pad + 1, out);
    out.push_str("\"rules\": [");
    for r in &b.rules {
        out.push('\n');
        indent(pad + 2, out);
        out.push('{');
        write_trigger(&r.trigger, out);
        out.push_str(",\"do\": ");
        write_action_list(&r.actions, out);
        out.push('}');
        out.push(',');
    }
    if out.ends_with(',') {
        out.pop();
    }
    if !b.rules.is_empty() {
        out.push('\n');
        indent(pad + 1, out);
    }
    out.push(']');
    out.push('\n');
    indent(pad, out);
    out.push('}');
}

fn write_binding(b: &Binding, out: &mut String) {
    out.push('{');
    out.push_str(&format!("\"var\":{}", json_str(&b.var)));
    if let Some(t) = b.transition {
        out.push_str(",\"transition\":");
        write_transition(t, out);
    }
    out.push('}');
}

fn write_hover(h: &HoverStyle, out: &mut String) {
    out.push('{');
    let mut first = true;
    let mut kv = |out: &mut String, key: &str, val: &str| {
        if !first {
            out.push(',');
        }
        out.push_str(&format!("\"{}\":{}", key, json_str(val)));
        first = false;
    };
    if let Some(f) = &h.fill {
        kv(out, "fill", f);
    }
    if let Some(c) = &h.color {
        kv(out, "color", c);
    }
    if let Some(o) = h.opacity {
        if !first {
            out.push(',');
        }
        out.push_str(&format!("\"opacity\":{}", fmt_num(o)));
    }
    out.push('}');
}

/// Compact single-line payload for the `<!-- vd-behavior: … -->` comment:
/// `{"behavior":…,"binding":…,"hover":…}` with empty parts omitted.
/// `None` when the element carries no Phase 2 data.
pub fn vd_payload(e: &Element) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(b) = &e.behavior {
        if !b.is_empty() {
            let mut s = String::new();
            write_behavior_compact(b, &mut s);
            parts.push(format!("\"behavior\":{}", s));
        }
    }
    if let Some(b) = &e.binding {
        let mut s = String::new();
        write_binding(b, &mut s);
        parts.push(format!("\"binding\":{}", s));
    }
    if let Some(h) = &e.hover {
        if !h.is_empty() {
            let mut s = String::new();
            write_hover(h, &mut s);
            parts.push(format!("\"hover\":{}", s));
        }
    }
    if let Some(p) = &e.prototype {
        let mut s = String::new();
        write_prototype(p, &mut s);
        parts.push(format!("\"prototype\":{}", s));
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("{{{}}}", parts.join(",")))
}

fn write_behavior_compact(b: &Behavior, out: &mut String) {
    out.push('{');
    out.push_str("\"vars\":[");
    for (i, v) in b.vars.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"name\":{},\"ty\":{},\"init\":",
            json_str(&v.name),
            json_str(v.ty.as_str())
        ));
        write_value(&v.initial, out);
        out.push('}');
    }
    out.push_str("],\"rules\":[");
    for (i, r) in b.rules.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        write_trigger(&r.trigger, out);
        out.push_str(",\"do\":");
        write_action_list(&r.actions, out);
        out.push('}');
    }
    out.push_str("]}");
}

// ──────────────────────────────────────────────────────────────── parser

/// Parses a standalone JSON value — used by the `vd-behavior` comment
/// payload and by external tooling.
pub fn parse_json(src: &str) -> Result<Json, String> {
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
    Ok(value)
}

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
            Some(c) => Err(format!(
                "Unexpected character '{}' (position {})",
                c, self.pos
            )),
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
            let style_v = v
                .get("style")
                .ok_or("Text node is missing the \"style\" field")?;
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
                    font_size: style_v
                        .get("font_size")
                        .and_then(|x| x.as_num())
                        .unwrap_or(14.0),
                    font_weight: style_v
                        .get("font_weight")
                        .and_then(|x| x.as_num())
                        .unwrap_or(400.0) as u16,
                    opacity: style_v
                        .get("opacity")
                        .and_then(|x| x.as_num())
                        .unwrap_or(100.0),
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
                x: v.get("x").and_then(|x| x.as_num()).unwrap_or(0.0),
                y: v.get("y").and_then(|x| x.as_num()).unwrap_or(0.0),
                absolute: v.get("absolute").and_then(|x| x.as_bool()).unwrap_or(false),
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
                component: v
                    .get("component")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
                instance: v.get("instance").and_then(|x| x.as_bool()).unwrap_or(false),
                interface: v.get("interface").and_then(parse_interface),
                overrides: v
                    .get("overrides")
                    .and_then(|x| x.as_arr())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|o| {
                                Some(Override {
                                    prop: o.get("prop")?.as_str()?.to_string(),
                                    value: o.get("value")?.as_str()?.to_string(),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                slot_content: v
                    .get("slot_content")
                    .and_then(|x| x.as_arr())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|sc| {
                                let mut nodes = Vec::new();
                                if let Some(list) = sc.get("nodes").and_then(|x| x.as_arr()) {
                                    for n in list {
                                        nodes.push(parse_node(n).ok()?);
                                    }
                                }
                                Some(SlotContent {
                                    slot: sc.get("slot")?.as_str()?.to_string(),
                                    nodes,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                behavior: v.get("behavior").and_then(parse_behavior),
                binding: v.get("binding").and_then(parse_binding),
                hover: v.get("hover").and_then(parse_hover),
                prototype: v.get("prototype").and_then(parse_prototype),
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

fn parse_interface(v: &Json) -> Option<ComponentInterface> {
    let props = v
        .get("props")
        .and_then(|x| x.as_arr())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    Some(PropDef {
                        name: p.get("name")?.as_str()?.to_string(),
                        default: p
                            .get("default")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                        kind: p
                            .get("kind")
                            .and_then(|x| x.as_str())
                            .and_then(PropKind::from_str)
                            .unwrap_or(PropKind::Text),
                        target: p.get("target").and_then(|x| x.as_num()).unwrap_or(0.0) as u64,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let events = v
        .get("events")
        .and_then(|x| x.as_arr())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| e.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let slots = v
        .get("slots")
        .and_then(|x| x.as_arr())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    Some(SlotDef {
                        name: s.get("name")?.as_str()?.to_string(),
                        container: s.get("container").and_then(|x| x.as_num()).unwrap_or(0.0)
                            as u64,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(ComponentInterface {
        props,
        events,
        slots,
    })
}

// ─────────────────────────── Phase 2: behavior parsing

/// Decoded `<!-- vd-behavior: … -->` payload (design doc §6.3): the parts
/// the code comment carries for one element.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VdPayload {
    pub behavior: Option<Behavior>,
    pub binding: Option<Binding>,
    pub hover: Option<HoverStyle>,
    pub prototype: Option<PrototypeLink>,
}

impl VdPayload {
    pub fn is_empty(&self) -> bool {
        self.behavior.is_none()
            && self.binding.is_none()
            && self.hover.is_none()
            && self.prototype.is_none()
    }
}

/// `{"behavior":…,"binding":…,"hover":…}` → [`VdPayload`]; unknown parts are
/// ignored so older exports keep loading.
pub fn vd_payload_from_json(v: &Json) -> VdPayload {
    VdPayload {
        behavior: v.get("behavior").and_then(parse_behavior),
        binding: v.get("binding").and_then(parse_binding),
        hover: v.get("hover").and_then(parse_hover),
        prototype: v.get("prototype").and_then(parse_prototype),
    }
}

fn parse_value_typed(v: Option<&Json>) -> Option<Value> {
    match v? {
        Json::Bool(b) => Some(Value::Bool(*b)),
        Json::Num(n) => Some(Value::Number(*n)),
        Json::Str(s) => Some(Value::Str(s.clone())),
        _ => None,
    }
}

fn parse_value_expr(v: &Json) -> Option<ValueExpr> {
    match v.get("expr")?.as_str()? {
        "value" => Some(ValueExpr::Value(parse_value_typed(v.get("val"))?)),
        "var" => Some(ValueExpr::Var(v.get("name")?.as_str()?.to_string())),
        "payload" => Some(ValueExpr::Payload),
        _ => None,
    }
}

fn parse_transition(v: &Json) -> Option<Transition> {
    let kind = v.get("kind")?.as_str()?;
    let ms = v.get("ms").and_then(|x| x.as_num()).unwrap_or(200.0) as u32;
    Transition::from_kind(kind, ms)
}

fn parse_trigger(v: &Json) -> Option<Trigger> {
    Some(match v.get("on")?.as_str()? {
        "click" => Trigger::OnClick,
        "hover_in" => Trigger::OnHoverIn,
        "hover_out" => Trigger::OnHoverOut,
        "mount" => Trigger::OnMount,
        "key" => Trigger::OnKey(
            v.get("key")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        ),
        "prop" => Trigger::OnPropChanged(
            v.get("prop")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        ),
        "event" => Trigger::OnEvent(
            v.get("event")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        ),
        _ => return None,
    })
}

fn parse_action(v: &Json) -> Option<Action> {
    Some(match v.get("act")?.as_str()? {
        "toggle" => Action::ToggleVar(v.get("var")?.as_str()?.to_string()),
        "set" => Action::SetVar(
            v.get("var")?.as_str()?.to_string(),
            v.get("expr")
                .and_then(parse_value_expr)
                .unwrap_or(ValueExpr::Value(Value::Bool(false))),
        ),
        "show" => Action::Show {
            target: v.get("target")?.as_str()?.to_string(),
            transition: v.get("transition").and_then(parse_transition),
        },
        "hide" => Action::Hide {
            target: v.get("target")?.as_str()?.to_string(),
            transition: v.get("transition").and_then(parse_transition),
        },
        "emit" => Action::Emit(v.get("event")?.as_str()?.to_string()),
        "delay" => Action::Delay(
            v.get("ms").and_then(|x| x.as_num()).unwrap_or(0.0) as u32,
            v.get("do")
                .and_then(|x| x.as_arr())
                .map(|arr| arr.iter().filter_map(parse_action).collect())
                .unwrap_or_default(),
        ),
        _ => return None,
    })
}

fn parse_behavior(v: &Json) -> Option<Behavior> {
    let vars = v
        .get("vars")
        .and_then(|x| x.as_arr())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    let ty = item
                        .get("ty")
                        .and_then(|x| x.as_str())
                        .and_then(VarType::from_str)
                        .unwrap_or(VarType::Bool);
                    let initial =
                        parse_value_typed(item.get("init")).unwrap_or_else(|| ty.default_value());
                    Some(Var { name, ty, initial })
                })
                .collect()
        })
        .unwrap_or_default();
    let rules = v
        .get("rules")
        .and_then(|x| x.as_arr())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    Some(Rule {
                        trigger: parse_trigger(item)?,
                        actions: item
                            .get("do")
                            .and_then(|x| x.as_arr())
                            .map(|a| a.iter().filter_map(parse_action).collect())
                            .unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Behavior { vars, rules })
}

fn parse_binding(v: &Json) -> Option<Binding> {
    Some(Binding {
        var: v.get("var")?.as_str()?.to_string(),
        transition: v.get("transition").and_then(parse_transition),
    })
}

fn parse_hover(v: &Json) -> Option<HoverStyle> {
    let fill = v.get("fill").and_then(|x| x.as_str()).map(String::from);
    let color = v.get("color").and_then(|x| x.as_str()).map(String::from);
    let opacity = v.get("opacity").and_then(|x| x.as_num());
    if fill.is_none() && color.is_none() && opacity.is_none() {
        return None;
    }
    Some(HoverStyle {
        fill,
        color,
        opacity,
    })
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
        grid_rows: v
            .get("grid_rows")
            .and_then(|x| x.as_num())
            .map(|n| n as u16),
        gap: v.get("gap").and_then(|x| x.as_num()).unwrap_or(0.0),
        row_gap: v.get("row_gap").and_then(|x| x.as_num()).unwrap_or(0.0),
        flex_wrap: if v.get("flex_wrap").and_then(|x| x.as_str()) == Some("wrap") {
            Wrap::Wrap
        } else {
            Wrap::NoWrap
        },
        align_content: v
            .get("align_content")
            .and_then(|x| x.as_str())
            .map(align_from_name)
            .unwrap_or(Align::Start),
        padding: Edges::only(edge("left"), edge("top"), edge("right"), edge("bottom")),
        align_items: v
            .get("align_items")
            .and_then(|x| x.as_str())
            .map(align_from_name)
            .unwrap_or(Align::Start),
        align_self: v
            .get("align_self")
            .and_then(|x| x.as_str())
            .and_then(Align::from_self),
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
        stroke_width: v
            .get("stroke_width")
            .and_then(|x| x.as_num())
            .unwrap_or(0.0),
        radius: v.get("radius").and_then(|x| x.as_num()).unwrap_or(0.0),
        opacity: v.get("opacity").and_then(|x| x.as_num()).unwrap_or(100.0),
        gradient: v.get("gradient").and_then(parse_gradient),
        effects: v
            .get("effects")
            .and_then(|x| x.as_arr())
            .map(|arr| arr.iter().filter_map(parse_shadow).collect())
            .unwrap_or_default(),
        blur: v.get("blur").and_then(|x| x.as_num()).unwrap_or(0.0),
    })
}

fn parse_gradient(v: &Json) -> Option<GradientDef> {
    let kind = GradientKind::from_str(v.get("kind")?.as_str()?)?;
    let angle = v.get("angle").and_then(|x| x.as_num()).unwrap_or(90.0);
    let stops: Vec<GradientStopDef> = v
        .get("stops")?
        .as_arr()?
        .iter()
        .filter_map(|s| {
            let hex = s.get("color")?.as_str()?.to_string();
            Some(GradientStopDef {
                color: Color::parse_hex(&hex)?,
                hex,
                at: s.get("at")?.as_num()?,
            })
        })
        .collect();
    if stops.len() < 2 {
        return None;
    }
    Some(GradientDef { kind, angle, stops })
}

fn parse_shadow(v: &Json) -> Option<ShadowEffect> {
    let hex = v.get("color")?.as_str()?.to_string();
    Some(ShadowEffect {
        color: Color::parse_hex(&hex)?,
        hex,
        x: v.get("x").and_then(|x| x.as_num()).unwrap_or(0.0),
        y: v.get("y").and_then(|x| x.as_num()).unwrap_or(0.0),
        blur: v.get("blur").and_then(|x| x.as_num()).unwrap_or(0.0),
        spread: v.get("spread").and_then(|x| x.as_num()).unwrap_or(0.0),
        enabled: v.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true),
    })
}

fn parse_prototype(v: &Json) -> Option<PrototypeLink> {
    Some(PrototypeLink {
        trigger: ProtoTrigger::from_str(v.get("on")?.as_str()?)?,
        action: ProtoAction::from_str(v.get("action")?.as_str()?)?,
        target: v.get("target").and_then(|x| x.as_num()).map(|n| n as u64),
        transition: ProtoTransition::from_str(v.get("transition")?.as_str()?)?,
        duration_ms: v.get("ms").and_then(|x| x.as_num()).unwrap_or(240.0) as u32,
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
    fn gradient_effects_and_prototype_roundtrip() {
        use crate::model::svelte_ast::*;

        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.style.gradient = Some(GradientDef {
                kind: GradientKind::Linear,
                angle: 135.0,
                stops: vec![
                    GradientStopDef::new(Color::hex(0x0D99FF), "#0D99FF", 0.0),
                    GradientStopDef::new(Color::hexa(0x6E56CF80), "#6E56CF80", 1.0),
                ],
            });
            e.style.effects = vec![ShadowEffect::new()];
            e.style.blur = 3.0;
            e.style.grid_cols = Some(3);
            e.style.grid_rows = Some(4);
            e.style.align_self = Some(Align::End);
            e.prototype = Some(PrototypeLink {
                trigger: ProtoTrigger::Hover,
                action: ProtoAction::OpenOverlay,
                target: Some(2),
                transition: ProtoTransition::SlideUp,
                duration_ms: 180,
            });
        }
        let root = Some(root);
        let json = to_json(&root);
        assert!(json.contains("\"gradient\""), "gradient written");
        assert!(json.contains("\"effects\""), "effects written");
        assert!(json.contains("\"prototype\""), "prototype written");
        let back = from_json(&json).expect("parse");
        assert_eq!(root, back, "fills, effects and links survive JSON");
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
    fn interface_and_overrides_roundtrip() {
        use crate::model::svelte_ast::{Override as Ov, PropDef, PropKind};

        let mut master = new_frame(1, 0.0, 0.0, 100.0, 40.0);
        if let SvelteNode::Element(e) = &mut master {
            e.component = true;
            e.name = "Card".into();
            e.interface = Some(ComponentInterface {
                props: vec![
                    PropDef {
                        name: "title".into(),
                        default: "Dialog".into(),
                        target: 2,
                        kind: PropKind::Text,
                    },
                    PropDef {
                        name: "accent".into(),
                        default: "#FA4D56".into(),
                        target: 1,
                        kind: PropKind::Fill,
                    },
                ],
                events: vec!["confirm".into(), "cancel".into()],
                slots: vec![SlotDef {
                    name: "default".into(),
                    container: 1,
                }],
            });
        }
        let mut instance = new_frame(2, 10.0, 10.0, 100.0, 40.0);
        if let SvelteNode::Element(e) = &mut instance {
            e.instance = true;
            e.name = "Card".into();
            e.overrides = vec![Ov {
                prop: "title".into(),
                value: "Delete file?".into(),
            }];
        }
        // master + instance as siblings under a plain root
        let mut root = new_frame(3, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.children = vec![master, instance];
        }
        let root = Some(root);

        let json = to_json(&root);
        let back = from_json(&json).expect("parse");
        assert_eq!(
            root, back,
            "interface + overrides must survive a round trip"
        );
    }

    #[test]
    fn slot_content_roundtrip() {
        // Phase 4: per-instance slot content must survive the JSON round trip.
        let mut instance = new_frame(2, 10.0, 10.0, 100.0, 40.0);
        if let SvelteNode::Element(e) = &mut instance {
            e.instance = true;
            e.name = "Card".into();
            e.slot_content = vec![SlotContent {
                slot: "default".into(),
                nodes: vec![new_rect(9, 0.0, 0.0, 60.0, 20.0, 0x0D99FF)],
            }];
        }
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.children = vec![instance];
        }
        let root = Some(root);

        let json = to_json(&root);
        assert!(json.contains("slot_content"), "written when present");
        let back = from_json(&json).expect("parse");
        assert_eq!(root, back, "slot content must survive a round trip");

        // A plain document must not gain the key.
        let plain = to_json(&Some(new_rect(1, 0.0, 0.0, 10.0, 10.0, 0x0D99FF)));
        assert!(!plain.contains("slot_content"));
    }

    #[test]
    fn documents_without_components_keep_the_old_shape() {
        // A plain document must not contain the new keys at all.
        let root = Some(new_rect(1, 0.0, 0.0, 10.0, 10.0, 0x0D99FF));
        let json = to_json(&root);
        assert!(!json.contains("interface"));
        assert!(!json.contains("overrides"));
        assert_eq!(from_json(&json).unwrap(), root);
    }

    #[test]
    fn escapes_special_chars() {
        assert_eq!(json_str("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    }

    // ── Phase 2: behavior round trips ──

    fn demo_behavior() -> Behavior {
        Behavior {
            vars: vec![
                Var {
                    name: "open".into(),
                    ty: VarType::Bool,
                    initial: Value::Bool(false),
                },
                Var {
                    name: "count".into(),
                    ty: VarType::Number,
                    initial: Value::Number(3.0),
                },
                Var {
                    name: "label".into(),
                    ty: VarType::String,
                    initial: Value::Str("Dialog".into()),
                },
            ],
            rules: vec![
                Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![
                        Action::ToggleVar("open".into()),
                        Action::SetVar("count".into(), ValueExpr::Value(Value::Number(7.0))),
                        Action::SetVar("label".into(), ValueExpr::Var("title".into())),
                    ],
                },
                Rule {
                    trigger: Trigger::OnKey("Enter".into()),
                    actions: vec![Action::Show {
                        target: "Panel".into(),
                        transition: Some(Transition::Slide { ms: 200 }),
                    }],
                },
                Rule {
                    trigger: Trigger::OnPropChanged("title".into()),
                    actions: vec![Action::Emit("confirm".into())],
                },
                Rule {
                    trigger: Trigger::OnEvent("selected".into()),
                    actions: vec![Action::Delay(
                        300,
                        vec![Action::Hide {
                            target: "Panel".into(),
                            transition: Some(Transition::Fade { ms: 150 }),
                        }],
                    )],
                },
            ],
        }
    }

    #[test]
    fn behavior_roundtrips_through_document_json() {
        let mut master = new_frame(1, 0.0, 0.0, 100.0, 80.0);
        if let SvelteNode::Element(e) = &mut master {
            e.component = true;
            e.name = "Overlay".into();
            e.behavior = Some(demo_behavior());
            e.binding = Some(Binding {
                var: "open".into(),
                transition: Some(Transition::Scale { ms: 120 }),
            });
            e.hover = Some(HoverStyle {
                fill: Some("#3A3A3E".into()),
                color: None,
                opacity: Some(0.8),
            });
        }
        let root = Some(master);
        let json = to_json(&root);
        let back = from_json(&json).expect("parse");
        assert_eq!(root, back, "behavior + binding + hover must survive");
    }

    #[test]
    fn compact_vd_payload_roundtrips() {
        let mut el = match new_frame(1, 0.0, 0.0, 10.0, 10.0) {
            SvelteNode::Element(e) => e,
            _ => unreachable!(),
        };
        el.behavior = Some(demo_behavior());
        el.binding = Some(Binding {
            var: "open".into(),
            transition: Some(Transition::Fade { ms: 150 }),
        });
        el.hover = Some(HoverStyle {
            fill: Some("#111111".into()),
            color: Some("#FFFFFF".into()),
            opacity: None,
        });
        let payload = vd_payload(&el).expect("payload present");
        // Single line, no raw newlines — safe inside an HTML comment.
        assert!(!payload.contains('\n'));
        let parsed = parse_json(&payload).expect("payload is valid JSON");
        assert_eq!(
            parse_behavior(parsed.get("behavior").unwrap()).as_ref(),
            el.behavior.as_ref()
        );
        assert_eq!(
            parse_binding(parsed.get("binding").unwrap()).as_ref(),
            el.binding.as_ref()
        );
        assert_eq!(
            parse_hover(parsed.get("hover").unwrap()).as_ref(),
            el.hover.as_ref()
        );
    }

    #[test]
    fn empty_behavior_produces_no_payload() {
        let mut el = match new_frame(1, 0.0, 0.0, 10.0, 10.0) {
            SvelteNode::Element(e) => e,
            _ => unreachable!(),
        };
        el.behavior = Some(Behavior::default());
        assert!(vd_payload(&el).is_none());
        // Hover without values is also skipped.
        el.hover = Some(HoverStyle::default());
        assert!(vd_payload(&el).is_none());
    }

    #[test]
    fn plain_documents_stay_byte_identical() {
        let root = Some(new_rect(1, 0.0, 0.0, 10.0, 10.0, 0x0D99FF));
        let json = to_json(&root);
        assert!(!json.contains("behavior"));
        assert!(!json.contains("binding"));
        assert!(!json.contains("hover"));
        assert_eq!(from_json(&json).unwrap(), root);
    }
}
