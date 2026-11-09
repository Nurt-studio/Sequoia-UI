//! Behavior model (Phase 2): local component state, event rules, bindings
//! and the CSS hover state — design doc §3.3, §5.2 and §6.1.
//!
//! Rules live on the element that triggers them (`Element::behavior`), while
//! variables live on the scope owner: the nearest master component on the
//! path to the root, or the document root when the element is not inside a
//! component. A rule may only reference variables visible in its scope, and
//! Show/Hide may only target elements inside the same scope subtree — the
//! closed dictionary plus these two limits keep the model spaghetti-free;
//! the node graph (Phase 3) visualizes it and checks the dataflow for cycles.

// ─────────────────────────────────────────────────────────── values

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Number(f32),
    Str(String),
}

impl Value {
    /// Human-readable form used in rule rows (`false`, `3`, `"Okno"`).
    pub fn label(&self) -> String {
        match self {
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            Value::Str(s) => format!("\"{s}\""),
        }
    }
    /// JavaScript truthiness (`{#if …}` / the Present interpreter).
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
        }
    }
    /// JavaScript literal (Svelte 5 codegen).
    pub fn to_js(&self) -> String {
        match self {
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            Value::Str(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        }
    }
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::Str(_) => "string",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarType {
    Bool,
    Number,
    String,
}

impl VarType {
    pub fn as_str(self) -> &'static str {
        match self {
            VarType::Bool => "bool",
            VarType::Number => "number",
            VarType::String => "string",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "bool" => VarType::Bool,
            "number" => VarType::Number,
            "string" => VarType::String,
            _ => return None,
        })
    }
    pub fn default_value(self) -> Value {
        match self {
            VarType::Bool => Value::Bool(false),
            VarType::Number => Value::Number(0.0),
            VarType::String => Value::Str(String::new()),
        }
    }
}

/// A piece of local component state (`let open = $state(false)` in Svelte 5).
#[derive(Clone, Debug, PartialEq)]
pub struct Var {
    pub name: String,
    pub ty: VarType,
    pub initial: Value,
}

impl Var {
    pub fn new(name: &str, ty: VarType) -> Self {
        Self {
            name: name.to_string(),
            ty,
            initial: ty.default_value(),
        }
    }
    pub fn label(&self) -> String {
        format!(
            "{} {} = {}",
            self.ty.as_str(),
            self.name,
            self.initial.label()
        )
    }
}

/// Right-hand side of `SetVar`: a literal, another variable or the payload
/// of an incoming event (payload is wired up in Phase 4).
#[derive(Clone, Debug, PartialEq)]
pub enum ValueExpr {
    Value(Value),
    Var(String),
    /// Payload of the event that fired the rule (`payload` in codegen).
    Payload,
}

impl ValueExpr {
    pub fn label(&self) -> String {
        match self {
            ValueExpr::Value(v) => v.label(),
            ValueExpr::Var(n) => n.clone(),
            ValueExpr::Payload => "payload".into(),
        }
    }
    pub fn to_js(&self) -> String {
        match self {
            ValueExpr::Value(v) => v.to_js(),
            ValueExpr::Var(n) => n.clone(),
            ValueExpr::Payload => "payload".into(),
        }
    }
}

// ────────────────────────────────────────────────────────── triggers

/// What fires a rule. The rule belongs to the element the event happens on.
#[derive(Clone, Debug, PartialEq)]
pub enum Trigger {
    OnClick,
    OnHoverIn,
    OnHoverOut,
    OnMount,
    OnKey(String),
    OnPropChanged(String),
    OnEvent(String),
}

impl Trigger {
    /// Short kind id used by serialization and the UI picker.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Trigger::OnClick => "click",
            Trigger::OnHoverIn => "hover_in",
            Trigger::OnHoverOut => "hover_out",
            Trigger::OnMount => "mount",
            Trigger::OnKey(_) => "key",
            Trigger::OnPropChanged(_) => "prop",
            Trigger::OnEvent(_) => "event",
        }
    }
    pub fn param(&self) -> Option<&str> {
        match self {
            Trigger::OnKey(k) | Trigger::OnPropChanged(k) | Trigger::OnEvent(k) => Some(k),
            _ => None,
        }
    }
    /// Row label in the rules list (`On Click`, `On Key (Enter)`, …).
    pub fn label(&self) -> String {
        match self {
            Trigger::OnClick => "On Click".into(),
            Trigger::OnHoverIn => "On Hover In".into(),
            Trigger::OnHoverOut => "On Hover Out".into(),
            Trigger::OnMount => "On Mount".into(),
            Trigger::OnKey(k) => format!("On Key ({k})"),
            Trigger::OnPropChanged(p) => format!("On Prop Changed ({p})"),
            Trigger::OnEvent(e) => format!("On Event ({e})"),
        }
    }
    /// DOM attribute generated for this trigger, when it is an attribute.
    pub fn dom_attr(&self) -> Option<&'static str> {
        match self {
            Trigger::OnClick => Some("onclick"),
            Trigger::OnHoverIn => Some("onmouseenter"),
            Trigger::OnHoverOut => Some("onmouseleave"),
            Trigger::OnKey(_) => Some("onkeydown"),
            Trigger::OnMount | Trigger::OnPropChanged(_) | Trigger::OnEvent(_) => None,
        }
    }
}

// ───────────────────────────────────────────────────────── transitions

/// Svelte transition used by bindings and Show/Hide (design doc §4.5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Transition {
    Fade { ms: u32 },
    Slide { ms: u32 },
    Scale { ms: u32 },
}

impl Transition {
    pub fn kind_name(self) -> &'static str {
        match self {
            Transition::Fade { .. } => "fade",
            Transition::Slide { .. } => "slide",
            Transition::Scale { .. } => "scale",
        }
    }
    pub fn ms(self) -> u32 {
        match self {
            Transition::Fade { ms } | Transition::Slide { ms } | Transition::Scale { ms } => ms,
        }
    }
    pub fn with_ms(self, ms: u32) -> Self {
        match self {
            Transition::Fade { .. } => Transition::Fade { ms },
            Transition::Slide { .. } => Transition::Slide { ms },
            Transition::Scale { .. } => Transition::Scale { ms },
        }
    }
    pub fn from_kind(kind: &str, ms: u32) -> Option<Self> {
        Some(match kind {
            "fade" => Transition::Fade { ms },
            "slide" => Transition::Slide { ms },
            "scale" => Transition::Scale { ms },
            _ => return None,
        })
    }
    /// `slide={{ duration: 200 }}` — the Svelte directive value.
    pub fn to_svelte(self) -> String {
        format!("{}={{{{ duration: {} }}}}", self.kind_name(), self.ms())
    }
    /// Import name for the `svelte/transition` import line.
    pub fn import_name(self) -> &'static str {
        self.kind_name()
    }
}

// ─────────────────────────────────────────────────────────── actions

/// One step of a rule. The closed dictionary from design doc §3.3 —
/// nothing else can be expressed, which is the anti-spaghetti guarantee.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    ToggleVar(String),
    SetVar(String, ValueExpr),
    Show {
        target: String,
        transition: Option<Transition>,
    },
    Hide {
        target: String,
        transition: Option<Transition>,
    },
    /// `onconfirm?.()` — callback props in Svelte 5 (design doc §6.1).
    Emit(String),
    /// `setTimeout(() => { … }, ms)` — the nested steps run when the
    /// timer fires; the node graph renders them hanging off the Delay
    /// node's `after` output (Phase 3).
    Delay(u32, Vec<Action>),
}

impl Action {
    /// Row label in the rules list (`Toggle open`, `Show "Panel"`, …).
    pub fn label(&self) -> String {
        match self {
            Action::ToggleVar(v) => format!("Toggle {v}"),
            Action::SetVar(v, e) => format!("Set {} = {}", v, e.label()),
            Action::Show { target, .. } => format!("Show \"{target}\""),
            Action::Hide { target, .. } => format!("Hide \"{target}\""),
            Action::Emit(e) => format!("Emit {e}"),
            Action::Delay(ms, _) => format!("Delay {} ms", ms),
        }
    }
    /// Kind id used by the UI add-buttons and serialization.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Action::ToggleVar(_) => "toggle",
            Action::SetVar(..) => "set",
            Action::Show { .. } => "show",
            Action::Hide { .. } => "hide",
            Action::Emit(_) => "emit",
            Action::Delay(..) => "delay",
        }
    }
}

/// Trigger → actions. Lives on the element the trigger fires on.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub trigger: Trigger,
    pub actions: Vec<Action>,
}

impl Rule {
    pub fn new(trigger: Trigger) -> Self {
        Self {
            trigger,
            actions: Vec::new(),
        }
    }
    /// `On Click → Toggle open` — single-line summary for the rules list.
    pub fn summary(&self) -> String {
        let acts = if self.actions.is_empty() {
            "…".to_string()
        } else {
            self.actions
                .iter()
                .map(|a| a.label())
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!("{} → {}", self.trigger.label(), acts)
    }
}

// ─────────────────────────────────────────────────────────── behavior

/// Local state + rules of one element (design doc §3.3). Vars are
/// conventionally empty on non-master elements — they live on the scope
/// owner — but the model does not enforce it, so imported documents keep
/// working either way.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Behavior {
    pub vars: Vec<Var>,
    pub rules: Vec<Rule>,
}

impl Behavior {
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty() && self.rules.is_empty()
    }
    pub fn var(&self, name: &str) -> Option<&Var> {
        self.vars.iter().find(|v| v.name == name)
    }
}

/// Conditional visibility of an element: rendered as `{#if var}` with an
/// optional transition (design doc §5.2, BINDING section).
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub var: String,
    pub transition: Option<Transition>,
}

/// Pure-CSS hover state (design doc §6.1: styling hover stays CSS, only
/// hover that *does something* becomes JS via OnHoverIn/Out rules).
/// Colors are hex strings (`#3A3A3E`) like `fill_hex`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HoverStyle {
    pub fill: Option<String>,
    pub color: Option<String>,
    pub opacity: Option<f32>,
}

impl HoverStyle {
    pub fn is_empty(&self) -> bool {
        self.fill.is_none() && self.color.is_none() && self.opacity.is_none()
    }
}

// ────────────────────────────────────────────── scope resolution

use crate::model::svelte_ast::SvelteNode;

/// Nearest ancestor-or-self of `element_id` that is a master component;
/// `None` when the element is not inside any component.
fn component_ancestor<'a>(root: &'a SvelteNode, element_id: u64) -> Option<&'a SvelteNode> {
    let mut cursor = crate::model::tree_ops::find(root, element_id)?;
    loop {
        if cursor.is_component() {
            return Some(cursor);
        }
        cursor = crate::model::tree_ops::parent_of(root, cursor.id())?;
    }
}

/// The subtree a rule on `element_id` may reach: the nearest component
/// ancestor-or-self, or the whole document. "The graph only sees the
/// children of its own component" (design doc §4.4).
pub fn scope_root<'a>(root: &'a SvelteNode, element_id: u64) -> &'a SvelteNode {
    component_ancestor(root, element_id).unwrap_or(root)
}

/// Where the variables visible to `element_id` live: the nearest master
/// component on the path, or the document root element.
pub fn var_owner_id(root: &SvelteNode, element_id: u64) -> u64 {
    match component_ancestor(root, element_id) {
        Some(c) => c.id(),
        None => root.id(),
    }
}

/// Variables visible to `element_id`, nearest owner first. Name conflicts
/// are resolved in favor of the nearer owner.
pub fn scope_vars(root: &SvelteNode, element_id: u64) -> Vec<Var> {
    let mut out: Vec<Var> = Vec::new();
    let mut push_unique = |vars: &[Var]| {
        for v in vars {
            if !out.iter().any(|o| o.name == v.name) {
                out.push(v.clone());
            }
        }
    };
    // Climb from the element to the root, collecting every behavior on the
    // path (self first, then ancestors up to and including the scope root).
    let Ok(mut chain) = ancestor_chain(root, element_id) else {
        return out;
    };
    for id in chain.drain(..) {
        if let Some(SvelteNode::Element(e)) = crate::model::tree_ops::find(root, id) {
            if let Some(b) = &e.behavior {
                push_unique(&b.vars);
            }
        }
    }
    out
}

/// Ids from `element_id` up to and including the root (self first).
fn ancestor_chain(root: &SvelteNode, element_id: u64) -> Result<Vec<u64>, ()> {
    let mut chain = Vec::new();
    let mut cursor = crate::model::tree_ops::find(root, element_id).ok_or(())?;
    loop {
        chain.push(cursor.id());
        match crate::model::tree_ops::parent_of(root, cursor.id()) {
            Some(p) => cursor = p,
            None => break,
        }
    }
    Ok(chain)
}

/// Elements reachable from `node` (self included), document order:
/// `(id, name, has_binding)` — the Show/Hide target picker.
pub fn descendant_targets(node: &SvelteNode) -> Vec<(u64, String, bool)> {
    let mut out = Vec::new();
    if let SvelteNode::Element(e) = node {
        out.push((e.id, e.name.clone(), e.binding.is_some()));
    }
    for c in node.children() {
        out.extend(descendant_targets(c));
    }
    out
}

/// Finds an element by name inside `scope` (document order, first match).
pub fn find_target_by_name<'a>(scope: &'a SvelteNode, name: &str) -> Option<&'a SvelteNode> {
    if let SvelteNode::Element(e) = scope {
        if e.name == name {
            return Some(scope);
        }
    }
    for c in scope.children() {
        if let Some(f) = find_target_by_name(c, name) {
            return Some(f);
        }
    }
    None
}

/// Resolve a Show/Hide target: exact name first, then case-insensitive —
/// the code round trip normalizes names to CSS classes (`Panel` → `panel`),
/// and rules must keep working on imported documents.
pub fn resolve_target<'a>(scope: &'a SvelteNode, name: &str) -> Option<&'a SvelteNode> {
    if let Some(hit) = find_target_by_name(scope, name) {
        return Some(hit);
    }
    find_target_by_name_ci(scope, name)
}

fn find_target_by_name_ci<'a>(scope: &'a SvelteNode, name: &str) -> Option<&'a SvelteNode> {
    let lower = name.to_ascii_lowercase();
    if let SvelteNode::Element(e) = scope {
        if e.name.to_ascii_lowercase() == lower {
            return Some(scope);
        }
    }
    for c in scope.children() {
        if let Some(f) = find_target_by_name_ci(c, name) {
            return Some(f);
        }
    }
    None
}

/// The binding variable of the element named `name` inside `scope`, if it
/// has one. Show/Hide codegen resolves through this.
pub fn binding_var_of(scope: &SvelteNode, name: &str) -> Option<String> {
    match resolve_target(scope, name) {
        Some(SvelteNode::Element(e)) => e.binding.as_ref().map(|b| b.var.clone()),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────── validation

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warn,
    Info,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    /// Index of the offending rule, when the issue is rule-local.
    pub rule: Option<usize>,
}

pub(crate) fn severity_mark(s: Severity) -> &'static str {
    match s {
        Severity::Error => "✕",
        Severity::Warn => "!",
        Severity::Info => "i",
    }
}

/// JS identifier check for variable names.
pub fn is_valid_var_name(name: &str) -> bool {
    const RESERVED: &[&str] = &[
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "function",
        "if",
        "import",
        "in",
        "instanceof",
        "new",
        "null",
        "return",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "let",
        "static",
        "yield",
        "await",
    ];
    let mut chars = name.chars();
    let ok_first = chars
        .next()
        .map(|c| c.is_alphabetic() || c == '_' || c == '$')
        .unwrap_or(false);
    ok_first
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        && !RESERVED.contains(&name)
}

/// Validates the rules of `element_id` (the element's own `Behavior`) plus
/// its binding against the visible scope. Runs in the Interact tab — the
/// model is checked *before* the graph (design doc §9, item 7).
pub fn validate(root: &SvelteNode, element_id: u64) -> Vec<Issue> {
    let mut issues = Vec::new();
    let Some(SvelteNode::Element(el)) = crate::model::tree_ops::find(root, element_id) else {
        return issues;
    };
    let scope = scope_root(root, element_id);
    let vars = scope_vars(root, element_id);
    let var = |name: &str| vars.iter().find(|v| v.name == name).cloned();

    // Binding checks.
    if let Some(b) = &el.binding {
        match var(&b.var) {
            None => issues.push(Issue {
                severity: Severity::Error,
                message: format!("Binding: variable \"{}\" is not in scope", b.var),
                rule: None,
            }),
            Some(v) => {
                if v.ty != VarType::Bool {
                    issues.push(Issue {
                        severity: Severity::Warn,
                        message: format!(
                            "Binding: \"{}\" is {}, {{#if}} treats it as truthy",
                            b.var,
                            v.ty.as_str()
                        ),
                        rule: None,
                    });
                }
            }
        }
    }

    // Var sanity (only when this element owns vars — masters and roots).
    if let Some(b) = &el.behavior {
        let mut seen: Vec<&str> = Vec::new();
        for v in &b.vars {
            if !is_valid_var_name(&v.name) {
                issues.push(Issue {
                    severity: Severity::Error,
                    message: format!("Variable \"{}\": invalid name", v.name),
                    rule: None,
                });
            }
            if seen.contains(&v.name.as_str()) {
                issues.push(Issue {
                    severity: Severity::Error,
                    message: format!("Duplicate variable \"{}\"", v.name),
                    rule: None,
                });
            }
            seen.push(&v.name);
            let init_ty = v.initial.type_name();
            if init_ty != v.ty.as_str() {
                issues.push(Issue {
                    severity: Severity::Error,
                    message: format!(
                        "Variable \"{}\": initial {} ≠ {}",
                        v.name,
                        init_ty,
                        v.ty.as_str()
                    ),
                    rule: None,
                });
            }
        }
    }

    let Some(behavior) = &el.behavior else {
        return issues;
    };

    // Rule checks.
    for (ri, rule) in behavior.rules.iter().enumerate() {
        if rule.actions.is_empty() {
            issues.push(Issue {
                severity: Severity::Warn,
                message: format!("{}: rule has no actions", rule.trigger.label()),
                rule: Some(ri),
            });
        }
        match &rule.trigger {
            Trigger::OnKey(k) if k.trim().is_empty() => issues.push(Issue {
                severity: Severity::Warn,
                message: "On Key without a key name".into(),
                rule: Some(ri),
            }),
            Trigger::OnEvent(e) => {
                let msg = if el.instance {
                    format!(
                        "On Event ({e}): fires when the child emits — codegen adds on:{e} to the instance tag"
                    )
                } else {
                    format!(
                        "On Event ({e}): has no source — put it on an instance (◈) of a component that emits {e}"
                    )
                };
                issues.push(Issue {
                    severity: Severity::Info,
                    message: msg,
                    rule: Some(ri),
                });
            }
            Trigger::OnPropChanged(p) => {
                let known = el
                    .interface
                    .as_ref()
                    .map(|it| it.prop(p).is_some())
                    .unwrap_or(false);
                if !known {
                    issues.push(Issue {
                        severity: Severity::Warn,
                        message: format!("On Prop Changed: prop \"{p}\" is not in the interface"),
                        rule: Some(ri),
                    });
                }
            }
            _ => {}
        }
        for action in &rule.actions {
            match action {
                Action::ToggleVar(name) => match var(name) {
                    None => issues.push(Issue {
                        severity: Severity::Error,
                        message: format!("Toggle: variable \"{name}\" is not in scope"),
                        rule: Some(ri),
                    }),
                    Some(v) if v.ty != VarType::Bool => issues.push(Issue {
                        severity: Severity::Error,
                        message: format!("Toggle: \"{}\" is {} — bool only", name, v.ty.as_str()),
                        rule: Some(ri),
                    }),
                    _ => {}
                },
                Action::SetVar(name, expr) => {
                    let Some(v) = var(name) else {
                        issues.push(Issue {
                            severity: Severity::Error,
                            message: format!("Set: variable \"{name}\" is not in scope"),
                            rule: Some(ri),
                        });
                        continue;
                    };
                    let value_ty: Option<String> = match expr {
                        ValueExpr::Value(val) => Some(val.type_name().to_string()),
                        ValueExpr::Var(other) => var(other).map(|o| o.ty.as_str().to_string()),
                        ValueExpr::Payload => None,
                    };
                    if let Some(t) = value_ty
                        && t != v.ty.as_str()
                    {
                        issues.push(Issue {
                            severity: Severity::Error,
                            message: format!("Set {}: {} ≠ {} ({t})", name, t, v.ty.as_str()),
                            rule: Some(ri),
                        });
                    }
                }
                Action::Show { target, .. } | Action::Hide { target, .. } => {
                    match resolve_target(scope, target) {
                        None => issues.push(Issue {
                            severity: Severity::Error,
                            message: format!(
                                "Target \"{target}\" is out of scope (own component children only)"
                            ),
                            rule: Some(ri),
                        }),
                        Some(t) => {
                            let has_binding =
                                matches!(t, SvelteNode::Element(e) if e.binding.is_some());
                            if !has_binding {
                                issues.push(Issue {
                                    severity: Severity::Warn,
                                    message: format!(
                                        "Target \"{target}\" has no Binding — show/hide won't work"
                                    ),
                                    rule: Some(ri),
                                });
                            }
                        }
                    }
                }
                Action::Emit(e) => {
                    let known = el
                        .interface
                        .as_ref()
                        .map(|it| it.events.iter().any(|x| x == e))
                        .unwrap_or(false);
                    if el.component && !known {
                        issues.push(Issue {
                            severity: Severity::Warn,
                            message: format!("Emit: \"{e}\" is not in the master's interface"),
                            rule: Some(ri),
                        });
                    }
                }
                Action::Delay(..) => issues.push(Issue {
                    severity: Severity::Info,
                    message: "Delay: runs the nested steps after the timer (setTimeout)".into(),
                    rule: Some(ri),
                }),
            }
        }
    }
    issues
}

// ──────────────────────────────────────────────────────────── tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::svelte_ast::*;

    /// root Page(100){ master Card(200){ Btn(300) }, Btn(400) } — Btn(400) is
    /// outside any component, so its scope root is the whole document. Ids
    /// are far apart on purpose: `new_button` also assigns the label text
    /// id + 1, which must not collide with the other nodes.
    fn sample() -> SvelteNode {
        let mut btn_in = new_button(300, 0.0, 0.0);
        let mut card = new_frame(200, 0.0, 0.0, 100.0, 40.0);
        if let SvelteNode::Element(e) = &mut card {
            e.component = true;
            e.name = "Card".into();
        }
        if let SvelteNode::Element(e) = &mut btn_in {
            e.name = "Btn".into();
        }
        let mut btn_out = new_button(400, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut btn_out {
            e.name = "Btn".into();
        }
        let mut root = new_frame(100, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.name = "Page".into();
            if let SvelteNode::Element(c) = &mut card {
                c.children = vec![btn_in];
            }
            e.children = vec![card, btn_out];
        }
        root
    }

    #[test]
    fn scope_owner_prefers_the_master_then_the_root() {
        let root = sample();
        assert_eq!(
            var_owner_id(&root, 300),
            200,
            "inside Card → Card owns vars"
        );
        assert_eq!(
            var_owner_id(&root, 400),
            100,
            "outside → document root owns vars"
        );
        assert_eq!(
            var_owner_id(&root, 200),
            200,
            "the master owns its own vars"
        );
    }

    #[test]
    fn scope_vars_climb_the_chain_and_dedupe() {
        let mut root = sample();
        if let SvelteNode::Element(e) = &mut root {
            e.behavior = Some(Behavior {
                vars: vec![Var::new("open", VarType::Bool)],
                rules: vec![],
            });
        }
        if let Some(SvelteNode::Element(card)) = crate::model::tree_ops::find_mut(&mut root, 200) {
            card.behavior = Some(Behavior {
                vars: vec![
                    Var::new("open", VarType::Bool),
                    Var::new("count", VarType::Number),
                ],
                rules: vec![],
            });
        }
        // Btn(300): Card's "open" shadows the root one, then root's "open" is
        // skipped as a duplicate.
        let vars = scope_vars(&root, 300);
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].name, "open");
        assert_eq!(vars[1].name, "count");
    }

    #[test]
    fn targets_stop_at_the_component_boundary() {
        let root = sample();
        let scope = scope_root(&root, 300);
        let targets = descendant_targets(scope);
        let ids: Vec<u64> = targets.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(
            ids,
            vec![200, 300],
            "Btn(400) belongs to the page, not to Card"
        );
        let page_scope = scope_root(&root, 400);
        assert_eq!(page_scope.id(), 100);
    }

    #[test]
    fn validation_catches_unknown_and_mistyped_vars() {
        let mut root = sample();
        if let SvelteNode::Element(e) = &mut root {
            e.behavior = Some(Behavior {
                vars: vec![Var::new("open", VarType::Bool)],
                rules: vec![],
            });
        }
        if let Some(SvelteNode::Element(b)) = crate::model::tree_ops::find_mut(&mut root, 400) {
            b.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![
                        Action::ToggleVar("nope".into()),
                        Action::ToggleVar("open".into()),
                    ],
                }],
            });
        }
        let issues = validate(&root, 400);
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains("nope"))
        );
        assert!(
            !issues
                .iter()
                .any(|i| i.message.contains("\"open\" is not in scope"))
        );
    }

    #[test]
    fn validation_warns_on_show_target_without_binding() {
        let mut root = sample();
        if let Some(SvelteNode::Element(btn)) = crate::model::tree_ops::find_mut(&mut root, 300) {
            btn.binding = Some(Binding {
                var: "open".into(),
                transition: None,
            });
        }
        if let Some(SvelteNode::Element(b)) = crate::model::tree_ops::find_mut(&mut root, 400) {
            b.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![
                        Action::Show {
                            target: "Btn".into(),
                            transition: None,
                        },
                        Action::Show {
                            target: "Card".into(),
                            transition: None,
                        },
                        Action::Show {
                            target: "Cudzy".into(),
                            transition: None,
                        },
                    ],
                }],
            });
        }
        let issues = validate(&root, 400);
        // "Btn" resolves to the first match (300) — it HAS a binding, so no
        // warning. "Card" resolves but has no binding → warn. "Cudzy" is
        // outside the scope → error.
        assert!(!issues.iter().any(|i| i.message.contains("\"Btn\"")));
        assert!(issues.iter().any(|i| i.severity == Severity::Warn
            && i.message.contains("Card")
            && i.message.contains("has no Binding")));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains("Cudzy"))
        );
    }

    #[test]
    fn var_names_are_checked_like_js_identifiers() {
        assert!(is_valid_var_name("open"));
        assert!(is_valid_var_name("_x1"));
        assert!(!is_valid_var_name("1x"));
        assert!(!is_valid_var_name("klucz obcy"));
        assert!(!is_valid_var_name("class"));
    }

    #[test]
    fn transition_formats_svelte_directives() {
        let t = Transition::Slide { ms: 200 };
        assert_eq!(t.to_svelte(), "slide={{ duration: 200 }}");
        assert_eq!(t.with_ms(50), Transition::Slide { ms: 50 });
        assert_eq!(
            Transition::from_kind("fade", 150),
            Some(Transition::Fade { ms: 150 })
        );
    }
}
