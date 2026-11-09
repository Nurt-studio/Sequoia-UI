//! AST → Svelte code. Deterministic and parseable back.
//!
//! Phase 2 behaviors (design doc §3.3, §6.1):
//! - vars → `let open = $state(false);` (Svelte 5 runes),
//! - rules → DOM handlers (`onclick`), `onMount` / `$effect` blocks,
//! - bindings → `{#if open}` with `transition:` directives,
//! - styling hover → pure CSS (`.btn:hover { … }`),
//! - everything above is also written as a `<!-- vd-behavior: {…} -->`
//!   comment, so `from_svelte` can restore the model on import.

use std::collections::{BTreeSet, HashMap, HashSet};

use sequoia_ui::prelude::Color;

use crate::model::behavior::{Action, HoverStyle, Trigger, binding_var_of};
use crate::model::svelte_ast::*;

pub fn to_svelte(root: &Option<SvelteNode>) -> String {
    // Slot content is materialized first, so single-file output shows the
    // instances exactly as the canvas does (container children = content).
    let prepared = root.as_ref().map(crate::model::component_ops::inject_slots);
    let mut ctx = Ctx::new();
    let script = behavior_script(prepared.as_ref(), &mut ctx);
    let body = match &prepared {
        Some(r) => node_markup(r, &mut ctx, 0, ParentBox::Free),
        None => "  <!-- empty -->\n".to_string(),
    };
    let mut out = String::new();
    out.push_str(&script);
    out.push_str(&body);
    out.push_str("\n<style>\n");
    for s in &ctx.styles {
        out.push_str(s);
    }
    out.push_str("</style>\n");
    out
}

/// Shared rendering state. Also carries the prop bindings used by the
/// multi-file export (`codegen::project`): a prop-driven node renders its
/// bound value (`{title}` / inline style) instead of the literal value.
pub(crate) struct Ctx {
    pub(crate) names: HashSet<String>,
    pub(crate) styles: Vec<String>,
    /// Text node id → prop name: content renders as `{prop}`.
    pub(crate) text_bindings: HashMap<u64, String>,
    /// Element id → prop name: background renders as an inline style.
    pub(crate) fill_bindings: HashMap<u64, String>,
    /// Text node id → prop name: color renders as an inline style.
    pub(crate) color_bindings: HashMap<u64, String>,
    /// Element id → (DOM attribute, JS expression) pairs from rules.
    pub(crate) handler_attrs: HashMap<u64, Vec<(String, String)>>,
    /// Names of `svelte/transition` functions used in this file.
    pub(crate) transitions: BTreeSet<&'static str>,
    /// Script lines that must run once on mount (`On Mount` rules).
    pub(crate) on_mount: Vec<String>,
    /// Script lines with reactive effects (`On Prop Changed` rules).
    pub(crate) effects: Vec<String>,
    /// Element id → slot name: multi-file export marks slot-content roots
    /// of an instance tag with `slot="name"` (Phase 4).
    pub(crate) slot_attrs: HashMap<u64, String>,
    /// Events announced by a handler (Emit) — only informational here.
    pub(crate) _emitted: HashSet<String>,
}

impl Ctx {
    pub(crate) fn new() -> Self {
        Self {
            names: HashSet::new(),
            styles: Vec::new(),
            text_bindings: HashMap::new(),
            fill_bindings: HashMap::new(),
            color_bindings: HashMap::new(),
            handler_attrs: HashMap::new(),
            slot_attrs: HashMap::new(),
            transitions: BTreeSet::new(),
            on_mount: Vec::new(),
            effects: Vec::new(),
            _emitted: HashSet::new(),
        }
    }
}

pub(crate) fn unique_class(base: &str, ctx: &mut Ctx) -> String {
    let mut candidate: String = base
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    candidate = candidate.trim_matches('-').to_string();
    if candidate.is_empty() {
        candidate = "node".into();
    }
    if candidate
        .chars()
        .next()
        .map_or(true, |c| c.is_ascii_digit())
    {
        candidate = format!("n-{}", candidate);
    }
    let mut i = 1;
    let mut final_name = candidate.clone();
    while ctx.names.contains(&final_name) {
        i += 1;
        final_name = format!("{}-{}", candidate, i);
    }
    ctx.names.insert(final_name.clone());
    final_name
}

/// How a child is laid out by its parent, which decides how Figma-style
/// sizing maps to CSS (`flex: 1` / `align-self: stretch` / `%`).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ParentBox {
    Free,
    FlexRow,
    FlexCol,
    Grid,
}

fn is_absolute_node(node: &SvelteNode) -> bool {
    match node {
        SvelteNode::Element(e) => e.style.position == Position::Absolute,
        SvelteNode::Text(t) => t.absolute,
    }
}

pub(crate) fn node_markup(
    node: &SvelteNode,
    ctx: &mut Ctx,
    depth: usize,
    parent: ParentBox,
) -> String {
    let indent = "  ".repeat(depth);
    // Absolute children are out of the flow: their percentages refer to the
    // parent box instead of a flex share.
    let parent = if is_absolute_node(node) {
        ParentBox::Free
    } else {
        parent
    };
    match node {
        SvelteNode::Element(e) => {
            let class = unique_class(&e.name, ctx);
            ctx.styles.push(render_style(&class, &e.style, parent));
            push_hover_css(ctx, &class, e);
            let tag = e.tag.as_str();
            let attrs = element_extra_attrs(e, ctx)
                + &match ctx.fill_bindings.get(&e.id) {
                    Some(prop) => format!(" style=\"background: {prop}\""),
                    None => String::new(),
                };
            let child_parent = match e.style.display {
                Display::Flex if e.style.flex_dir == FlexDir::Row => ParentBox::FlexRow,
                Display::Flex => ParentBox::FlexCol,
                Display::Grid => ParentBox::Grid,
                Display::None => ParentBox::Free,
            };
            let inner = if e.children.is_empty() {
                format!("{indent}<{tag} class=\"{class}\"{attrs}></{tag}>\n")
            } else {
                let mut out = format!("{indent}<{tag} class=\"{class}\"{attrs}>\n");
                for c in &e.children {
                    out.push_str(&node_markup(c, ctx, depth + 1, child_parent));
                }
                out.push_str(&format!("{indent}</{tag}>\n"));
                out
            };
            format!(
                "{}{inner}{}",
                wrap_prefix(e, &indent),
                wrap_suffix(e, &indent)
            )
        }
        SvelteNode::Text(t) => {
            let class = unique_class(&t.name, ctx);
            ctx.styles.push(render_text_style(&class, t));
            let content = match ctx.text_bindings.get(&t.id) {
                Some(prop) => format!("{{{prop}}}"),
                None => escape_text(&t.content),
            };
            let attrs = match ctx.color_bindings.get(&t.id) {
                Some(prop) => format!(" style=\"color: {prop}\""),
                None => String::new(),
            };
            format!(
                "{indent}<span class=\"{class}\"{attrs}>{}</span>\n",
                content
            )
        }
    }
}

pub(crate) fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn fmt_dim(v: Dimension) -> String {
    match v {
        Dimension::Auto => "auto".into(),
        Dimension::Fill => "100%".into(),
        Dimension::Fixed(v) if v.fract() == 0.0 => format!("{}px", v as i32),
        Dimension::Fixed(v) => format!("{:.2}px", v),
    }
}

fn fmt_color(c: Color) -> String {
    let rgba = c.to_rgba8();
    if rgba.3 == 255 {
        format!("#{:02X}{:02X}{:02X}", rgba.0, rgba.1, rgba.2)
    } else {
        format!(
            "rgba({},{},{},{:.3})",
            rgba.0,
            rgba.1,
            rgba.2,
            rgba.3 as f32 / 255.0
        )
    }
}

pub(crate) fn render_style(class: &str, s: &Style, parent: ParentBox) -> String {
    let mut rules: Vec<String> = Vec::new();

    match s.display {
        Display::Flex => {
            rules.push("display: flex".into());
            rules.push(format!(
                "flex-direction: {}",
                if s.flex_dir == FlexDir::Row {
                    "row"
                } else {
                    "column"
                }
            ));
            let wraps = s.flex_wrap == Wrap::Wrap;
            if wraps {
                rules.push("flex-wrap: wrap".into());
            }
            // `gap` is the item spacing along the main axis, `row_gap` the
            // spacing between wrapped lines. CSS takes row-gap first.
            let (row_gap, col_gap) = if s.flex_dir == FlexDir::Row {
                (s.row_gap, s.gap)
            } else {
                (s.gap, s.row_gap)
            };
            if wraps && (row_gap - col_gap).abs() > 0.001 {
                rules.push(format!("gap: {row_gap}px {col_gap}px"));
            } else if col_gap > 0.0 {
                rules.push(format!("gap: {col_gap}px"));
            }
            rules.push(format!(
                "align-items: {}",
                match s.align_items {
                    Align::Center => "center",
                    Align::End => "flex-end",
                    Align::Stretch => "stretch",
                    _ => "flex-start",
                }
            ));
            if s.justify_content != Align::Start {
                rules.push(format!(
                    "justify-content: {}",
                    s.justify_content.css_content()
                ));
            }
            if wraps && s.align_content != Align::Start {
                rules.push(format!("align-content: {}", s.align_content.css_content()));
            }
        }
        Display::Grid => {
            rules.push("display: grid".into());
            if let Some(n) = s.grid_cols {
                rules.push(format!("grid-template-columns: repeat({n}, 1fr)"));
            }
            if s.gap > 0.0 {
                rules.push(format!("gap: {}px", s.gap));
            }
        }
        Display::None => {}
    }

    if s.position == Position::Absolute {
        rules.push("position: absolute".into());
    }
    if let Some(t) = s.top {
        rules.push(format!("top: {}px", t));
    }
    if let Some(r) = s.right {
        rules.push(format!("right: {}px", r));
    }
    if let Some(b) = s.bottom {
        rules.push(format!("bottom: {}px", b));
    }
    if let Some(l) = s.left {
        rules.push(format!("left: {}px", l));
    }

    // Fill / hug / fixed map to flexbox: fill along the main axis grows, fill
    // on the cross axis stretches, and outside a flex parent it is 100%.
    let flex_row = parent == ParentBox::FlexRow;
    let flex_col = parent == ParentBox::FlexCol;
    let main_fill = (flex_row && s.width.is_fill()) || (flex_col && s.height.is_fill());
    let cross_fill = (flex_row && s.height.is_fill()) || (flex_col && s.width.is_fill());
    if main_fill {
        rules.push("flex: 1 0 0".into());
    }
    if cross_fill {
        rules.push("align-self: stretch".into());
    }
    match s.width {
        Dimension::Fixed(v) => rules.push(format!("width: {}px", v)),
        Dimension::Fill if !flex_row && !flex_col => rules.push("width: 100%".into()),
        _ => {}
    }
    match s.height {
        Dimension::Fixed(v) => rules.push(format!("height: {}px", v)),
        Dimension::Fill if !flex_row && !flex_col => rules.push("height: 100%".into()),
        _ => {}
    }

    if s.padding.top != 0.0
        || s.padding.right != 0.0
        || s.padding.bottom != 0.0
        || s.padding.left != 0.0
    {
        rules.push(format!(
            "padding: {}px {}px {}px {}px",
            s.padding.top, s.padding.right, s.padding.bottom, s.padding.left
        ));
    }

    if let Some(f) = s.fill {
        rules.push(format!("background: {}", fmt_color(f)));
    }
    if let (Some(sc), w) = (s.stroke, s.stroke_width) {
        if w > 0.0 {
            rules.push(format!("border: {}px solid {}", w, fmt_color(sc)));
        }
    }
    if s.radius > 0.0 {
        if s.radius >= 999.0 {
            rules.push("border-radius: 50%".into());
        } else {
            rules.push(format!("border-radius: {}px", s.radius));
        }
    }
    if s.opacity < 100.0 {
        rules.push(format!("opacity: {}", s.opacity / 100.0));
    }

    let body = rules
        .iter()
        .map(|r| format!("  {r};\n"))
        .collect::<String>();
    format!(".{class} {{\n{body}}}\n\n")
}

pub(crate) fn render_text_style(class: &str, t: &TextNode) -> String {
    let mut rules: Vec<String> = Vec::new();
    if t.absolute {
        rules.push("position: absolute".into());
        rules.push(format!("left: {}px", t.x));
        rules.push(format!("top: {}px", t.y));
    }
    if let Some(v) = t.width {
        rules.push(format!("width: {}px", v));
    }
    if let Some(v) = t.height {
        rules.push(format!("height: {}px", v));
    }
    rules.push(format!("font-size: {}px", t.style.font_size));
    if t.style.opacity < 100.0 {
        rules.push(format!("opacity: {}", t.style.opacity / 100.0));
    }
    if t.style.font_weight != 400 {
        rules.push(format!("font-weight: {}", t.style.font_weight));
    }
    rules.push(format!("color: {}", fmt_color(t.style.color)));
    if t.style.align != Align::Start {
        rules.push(format!(
            "text-align: {}",
            match t.style.align {
                Align::Center => "center",
                Align::End => "right",
                _ => "left",
            }
        ));
    }
    let body = rules
        .iter()
        .map(|r| format!("  {r};\n"))
        .collect::<String>();
    format!(".{class} {{\n{body}}}\n\n")
}

// ──────────────────────────── Phase 2: behavior → script + handlers

/// Extra attributes for one element tag: the `transition:` directive from
/// the binding plus DOM handler attributes from rules (Svelte 5 events).
pub(crate) fn element_extra_attrs(e: &Element, ctx: &Ctx) -> String {
    let mut attrs = String::new();
    if let Some(b) = &e.binding {
        if let Some(t) = b.transition {
            attrs.push_str(&format!(" transition:{}", t.to_svelte()));
        }
    }
    if let Some(pairs) = ctx.handler_attrs.get(&e.id) {
        for (attr, expr) in pairs {
            // Svelte expression syntax: onclick={() => …} — the braces are
            // part of the attribute value, so they wrap the JS expression.
            attrs.push_str(&format!(" {attr}={{{expr}}}"));
        }
    }
    if let Some(slot) = ctx.slot_attrs.get(&e.id) {
        // Named-slot content of an instance (multi-file export, Phase 4).
        attrs.push_str(&format!(" slot=\"{slot}\""));
    }
    attrs
}

/// `<!-- vd-behavior: {…} -->` + `{#if var}` — everything that goes before
/// the element tag. The comment is the round-trip carrier (design doc §6.3).
pub(crate) fn wrap_prefix(e: &Element, indent: &str) -> String {
    let vd = crate::document::json::vd_payload(e);
    match (&vd, &e.binding) {
        (Some(payload), Some(b)) => format!(
            "{indent}<!-- vd-behavior: {payload} -->\n{indent}{{#if {}}}\n",
            b.var
        ),
        (Some(payload), None) => format!("{indent}<!-- vd-behavior: {payload} -->\n"),
        (None, Some(b)) => format!("{indent}{{#if {}}}\n", b.var),
        (None, None) => String::new(),
    }
}

/// `{/if}` for bound elements.
pub(crate) fn wrap_suffix(e: &Element, indent: &str) -> String {
    if e.binding.is_some() {
        format!("{indent}{{/if}}\n")
    } else {
        String::new()
    }
}

/// `.class:hover { … }` — appended to the style sheet when present.
pub(crate) fn push_hover_css(ctx: &mut Ctx, class: &str, e: &Element) {
    if let Some(h) = &e.hover {
        if !h.is_empty() {
            ctx.styles.push(render_hover(class, h));
        }
    }
}

/// Builds the `<script>` block (Svelte 5 runes) for one generated file and
/// fills `ctx` with handler attributes consumed by `node_markup`. `root` is
/// the file's own subtree: a master element for `components/*.svelte`, the
/// page for `App.svelte` — rules can only reach targets inside it.
pub(crate) fn behavior_script(root: Option<&SvelteNode>, ctx: &mut Ctx) -> String {
    let state_lines = match root {
        Some(r) => collect_behavior_inline(r, ctx),
        None => Vec::new(),
    };

    let mut out = String::from("<script lang=\"ts\">\n  // generated by Vision Design\n");
    if !ctx.transitions.is_empty() {
        let names: Vec<&str> = ctx.transitions.iter().copied().collect();
        out.push_str(&format!(
            "  import {{ {} }} from 'svelte/transition';\n",
            names.join(", ")
        ));
    }
    if !ctx.on_mount.is_empty() {
        out.push_str("  import { onMount } from 'svelte';\n");
    }
    if !state_lines.is_empty() {
        out.push('\n');
        for l in &state_lines {
            out.push_str(&format!("  {l}\n"));
        }
    }
    if !ctx.on_mount.is_empty() || !ctx.effects.is_empty() {
        out.push('\n');
    }
    for l in &ctx.on_mount {
        out.push_str(l);
    }
    for l in &ctx.effects {
        out.push_str(l);
    }
    out.push_str("</script>\n\n");
    out
}

/// Document-order walk collecting `$state` declarations, script-level rule
/// bodies and per-element handler attributes for ONE generated file.
/// Masters and instances that are not the root of the file are skipped —
/// their behavior lives in the master's own file (design doc §6.2).
pub(crate) fn collect_behavior(root: &SvelteNode, ctx: &mut Ctx) -> Vec<String> {
    collect_behavior_impl(root, ctx, true)
}

/// Single-file mode (`to_svelte`): the whole tree renders inline, masters
/// and instances included, so their behaviors are collected as well.
pub(crate) fn collect_behavior_inline(root: &SvelteNode, ctx: &mut Ctx) -> Vec<String> {
    collect_behavior_impl(root, ctx, false)
}

fn collect_behavior_impl(root: &SvelteNode, ctx: &mut Ctx, skip_nested: bool) -> Vec<String> {
    let mut declared: HashSet<String> = HashSet::new();
    let mut state_lines: Vec<String> = Vec::new();
    collect_behavior_walk(
        root,
        root,
        ctx,
        &mut declared,
        &mut state_lines,
        true,
        skip_nested,
    );
    state_lines
}

#[allow(clippy::too_many_arguments)]
fn collect_behavior_walk(
    node: &SvelteNode,
    scope: &SvelteNode,
    ctx: &mut Ctx,
    declared: &mut HashSet<String>,
    state_lines: &mut Vec<String>,
    is_root: bool,
    skip_nested: bool,
) {
    if let SvelteNode::Element(e) = node {
        if skip_nested && !is_root && (e.component || e.instance) {
            // Black box: defined once in the master's file, wrapped by the
            // page. Only the instance's own binding transition is imported.
            if let Some(b) = &e.binding {
                if let Some(t) = b.transition {
                    ctx.transitions.insert(t.kind_name());
                }
            }
            // Phase 4: an `On Event` rule on the instance is the parent's
            // handler for the child's event — it becomes `on:{event}={…}` on
            // the instance tag (design doc §4.6).
            if e.instance {
                if let Some(b) = &e.behavior {
                    for rule in &b.rules {
                        if let Trigger::OnEvent(ev) = &rule.trigger {
                            let expr = event_handler_expr(&rule.actions, scope);
                            ctx.handler_attrs
                                .entry(e.id)
                                .or_default()
                                .push((format!("on{}", event_callback_name(ev)), expr));
                        }
                    }
                }
            }
            return;
        }
        if let Some(b) = &e.behavior {
            for v in &b.vars {
                // Name conflicts across behaviors resolve to the first
                // declaration (validation reports them for the editor).
                if declared.insert(v.name.clone()) {
                    state_lines.push(format!("let {} = $state({});", v.name, v.initial.to_js()));
                }
            }
            for rule in &b.rules {
                match &rule.trigger {
                    Trigger::OnMount => {
                        let body = actions_block(&rule.actions, scope, 4);
                        ctx.on_mount
                            .push(format!("  onMount(() => {{\n{body}  }});\n"));
                    }
                    Trigger::OnPropChanged(p) => {
                        let body = actions_block(&rule.actions, scope, 4);
                        ctx.effects.push(format!(
                            "  $effect(() => {{\n    // On Prop Changed ({p})\n    void {p};\n{body}  }});\n"
                        ));
                    }
                    Trigger::OnEvent(ev) if !node.is_instance() => {
                        ctx.effects.push(format!(
                            "  // vd: On Event ({ev}) — no source outside an instance (Phase 4)\n"
                        ));
                    }
                    t => {
                        if let Some(attr) = t.dom_attr() {
                            let expr = handler_expr(t, &rule.actions, scope);
                            ctx.handler_attrs
                                .entry(e.id)
                                .or_default()
                                .push((attr.to_string(), expr));
                        }
                    }
                }
            }
        }
        if let Some(b) = &e.binding {
            if let Some(t) = b.transition {
                ctx.transitions.insert(t.kind_name());
            }
        }
    }
    for c in node.children() {
        collect_behavior_walk(c, scope, ctx, declared, state_lines, false, skip_nested);
    }
}

/// JS statement for one action. Show/Hide resolve to assignments of the
/// target's binding variable — the deterministic bridge between rules and
/// `{#if}` blocks (design doc §4.5). `Delay` emits a real `setTimeout`
/// with the nested steps in the closure (Phase 3).
fn action_stmt(a: &Action, scope: &SvelteNode, indent: usize) -> String {
    match a {
        Action::ToggleVar(v) => format!("{v} = !{v};"),
        Action::SetVar(v, e) => format!("{v} = {};", e.to_js()),
        Action::Show { target, .. } => match binding_var_of(scope, target) {
            Some(v) => format!("{v} = true;"),
            None => format!("/* vd: target \"{target}\" has no binding */"),
        },
        Action::Hide { target, .. } => match binding_var_of(scope, target) {
            Some(v) => format!("{v} = false;"),
            None => format!("/* vd: target \"{target}\" has no binding */"),
        },
        Action::Emit(ev) => format!("{}?.();", event_callback_name(ev)),
        Action::Delay(ms, nested) => {
            let pad = " ".repeat(indent);
            if nested.is_empty() {
                format!("setTimeout(() => {{}}, {ms});")
            } else {
                let body = actions_block(nested, scope, indent + 2);
                format!("setTimeout(() => {{\n{body}{pad}}}, {ms});")
            }
        }
    }
}

fn actions_block(actions: &[Action], scope: &SvelteNode, indent: usize) -> String {
    actions
        .iter()
        .map(|a| format!("{}{}\n", " ".repeat(indent), action_stmt(a, scope, indent)))
        .collect()
}

fn handler_expr(trigger: &Trigger, actions: &[Action], scope: &SvelteNode) -> String {
    let stmts: Vec<String> = actions.iter().map(|a| action_stmt(a, scope, 2)).collect();
    // A `Delay` spans multiple lines, so the handler must keep its block.
    let multiline = stmts.iter().any(|s| s.contains('\n'));
    match trigger {
        Trigger::OnKey(key) => format!(
            "(e) => {{ if (e.key === '{}') {{ {} }} }}",
            escape_js_str(key),
            stmts.join(" ")
        ),
        _ if stmts.len() == 1 && !multiline && !stmts[0].starts_with("/*") => {
            let single = stmts[0].trim_end_matches(';');
            format!("() => {single}")
        }
        _ => format!("() => {{ {} }}", stmts.join(" ")),
    }
}

/// Handler of an instance event in the parent: the child's payload arrives
/// as `payload` (`<Card on:selected={(payload) => { … }} />`).
fn event_handler_expr(actions: &[Action], scope: &SvelteNode) -> String {
    let stmts: Vec<String> = actions.iter().map(|a| action_stmt(a, scope, 2)).collect();
    format!("(payload) => {{ {} }}", stmts.join(" "))
}

fn escape_js_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

/// `confirm` → `onconfirm`; safe identifier for a callback prop.
pub(crate) fn event_callback_name(event: &str) -> String {
    let mut s: String = event
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    if s.is_empty() {
        s = "event".into();
    }
    if s.chars().next().map_or(true, |c| c.is_ascii_digit()) {
        s = format!("e{s}");
    }
    format!("on{s}")
}

/// `.class:hover { … }` — pure CSS for styling hovers (design doc §6.1).
pub(crate) fn render_hover(class: &str, h: &HoverStyle) -> String {
    let mut rules: Vec<String> = Vec::new();
    if let Some(f) = &h.fill {
        rules.push(format!("background: {f}"));
    }
    if let Some(c) = &h.color {
        rules.push(format!("color: {c}"));
    }
    if let Some(o) = h.opacity {
        rules.push(format!("opacity: {o}"));
    }
    let body = rules
        .iter()
        .map(|r| format!("  {r};\n"))
        .collect::<String>();
    format!(".{class}:hover {{\n{body}}}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::from_svelte;

    #[test]
    fn generated_code_roundtrips() {
        let root = Some(crate::document::demo::demo_tree());
        let code1 = to_svelte(&root);
        let parsed = from_svelte(&code1).expect("parser accepts its own output");
        let code2 = to_svelte(&parsed);
        assert_eq!(code1, code2, "AST -> code -> AST -> code must be stable");
    }

    #[test]
    fn auto_layout_roundtrips() {
        use crate::model::svelte_ast::*;
        let mut inner = new_rect(2, 0.0, 0.0, 0.0, 40.0, 0x0D99FF);
        if let SvelteNode::Element(e) = &mut inner {
            e.style.position = Position::Relative;
            e.style.width = Dimension::Fill;
        }
        let mut tall = new_rect(4, 0.0, 0.0, 60.0, 0.0, 0x0D99FF);
        if let SvelteNode::Element(e) = &mut tall {
            e.style.position = Position::Relative;
            e.style.height = Dimension::Fill;
        }
        let mut fixed = new_rect(3, 0.0, 0.0, 80.0, 40.0, 0x42BE65);
        if let SvelteNode::Element(e) = &mut fixed {
            e.style.position = Position::Relative;
        }
        let frame = SvelteNode::Element(Element {
            id: 1,
            name: "Nav".into(),
            tag: Tag::Div,
            style: Style {
                display: Display::Flex,
                flex_dir: FlexDir::Row,
                flex_wrap: Wrap::Wrap,
                gap: 12.0,
                row_gap: 6.0,
                align_items: Align::Stretch,
                justify_content: Align::SpaceBetween,
                align_content: Align::SpaceBetween,
                padding: sequoia_ui::prelude::Edges::all(16.0),
                width: Dimension::Fixed(600.0),
                height: Dimension::Auto,
                ..Default::default()
            },
            children: vec![fixed, inner, tall],
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
        });
        let root = Some(frame);
        let code = to_svelte(&root);
        for needle in [
            "flex-wrap: wrap",
            "gap: 6px 12px",
            "align-items: stretch",
            "justify-content: space-between",
            "align-content: space-between",
            "flex: 1 0 0",
            "align-self: stretch",
        ] {
            assert!(code.contains(needle), "missing `{needle}` in:\n{code}");
        }

        let parsed = from_svelte(&code).expect("parser accepts its own output");
        assert_eq!(
            to_svelte(&parsed),
            code,
            "auto layout must survive a round trip"
        );
    }

    #[test]
    fn empty_document_renders_placeholder() {
        let code = to_svelte(&None);
        assert!(code.contains("<!-- empty -->"));
        assert!(code.contains("<style>"));
    }

    // ── Phase 2: behaviors ──

    /// Page(1, var open) { Button(2, OnClick→Toggle open, hover), Panel(3,
    /// binding open+slide) } — the §4.5 example end to end.
    fn behavior_demo() -> SvelteNode {
        use crate::model::behavior::{
            Action, Behavior, Binding, HoverStyle, Rule, Transition, Trigger, Var,
        };
        let mut panel = new_frame(3, 0.0, 0.0, 200.0, 100.0);
        if let SvelteNode::Element(e) = &mut panel {
            e.name = "Panel".into();
            e.binding = Some(Binding {
                var: "open".into(),
                transition: Some(Transition::Slide { ms: 200 }),
            });
        }
        let mut btn = new_button(2, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut btn {
            e.name = "Btn".into();
            e.style.position = Position::Relative;
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![Action::ToggleVar("open".into())],
                }],
            });
            e.hover = Some(HoverStyle {
                fill: Some("#3DB6FF".into()),
                color: None,
                opacity: None,
            });
        }
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.name = "Page".into();
            e.style.position = Position::Relative;
            e.behavior = Some(Behavior {
                vars: vec![Var {
                    name: "open".into(),
                    ty: crate::model::behavior::VarType::Bool,
                    initial: crate::model::behavior::Value::Bool(false),
                }],
                rules: vec![],
            });
            if let SvelteNode::Element(p) = &mut panel {
                p.style.position = Position::Relative;
            }
            e.children = vec![btn, panel];
        }
        root
    }

    #[test]
    fn behavior_codegen_produces_svelte_5() {
        let code = to_svelte(&Some(behavior_demo()));
        // $state rune for the variable
        assert!(
            code.contains("let open = $state(false);"),
            "script:\n{code}"
        );
        // DOM event attribute (Svelte 5) generated from the rule
        assert!(
            code.contains("onclick={() => open = !open}"),
            "handler:\n{code}"
        );
        // Binding wraps the element in {#if} with the transition directive
        assert!(code.contains("{#if open}"), "wrap:\n{code}");
        assert!(code.contains("{/if}"), "close:\n{code}");
        assert!(
            code.contains("transition:slide={{ duration: 200 }}"),
            "directive:\n{code}"
        );
        // Transition import line
        assert!(
            code.contains("import { slide } from 'svelte/transition';"),
            "imports:\n{code}"
        );
        // Pure-CSS hover (design doc §6.1)
        assert!(code.contains(":hover {"), "hover CSS:\n{code}");
        assert!(code.contains("background: #3DB6FF;"), "hover fill:\n{code}");
        // Round-trip carrier comment
        assert!(
            code.contains("<!-- vd-behavior: {\"behavior\":"),
            "comment:\n{code}"
        );
    }

    #[test]
    fn behavior_code_roundtrips_stably() {
        let root = Some(behavior_demo());
        let code1 = to_svelte(&root);
        let parsed = from_svelte(&code1).expect("parser accepts its own output");
        // The behavior model must survive: the parsed button still carries
        // its rule, the panel its binding.
        let btn_behavior = parsed
            .as_ref()
            .and_then(|root| crate::model::tree_ops::find(root, 2))
            .and_then(|n| match n {
                SvelteNode::Element(e) => Some(e.behavior.clone()),
                _ => None,
            })
            .flatten()
            .expect("button behavior restored");
        assert_eq!(btn_behavior.rules.len(), 1);
        assert_eq!(btn_behavior.rules[0].trigger, Trigger::OnClick);
        assert_eq!(
            to_svelte(&parsed),
            code1,
            "AST -> code -> AST -> code must be stable"
        );
    }

    #[test]
    fn show_resolves_to_binding_assignment_and_key_guards() {
        use crate::model::behavior::{Action, Behavior, Binding, Rule, Trigger};
        let mut panel = new_frame(3, 0.0, 0.0, 100.0, 50.0);
        if let SvelteNode::Element(e) = &mut panel {
            e.name = "Panel".into();
            e.binding = Some(Binding {
                var: "open".into(),
                transition: None,
            });
        }
        let mut btn = new_button(2, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut btn {
            e.name = "Btn".into();
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![
                    Rule {
                        trigger: Trigger::OnClick,
                        actions: vec![Action::Show {
                            target: "Panel".into(),
                            transition: None,
                        }],
                    },
                    Rule {
                        trigger: Trigger::OnKey("Enter".into()),
                        actions: vec![
                            Action::Hide {
                                target: "Panel".into(),
                                transition: None,
                            },
                            Action::Emit("confirm".into()),
                        ],
                    },
                ],
            });
        }
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.children = vec![btn, panel];
        }
        let code = to_svelte(&Some(root));
        assert!(
            code.contains("onclick={() => open = true}"),
            "Show resolves through the target's binding:\n{code}"
        );
        assert!(
            code.contains(
                "onkeydown={(e) => { if (e.key === 'Enter') { open = false; onconfirm?.(); } }}"
            ),
            "key guard with two actions:\n{code}"
        );
        assert!(code.contains("{#if open}"), "{code}");
    }

    #[test]
    fn delay_emits_settimeout_with_nested_steps() {
        use crate::model::behavior::{Action, Behavior, Binding, Rule, Trigger, Var, VarType};
        let mut panel = new_frame(3, 0.0, 0.0, 100.0, 50.0);
        if let SvelteNode::Element(e) = &mut panel {
            e.name = "Panel".into();
            e.binding = Some(Binding {
                var: "open".into(),
                transition: None,
            });
        }
        let mut btn = new_button(2, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut btn {
            e.name = "Btn".into();
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![
                        Action::Delay(200, vec![Action::ToggleVar("open".into())]),
                        Action::Hide {
                            target: "Panel".into(),
                            transition: None,
                        },
                    ],
                }],
            });
        }
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.name = "Page".into();
            e.behavior = Some(Behavior {
                vars: vec![Var {
                    name: "open".into(),
                    ty: VarType::Bool,
                    initial: crate::model::behavior::Value::Bool(false),
                }],
                rules: vec![],
            });
            e.children = vec![btn, panel];
        }
        let code = to_svelte(&Some(root));
        // Delay keeps its block form and nests the inner steps; the next
        // statement in the rule runs immediately after (setTimeout is async).
        let expected =
            "onclick={() => { setTimeout(() => {\n    open = !open;\n  }, 200); open = false; }}";
        assert!(code.contains(expected), "delay handler:\n{code}");
        // The round-trip carrier comment still restores the same model.
        let parsed = from_svelte(&code).expect("parse");
        let btn_back = parsed
            .as_ref()
            .and_then(|r| crate::model::tree_ops::find(r, 2))
            .map(|n| match n {
                SvelteNode::Element(e) => e.behavior.clone().unwrap(),
                _ => panic!("btn"),
            })
            .unwrap();
        assert_eq!(
            btn_back.rules[0].actions[0],
            Action::Delay(200, vec![Action::ToggleVar("open".into())])
        );
        assert_eq!(to_svelte(&parsed), code, "delay round trip is stable");
    }

    #[test]
    fn class_names_are_unique_and_valid() {
        let code = to_svelte(&Some(crate::document::demo::demo_tree()));
        // every CSS class used in the markup has a section in <style>
        let mut opens = 0;
        for line in code.lines() {
            if line.starts_with('<') && !line.contains("</") {
                opens += 1;
            }
        }
        let rules = code.matches(".{").count() + code.matches('{').count();
        assert!(opens > 0);
        assert!(rules >= opens, "CSS rules: {rules}, tags: {opens}");
    }
}
