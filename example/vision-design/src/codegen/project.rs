//! Multi-file Svelte export (Phase 1).
//!
//! Rules (design doc §6.2):
//! - every master (`component: true`) → its own file `components/<Name>.svelte`,
//! - every instance → import + `<Name prop="…" />` with the overridden props,
//! - the rest of the tree → `App.svelte`.
//!
//! The editor stays the single source of truth: project files are an export,
//! not a round-trip target. Instances render as self-closing tags (slots are
//! Phase 4). A prop-bound node in a master file renders `{prop}` / an inline
//! style, so the master reads like hand-written Svelte.

use std::collections::{HashMap, HashSet};

use crate::document::json::json_str;
use crate::model::svelte_ast::*;

use super::to_svelte::{
    Ctx, ParentBox, collect_behavior, element_extra_attrs, escape_text, event_callback_name,
    node_markup, push_hover_css, render_style, unique_class, wrap_prefix, wrap_suffix,
};

/// One generated file; `path` is relative to the export root.
pub struct SvelteFile {
    pub path: String,
    pub code: String,
}

/// Master naming: id → component tag (JS identifier) and file base name.
#[derive(Default)]
struct Naming {
    by_name: HashMap<String, u64>,
    tags: HashMap<u64, String>,
    files: HashMap<u64, String>,
    /// Master id → its slot names (Phase 4): the consumer side of a slot
    /// needs them to decide between a bare and a named `slot=` attribute.
    slot_names: HashMap<u64, Vec<String>>,
}

impl Naming {
    fn tag_of(&self, master: u64) -> Option<&str> {
        self.tags.get(&master).map(|s| s.as_str())
    }
    fn file_of(&self, master: u64) -> Option<&str> {
        self.files.get(&master).map(|s| s.as_str())
    }
}

// ───────────────────────────────────────────────────── entry point

pub fn export_project(root: &Option<SvelteNode>) -> Vec<SvelteFile> {
    // 1. Collect masters (document order, including masters nested in masters).
    let masters: Vec<&Element> = match root {
        Some(r) => collect_masters(r),
        None => Vec::new(),
    };

    // 2. Assign file names + tag names (unique, valid identifiers).
    let mut naming = Naming::default();
    let mut used_files: HashSet<String> = HashSet::new();
    let mut used_tags: HashSet<String> = HashSet::new();
    for m in &masters {
        naming.by_name.entry(m.name.clone()).or_insert(m.id);
        let file = unique_in(sanitize_name(&m.name, '-'), &mut used_files);
        let tag = unique_in(sanitize_name(&m.name, '_'), &mut used_tags);
        naming.tags.insert(m.id, tag);
        naming.files.insert(m.id, file);
        if let Some(it) = &m.interface {
            naming
                .slot_names
                .insert(m.id, it.slots.iter().map(|s| s.name.clone()).collect());
        }
    }

    // 3. One file per master, then App.svelte.
    let mut out: Vec<SvelteFile> = masters
        .iter()
        .map(|m| {
            let base = naming.file_of(m.id).unwrap_or("Component").to_string();
            SvelteFile {
                path: format!("components/{base}.svelte"),
                code: master_file(m, &naming),
            }
        })
        .collect();

    out.push(SvelteFile {
        path: "App.svelte".into(),
        code: app_file(root, &naming),
    });
    out
}

// ────────────────────────────────────────────────── tree scanning

fn collect_masters<'a>(node: &'a SvelteNode) -> Vec<&'a Element> {
    let mut out = Vec::new();
    walk_masters(node, &mut out);
    out
}

fn walk_masters<'a>(node: &'a SvelteNode, out: &mut Vec<&'a Element>) {
    if let SvelteNode::Element(e) = node {
        if e.component {
            out.push(e);
        }
    }
    for c in node.children() {
        walk_masters(c, out);
    }
}

/// True when the subtree contains a master or instance (so it needs the
/// project-aware walker instead of the shared single-subtree renderer).
fn contains_component_nodes(node: &SvelteNode) -> bool {
    if node.is_component() || node.is_instance() {
        return true;
    }
    node.children().iter().any(contains_component_nodes)
}

// ──────────────────────────────────────────────── file emitting

struct FileEmitter<'a> {
    ctx: Ctx,
    naming: &'a Naming,
    /// Master ids whose component tag was used in this file (for imports).
    used_masters: Vec<u64>,
    /// Slot containers of the master being emitted: (element id, name).
    /// Empty for App.svelte — only master files can host `<slot>`.
    slots: Vec<(u64, String)>,
}

impl<'a> FileEmitter<'a> {
    fn new(naming: &'a Naming) -> Self {
        Self {
            ctx: Ctx::new(),
            naming,
            used_masters: Vec::new(),
            slots: Vec::new(),
        }
    }

    fn emit_node(&mut self, node: &SvelteNode, parent: ParentBox, depth: usize, out: &mut String) {
        // Same absolute-node adjustment as the shared renderer.
        let parent = if is_absolute(node) {
            ParentBox::Free
        } else {
            parent
        };
        match node {
            // Masters are definitions — they do not render in the app.
            SvelteNode::Element(e) if e.component => {}
            // An instance renders as a component tag wrapped in the style of
            // the instance itself (its position/size on the page).
            SvelteNode::Element(e) if e.instance => self.emit_instance(e, parent, depth, out),
            // Plain subtrees reuse the shared renderer 1:1 — except slot
            // containers, whose children become the `<slot>` fallback.
            _ if !contains_component_nodes(node) && !self.is_slot_container(node) => {
                out.push_str(&node_markup(node, &mut self.ctx, depth, parent))
            }
            SvelteNode::Element(e) => self.emit_element(e, parent, depth, out),
            SvelteNode::Text(_) => out.push_str(&node_markup(node, &mut self.ctx, depth, parent)),
        }
    }

    /// Emits one element with its own class + style, then recurses.
    fn emit_element(&mut self, e: &Element, parent: ParentBox, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        let class = unique_class(&e.name, &mut self.ctx);
        self.ctx.styles.push(render_style(&class, &e.style, parent));
        push_hover_css(&mut self.ctx, &class, e);
        let tag = e.tag.as_str();
        let attrs = element_extra_attrs(e, &self.ctx);
        let child_parent = match e.style.display {
            Display::Flex if e.style.flex_dir == FlexDir::Row => ParentBox::FlexRow,
            Display::Flex => ParentBox::FlexCol,
            Display::Grid => ParentBox::Grid,
            Display::None => ParentBox::Free,
        };
        out.push_str(&wrap_prefix(e, &indent));
        // A slot container renders its children as the `<slot>` fallback
        // (design doc §3.2): consumers without content show the fallback.
        if let Some(name) = self.slot_name_of(e.id) {
            let attr = match self.bare_default_slot() {
                Some(n) if n == name => String::new(),
                _ => format!(" name=\"{name}\""),
            };
            out.push_str(&format!("{indent}<{tag} class=\"{class}\"{attrs}>\n"));
            out.push_str(&format!("{indent}  <slot{attr}>\n"));
            for c in &e.children {
                self.emit_node(c, child_parent, depth + 2, out);
            }
            out.push_str(&format!("{indent}  </slot>\n"));
        } else if e.children.is_empty() {
            out.push_str(&format!(
                "{indent}<{tag} class=\"{class}\"{attrs}></{tag}>\n"
            ));
        } else {
            out.push_str(&format!("{indent}<{tag} class=\"{class}\"{attrs}>\n"));
            for c in &e.children {
                self.emit_node(c, child_parent, depth + 1, out);
            }
            out.push_str(&format!("{indent}</{tag}>\n"));
        }
        out.push_str(&wrap_suffix(e, &indent));
    }

    /// Name of the slot whose container is `id`, if any.
    fn slot_name_of(&self, id: u64) -> Option<String> {
        self.slots
            .iter()
            .find(|(cid, _)| *cid == id)
            .map(|(_, n)| n.clone())
    }

    /// True when `node` is an element hosting one of this file's slots.
    fn is_slot_container(&self, node: &SvelteNode) -> bool {
        match node {
            SvelteNode::Element(e) => self.slot_name_of(e.id).is_some(),
            _ => false,
        }
    }

    /// Some(`name`) when the master has exactly one slot and it is named
    /// `default` — it emits as a bare `<slot>` (no attribute).
    fn bare_default_slot(&self) -> Option<String> {
        if self.slots.len() == 1 && self.slots[0].1 == "default" {
            Some(self.slots[0].1.clone())
        } else {
            None
        }
    }

    fn emit_instance(&mut self, e: &Element, parent: ParentBox, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        let class = unique_class(&format!("{}-inst", e.name), &mut self.ctx);
        self.ctx.styles.push(render_style(&class, &e.style, parent));
        push_hover_css(&mut self.ctx, &class, e);
        let attrs = element_extra_attrs(e, &self.ctx);

        let master_id = self.naming.by_name.get(&e.name).copied();
        let inst_attrs = instance_attrs(e);
        out.push_str(&wrap_prefix(e, &indent));
        out.push_str(&format!("{indent}<div class=\"{class}\"{attrs}>\n"));
        // Slot content rides inside the instance tag: each content root gets
        // `slot="name"` (a single `default` slot consumes bare content).
        let bare_default = master_id
            .and_then(|id| self.naming.slot_names.get(&id))
            .is_some_and(|names| names.len() == 1 && names[0] == "default");
        match master_id.and_then(|id| self.naming.tag_of(id).map(|t| (id, t))) {
            Some((id, tag)) if e.slot_content.is_empty() => {
                self.used_masters.push(id);
                out.push_str(&format!("{indent}  <{tag}{inst_attrs} />\n"));
            }
            Some((id, tag)) => {
                self.used_masters.push(id);
                out.push_str(&format!("{indent}  <{tag}{inst_attrs}>\n"));
                for sc in &e.slot_content {
                    for n in &sc.nodes {
                        // Bare content only for a single `default` slot.
                        let needs_attr = !(bare_default && sc.slot == "default");
                        match n {
                            SvelteNode::Element(_) => {
                                if needs_attr {
                                    self.ctx.slot_attrs.insert(n.id(), sc.slot.clone());
                                }
                                self.emit_node(n, ParentBox::Free, depth + 2, out);
                            }
                            SvelteNode::Text(_) => {
                                // Text nodes cannot carry attributes — wrap.
                                let content = match n {
                                    SvelteNode::Text(t) => escape_text(&t.content),
                                    _ => String::new(),
                                };
                                let attr = if needs_attr {
                                    format!(" slot=\"{}\"", sc.slot)
                                } else {
                                    String::new()
                                };
                                out.push_str(&format!(
                                    "{indent}    <span{attr}>{content}</span>\n"
                                ));
                            }
                        }
                    }
                }
                out.push_str(&format!("{indent}  </{tag}>\n"));
            }
            None => out.push_str(&format!(
                "{indent}  <!-- master \"{}\" not found -->\n",
                e.name
            )),
        }
        out.push_str(&format!("{indent}</div>\n"));
        out.push_str(&wrap_suffix(e, &indent));
    }
}

fn is_absolute(node: &SvelteNode) -> bool {
    matches!(
        node,
        SvelteNode::Element(e) if e.style.position == Position::Absolute
    )
}

/// Overridden props become attributes; non-overridden props keep the
/// master's defaults, so only overrides are exported.
fn instance_attrs(e: &Element) -> String {
    let mut s = String::new();
    for o in &e.overrides {
        s.push_str(&format!(
            " {}=\"{}\"",
            sanitize_name(&o.prop, '_'),
            escape_attr(&o.value)
        ));
    }
    s
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('\n', "&#10;")
}

// ──────────────────────────────────────────────── file assembly

fn app_file(root: &Option<SvelteNode>, naming: &Naming) -> String {
    let mut em = FileEmitter::new(naming);
    let mut body = String::new();
    let mut state_lines: Vec<String> = Vec::new();
    match root {
        None => body.push_str("  <!-- empty -->\n"),
        Some(r) if r.is_component() => body.push_str("  <!-- root is a component master -->\n"),
        Some(r) => {
            // Page-level behaviors (masters/instances stay black boxes).
            state_lines = collect_behavior(r, &mut em.ctx);
            em.emit_node(r, ParentBox::Free, 0, &mut body);
        }
    }
    let imports = import_lines(&em.used_masters, naming, "./components/");
    assemble(&imports, None, &state_lines, &em.ctx, &body, &em.ctx.styles)
}

fn master_file(m: &Element, naming: &Naming) -> String {
    let mut em = FileEmitter::new(naming);

    // Props + events → one `$props()` destructure (Svelte 5). Events become
    // optional callback props (`onconfirm`), called by the Emit action.
    let mut idents: Vec<String> = Vec::new();
    let mut types: Vec<String> = Vec::new();
    let mut used_props: HashSet<String> = HashSet::new();
    if let Some(it) = &m.interface {
        for p in &it.props {
            let ident = unique_in(sanitize_name(&p.name, '_'), &mut used_props);
            match p.kind {
                PropKind::Text => {
                    em.ctx.text_bindings.insert(p.target, ident.clone());
                }
                PropKind::Fill => {
                    em.ctx.fill_bindings.insert(p.target, ident.clone());
                }
                PropKind::TextColor => {
                    em.ctx.color_bindings.insert(p.target, ident.clone());
                }
            }
            idents.push(format!("{} = {}", ident, json_str(&p.default)));
            types.push(format!("{ident}?: string"));
        }
        for ev in &it.events {
            let ident = unique_in(event_callback_name(ev), &mut used_props);
            types.push(format!("{ident}?: () => void"));
            idents.push(ident);
        }
    }
    let props_line = if idents.is_empty() {
        None
    } else {
        Some(format!(
            "let {{ {} }}: {{ {} }} = $props();",
            idents.join(", "),
            types.join("; ")
        ))
    };

    // The master root carries the component's box. Its on-canvas placement
    // (absolute + top/left) belongs to the page, not to the definition, so it
    // is stripped here — instances get their placement from the page.
    let mut root_el = m.clone();
    if root_el.style.position == Position::Absolute {
        root_el.style.position = Position::Relative;
        root_el.style.top = None;
        root_el.style.right = None;
        root_el.style.bottom = None;
        root_el.style.left = None;
    }

    // Component behaviors → $state / onMount / handler attributes.
    let state_lines = collect_behavior(&SvelteNode::Element(root_el.clone()), &mut em.ctx);

    // Slot containers of this master: their children become the `<slot>`
    // fallback (Phase 4, design doc §3.2).
    if let Some(it) = &m.interface {
        em.slots = it
            .slots
            .iter()
            .map(|s| (s.container, s.name.clone()))
            .collect();
    }

    let mut body = String::new();
    em.emit_element(&root_el, ParentBox::Free, 0, &mut body);

    let imports = import_lines(&em.used_masters, naming, "./");
    assemble(
        &imports,
        props_line,
        &state_lines,
        &em.ctx,
        &body,
        &em.ctx.styles,
    )
}

/// `import Tag from "<dir><File>.svelte";` — sorted, deduplicated. `dir` is
/// the relative path to the `components/` directory from the importing file.
fn import_lines(used: &[u64], naming: &Naming, dir: &str) -> Vec<String> {
    let mut ids: Vec<u64> = used.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids.iter()
        .filter_map(|id| {
            let tag = naming.tag_of(*id)?;
            let file = naming.file_of(*id)?;
            Some(format!("import {tag} from \"{dir}{file}.svelte\";"))
        })
        .collect()
}

/// File assembly (Svelte 5): imports, `$props()`, `$state` runes and the
/// script-level rule bodies (`onMount` / `$effect`).
fn assemble(
    imports: &[String],
    props_line: Option<String>,
    state_lines: &[String],
    ctx: &Ctx,
    body: &str,
    styles: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("<script lang=\"ts\">\n  // generated by Vision Design\n");
    for line in imports {
        out.push_str(&format!("  {line}\n"));
    }
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
    if props_line.is_some() || !state_lines.is_empty() {
        out.push('\n');
    }
    if let Some(p) = props_line {
        out.push_str(&format!("  {p}\n"));
    }
    for l in state_lines {
        out.push_str(&format!("  {l}\n"));
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
    out.push_str(body);
    out.push_str("\n<style>\n");
    for s in styles {
        out.push_str(s);
    }
    out.push_str("</style>\n");
    out
}

// ─────────────────────────────────────────────────── identifiers

/// File/tag-safe identifier: alphanumerics kept, the rest becomes `sep`,
/// edges trimmed, never empty and never starting with a digit.
fn sanitize_name(name: &str, sep: char) -> String {
    let mut s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { sep })
        .collect();
    while s.starts_with(sep) {
        s.remove(0);
    }
    while s.ends_with(sep) {
        s.pop();
    }
    if s.is_empty() {
        s = "Component".into();
    }
    if s.chars().next().map_or(true, |c| c.is_ascii_digit()) {
        s = format!("C{s}");
    }
    s
}

fn unique_in(base: String, used: &mut HashSet<String>) -> String {
    if used.insert(base.clone()) {
        return base;
    }
    let mut i = 2;
    loop {
        let cand = format!("{base}{i}");
        if used.insert(cand.clone()) {
            return cand;
        }
        i += 1;
    }
}

// ─────────────────────────────────────────────────────── tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::demo::demo_tree;

    fn find<'a>(files: &'a [SvelteFile], path: &str) -> &'a str {
        files
            .iter()
            .find(|f| f.path == path)
            .map(|f| f.code.as_str())
            .unwrap_or_else(|| panic!("missing file {path}"))
    }

    #[test]
    fn demo_exports_app_and_component() {
        let files = export_project(&Some(demo_tree()));
        assert_eq!(files.len(), 2, "App.svelte + components/Overlay.svelte");

        let app = find(&files, "App.svelte");
        assert!(
            app.contains("import Overlay from \"./components/Overlay.svelte\";"),
            "App imports the component:\n{app}"
        );
        assert!(
            app.contains("<Overlay"),
            "the instance renders as a tag:\n{app}"
        );
        assert!(
            app.contains("title=\"Usunąć plik?\""),
            "text override becomes an attribute:\n{app}"
        );
        assert!(
            app.contains("accent=\"#42BE65\""),
            "color override becomes an attribute:\n{app}"
        );
        assert!(
            !app.contains("Okno dialogowe"),
            "master defaults live in the component file, not in App:\n{app}"
        );

        let overlay = find(&files, "components/Overlay.svelte");
        assert!(
            overlay.contains("let { title = \"Okno dialogowe\""),
            "props are declared with $props() (Svelte 5):\n{overlay}"
        );
        assert!(
            overlay.contains("= $props();"),
            "the destructure ends with the rune:\n{overlay}"
        );
        assert!(
            overlay.contains("onconfirm?: () => void"),
            "interface events become optional callback props:\n{overlay}"
        );
        assert!(
            overlay.contains("accent = \"#FA4D56\""),
            "color prop default:\n{overlay}"
        );
        assert!(
            overlay.contains("{title}"),
            "bound text renders as an expression:\n{overlay}"
        );
        assert!(
            overlay.contains("style=\"background: accent\""),
            "bound fill renders as an inline style:\n{overlay}"
        );
        assert!(
            !overlay.contains("position: absolute"),
            "the master root must not bake in the canvas placement:\n{overlay}"
        );
    }

    #[test]
    fn slots_export_as_holes_and_named_content() {
        // Master Card { div Body(3) { text } }; slot "default" -> Body.
        let mut body = crate::model::svelte_ast::new_rect(3, 0.0, 0.0, 120.0, 40.0, 0x1B1B1B);
        if let crate::model::svelte_ast::SvelteNode::Element(e) = &mut body {
            e.style.position = crate::model::svelte_ast::Position::Relative;
            e.children = vec![crate::model::svelte_ast::new_text(4, 0.0, 0.0, "fallback")];
        }
        let mut master = crate::model::svelte_ast::new_frame(1, 0.0, 0.0, 200.0, 100.0);
        if let crate::model::svelte_ast::SvelteNode::Element(e) = &mut master {
            e.component = true;
            e.name = "Card".into();
            e.style.position = crate::model::svelte_ast::Position::Relative;
            e.children = vec![body];
            e.interface = Some(crate::model::svelte_ast::ComponentInterface {
                slots: vec![crate::model::svelte_ast::SlotDef {
                    name: "default".into(),
                    container: 3,
                }],
                ..Default::default()
            });
        }
        let master = match master {
            crate::model::svelte_ast::SvelteNode::Element(e) => e,
            _ => unreachable!(),
        };
        // Page: Card instance with the Row(9) provided as slot content.
        let content = crate::model::svelte_ast::new_rect(9, 0.0, 0.0, 60.0, 20.0, 0x0D99FF);
        let mut next = 10u64;
        let inst = crate::model::component_ops::instantiate(
            &master,
            vec![],
            vec![crate::model::svelte_ast::SlotContent {
                slot: "default".into(),
                nodes: vec![content],
            }],
            &mut next,
        );
        let mut page = crate::model::svelte_ast::new_frame(5, 0.0, 0.0, 500.0, 400.0);
        if let crate::model::svelte_ast::SvelteNode::Element(e) = &mut page {
            e.name = "Page".into();
            e.children = vec![crate::model::svelte_ast::SvelteNode::Element(master), inst];
        }
        let root = Some(page);

        let files = export_project(&root);
        let card = find(&files, "components/Card.svelte");
        assert!(
            card.contains("<slot>"),
            "the single default slot emits bare:\n{card}"
        );
        assert!(
            card.contains("fallback"),
            "the container children become the fallback:\n{card}"
        );
        assert!(card.contains("</slot>"), "the fallback is closed:\n{card}");

        let app = find(&files, "App.svelte");
        assert!(
            app.contains("<Card") && app.contains("</Card>"),
            "the instance tag opens and closes (content inside):\n{app}"
        );
        assert!(
            !app.contains("slot="),
            "bare content fills a single default slot:\n{app}"
        );
    }

    #[test]
    fn export_is_deterministic() {
        let root = Some(demo_tree());
        let a = export_project(&root);
        let b = export_project(&root);
        assert_eq!(a.len(), b.len());
        for (fa, fb) in a.iter().zip(b.iter()) {
            assert_eq!(fa.path, fb.path);
            assert_eq!(fa.code, fb.code);
        }
    }

    #[test]
    fn project_without_masters_is_a_single_file() {
        let mut rect = new_rect(2, 0.0, 0.0, 50.0, 50.0, 0x0D99FF);
        if let SvelteNode::Element(e) = &mut rect {
            e.style.position = Position::Absolute;
            e.style.left = Some(0.0);
            e.style.top = Some(0.0);
        }
        let mut root = new_frame(1, 0.0, 0.0, 200.0, 200.0);
        if let SvelteNode::Element(e) = &mut root {
            e.children = vec![rect];
        }
        let files = export_project(&Some(root));
        assert_eq!(files.len(), 1, "no components → just App.svelte");
        assert!(files[0].code.contains("class=\""));
    }

    #[test]
    fn empty_project_still_produces_app() {
        let files = export_project(&None);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "App.svelte");
        assert!(files[0].code.contains("<!-- empty -->"));
    }

    #[test]
    fn master_names_are_sanitized_and_unique() {
        assert_eq!(sanitize_name("Okno dialogowe!", '-'), "Okno-dialogowe");
        assert_eq!(sanitize_name("2Panel", '_'), "C2Panel");
        assert_eq!(sanitize_name("  ---  ", '-'), "Component");

        let mut used = HashSet::new();
        assert_eq!(unique_in("Card".into(), &mut used), "Card");
        assert_eq!(unique_in("Card".into(), &mut used), "Card2");
    }
}
