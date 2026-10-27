use std::cell::Cell;
use std::rc::Rc;

use sequoia_core::{Color, Fill, ImageSource, Length, Point, Rect, Vec2};
use sequoia_elements::{
    Button, Checkbox, Container, DateField, DragValue, DropdownButton, Field, MultiSelect,
    NumberField, ProgressBar, Slider1D, Switch, TextArea, TextField,
};
use sequoia_input::{CursorIcon, PointerButton, PointerEvent};
use sequoia_reactive::{Signal, effect};
use sequoia_style::{BoxStyle, ButtonStyle, ShadowSpec, TextStyle, Theme};
use sequoia_view::{
    Align, AnimSpec, AnimatedSize, AnyView, AsyncView, Axis, Canvas, Column, EventCtx, Grid,
    GridFlow, GridPlacement, Icon, Image, Indents, Justify, Live, LiveOffset, LiveOpacity,
    MAX_ANIM_DT, Margined, Masonry, Motion, Overflow, Painter, Panel, Pulse, Rotated, Row, Scaled,
    ScrollAxis, ScrollView, Shake, Spacer, SplitPanel, Stack, StackPos, Text, TextVAlign, Track,
    View, VirtualList, Wrap, when,
};

use crate::bindings::{ActionSig, FieldSignal, HostBindings, PropType, SeqRowData};
use crate::error::SeqError;
use crate::ir::{
    ComponentDef, DeferredKind, LengthVal, SeqApp, SeqArg, SeqNode, SeqValue, SeqWindow, StyleDecl,
    ThemeName,
};
use crate::value::Value;

pub fn resolve_theme(file: &str, app: &SeqApp) -> Result<Theme, SeqError> {
    match &app.theme {
        ThemeName::Dark => Ok(Theme::dark()),
        ThemeName::Light => Ok(Theme::light()),
        ThemeName::File(path) => {
            let full = theme_file_path(file, path);
            Theme::from_tokens_file(full.to_str().unwrap_or(path))
                .map_err(|e| SeqError::new(file, app.app_line, 1, format!("theme: {e}")))
        }
    }
}

fn theme_file_path(seq_file: &str, path: &str) -> std::path::PathBuf {
    let requested = std::path::Path::new(path);
    if requested.is_absolute() {
        return requested.to_path_buf();
    }
    match std::path::Path::new(seq_file)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        Some(dir) => dir.join(requested),
        None => requested.to_path_buf(),
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Resolver<'a> {
    Static,

    Host(&'a HostBindings),
}

pub fn lower(file: &str, app: &SeqApp) -> Result<AnyView, SeqError> {
    if let Some(d) = app.deferred.first() {
        let what = match d.kind {
            DeferredKind::Prop => "prop",
            DeferredKind::On => "callback on",
            DeferredKind::Style => "blok style",
        };
        return Err(SeqError::new(
            file,
            d.line,
            1,
            format!("{what} '{}' wymaga hosta Rust — inkrement 2", d.name),
        ));
    }
    let resolved;
    let app = if app.fns.is_empty() {
        app
    } else {
        resolved = crate::resolve_seq_calls(app.clone());
        &resolved
    };
    let app = expand_components(file, app)?;
    let theme = resolve_theme(file, &app)?;
    let window = app
        .windows
        .first()
        .ok_or_else(|| SeqError::new(file, 1, 1, "app wymaga min. 1 bloku `window`".to_string()))?;
    let styles = build_styles(file, &theme, &app.styles)?;
    lower_window(file, &theme, &styles, window)
}

pub fn lower_with(file: &str, app: &SeqApp, bindings: &HostBindings) -> Result<AnyView, SeqError> {
    check_decls(file, app, bindings)?;
    let resolved;
    let app = if app.fns.is_empty() {
        app
    } else {
        resolved = crate::resolve_seq_calls(app.clone());
        &resolved
    };
    let owned;
    let bindings = if app.fns.is_empty() {
        bindings
    } else {
        match bindings.get_seq() {
            Some(rt) if rt.file() == file => {
                rt.rebind(&app.fns);
                bindings
            }
            _ => {
                owned = bindings
                    .clone()
                    .with_seq(Rc::new(crate::eval::SeqRuntime::new(
                        file,
                        &app.fns,
                        &app.states,
                    )));
                &owned
            }
        }
    };
    let app = expand_components(file, app)?;
    let theme = resolve_theme(file, &app)?;
    let window = app
        .windows
        .first()
        .ok_or_else(|| SeqError::new(file, 1, 1, "app wymaga min. 1 bloku `window`".to_string()))?;
    let styles = build_styles(file, &theme, &app.styles)?;
    lower_window_with(file, &theme, &styles, window, bindings)
}

const MAX_COMP_DEPTH: usize = 32;

const MAX_EXPANDED_NODES: usize = 100_000;

fn take_budget(file: &str, node: &SeqNode, budget: &mut usize) -> Result<(), SeqError> {
    *budget = budget.saturating_sub(1);
    if *budget == 0 {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!("component expansion exceeded {MAX_EXPANDED_NODES} node budget"),
        ));
    }
    Ok(())
}

pub fn expand_components(file: &str, app: &SeqApp) -> Result<SeqApp, SeqError> {
    if app.components.is_empty() {
        return Ok(app.clone());
    }
    let mut table: std::collections::HashMap<&str, &ComponentDef> =
        std::collections::HashMap::new();
    for c in &app.components {
        table.insert(c.name.as_str(), c);
    }
    let mut out = app.clone();
    let mut budget = MAX_EXPANDED_NODES;
    for w in &mut out.windows {
        w.nodes = expand_nodes(file, &table, &w.nodes, 0, &mut budget)?;
    }
    Ok(out)
}

fn expand_nodes(
    file: &str,
    table: &std::collections::HashMap<&str, &ComponentDef>,
    nodes: &[SeqNode],
    depth: usize,
    budget: &mut usize,
) -> Result<Vec<SeqNode>, SeqError> {
    let mut out = Vec::new();
    for n in nodes {
        if let Some(def) = table.get(n.kind.as_str()) {
            out.extend(expand_call(file, table, def, n, depth, budget)?);
        } else {
            take_budget(file, n, budget)?;
            let mut n = n.clone();
            n.children = expand_nodes(file, table, &n.children, depth, budget)?;
            out.push(n);
        }
    }
    Ok(out)
}

fn expand_call(
    file: &str,
    table: &std::collections::HashMap<&str, &ComponentDef>,
    def: &ComponentDef,
    node: &SeqNode,
    depth: usize,
    budget: &mut usize,
) -> Result<Vec<SeqNode>, SeqError> {
    if depth > MAX_COMP_DEPTH {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!("component nesting limit exceeded ({MAX_COMP_DEPTH})"),
        ));
    }

    let mut pmap: std::collections::HashMap<&str, &SeqValue> = std::collections::HashMap::new();
    let mut flex: Option<SeqValue> = None;
    for arg in &node.args {
        if arg.name == "flex" {
            flex = Some(arg.value.clone());
            continue;
        }
        let Some(param) = def.params.iter().find(|p| p.name == arg.name) else {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("nieznany parametr '{}' w {}", arg.name, def.name),
            ));
        };
        pmap.insert(param.name.as_str(), &arg.value);
    }
    for p in &def.params {
        if !pmap.contains_key(p.name.as_str()) {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                format!("brakuje parametru '{}' w {}", p.name, def.name),
            ));
        }
    }

    let kids = expand_nodes(file, table, &node.children, depth + 1, budget)?;

    let body: Vec<SeqNode> = def.body.iter().map(|b| substitute_node(b, &pmap)).collect();
    let mut body = splice_children(&body, &kids);

    if let Some(flex_val) = flex {
        if body.len() == 1 {
            let root = body.pop().expect("one root");
            body.push(with_flex(root, node.line, flex_val));
        } else {
            take_budget(file, node, budget)?;
            body = vec![wrap_column(node.line, body, Some(flex_val))];
        }
    } else if body.len() != 1 {
        take_budget(file, node, budget)?;
        body = vec![wrap_column(node.line, body, None)];
    }

    expand_nodes(&def.file, table, &body, depth + 1, budget)
}

fn with_flex(mut node: SeqNode, line: usize, flex: SeqValue) -> SeqNode {
    node.args.retain(|a| a.name != "flex");
    node.args.push(SeqArg {
        name: "flex".to_string(),
        line,
        value: flex,
    });
    node
}

fn wrap_column(line: usize, children: Vec<SeqNode>, flex: Option<SeqValue>) -> SeqNode {
    let mut args = Vec::new();
    if let Some(value) = flex {
        args.push(SeqArg {
            name: "flex".to_string(),
            line,
            value,
        });
    }
    SeqNode {
        kind: "Column".to_string(),
        line,
        label: None,
        args,
        children,
    }
}

fn substitute_node(node: &SeqNode, pmap: &std::collections::HashMap<&str, &SeqValue>) -> SeqNode {
    let mut out = node.clone();
    out.args = out
        .args
        .into_iter()
        .map(|mut a| {
            a.value = substitute_value(&a.value, pmap);
            a
        })
        .collect();
    out.children = out
        .children
        .into_iter()
        .map(|c| substitute_node(&c, pmap))
        .collect();
    out
}

fn substitute_value(
    value: &SeqValue,
    pmap: &std::collections::HashMap<&str, &SeqValue>,
) -> SeqValue {
    match value {
        SeqValue::Ident(name) => pmap
            .get(name.as_str())
            .map(|v| (*v).clone())
            .unwrap_or_else(|| value.clone()),
        SeqValue::Tuple(items) => {
            SeqValue::Tuple(items.iter().map(|v| substitute_value(v, pmap)).collect())
        }
        SeqValue::Array(items) => {
            SeqValue::Array(items.iter().map(|v| substitute_value(v, pmap)).collect())
        }
        SeqValue::Callback { action, args } => SeqValue::Callback {
            action: action.clone(),
            args: args.iter().map(|v| substitute_value(v, pmap)).collect(),
        },
        SeqValue::CallSeq { name, args } => SeqValue::CallSeq {
            name: name.clone(),
            args: args.iter().map(|v| substitute_value(v, pmap)).collect(),
        },
        _ => value.clone(),
    }
}

fn splice_children(nodes: &[SeqNode], kids: &[SeqNode]) -> Vec<SeqNode> {
    let mut out = Vec::new();
    for n in nodes {
        if n.kind == "children" {
            out.extend(kids.iter().cloned());
        } else {
            let mut n = n.clone();
            n.children = splice_children(&n.children, kids);
            out.push(n);
        }
    }
    out
}

pub fn check_decls(file: &str, app: &SeqApp, bindings: &HostBindings) -> Result<(), SeqError> {
    for d in &app.deferred {
        match d.kind {
            DeferredKind::Style => {
                return Err(SeqError::new(
                    file,
                    d.line,
                    1,
                    format!("style block '{}' - style system later", d.name),
                ));
            }
            DeferredKind::Prop => {
                let want = d.sig.first().map(String::as_str).unwrap_or("");
                let want = PropType::from_seq(want).ok_or_else(|| {
                    SeqError::new(
                        file,
                        d.line,
                        1,
                        format!("nieznany typ prop '{want}' (String/Bool/Int/Float)"),
                    )
                })?;
                match bindings.prop_type(&d.name) {
                    Some(got) if got == want => {}
                    Some(got) => {
                        return Err(SeqError::new(
                            file,
                            d.line,
                            1,
                            format!(
                                "prop '{}' is {} in .seq and host gave {}",
                                d.name,
                                want.seq_name(),
                                got.seq_name()
                            ),
                        ));
                    }
                    None => {
                        return Err(SeqError::new(
                            file,
                            d.line,
                            1,
                            format!(
                                "prop '{}' ({}) without binding - HostBindings::prop_{}()",
                                d.name,
                                want.seq_name(),
                                want.seq_name().to_lowercase()
                            ),
                        ));
                    }
                }
            }
            DeferredKind::On => {
                if app.fns.iter().any(|f| f.name == d.name) {
                    continue;
                }
                let want = match d
                    .sig
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .as_slice()
                {
                    [] => ActionSig::A0,
                    ["Int"] => ActionSig::Int,
                    ["Float"] => ActionSig::Float,
                    ["String"] => ActionSig::Str,
                    ["Bool"] => ActionSig::Bool,
                    ["Float", "Float"] => ActionSig::Point,
                    ["Float", "Float", "String"] => ActionSig::PointNamed,
                    ["Float", "Float", "Float"] => ActionSig::Wheel,
                    [t] => {
                        return Err(SeqError::new(
                            file,
                            d.line,
                            1,
                            format!("nieznany typ parametru '{t}' (Int/Float/String/Bool)"),
                        ));
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            d.line,
                            1,
                            format!(
                                "callback '{}' — wspierane sygnatury: (), (Int/Float/String/Bool), (Float, Float), (Float, Float, String), (Float, Float, Float)",
                                d.name
                            ),
                        ));
                    }
                };
                match bindings.action_sig(&d.name) {
                    Some(got)
                        if got == want || (want == ActionSig::Wheel && got == ActionSig::Pinch) => {
                    }
                    Some(_) => {
                        return Err(SeqError::new(
                            file,
                            d.line,
                            1,
                            format!("the signature on {} does not match the host", d.name),
                        ));
                    }
                    None => {
                        return Err(SeqError::new(
                            file,
                            d.line,
                            1,
                            format!(
                                "callback '{}' without binding - HostBindings::on_..()",
                                d.name
                            ),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextWeight {
    Bold,

    Semi,

    Medium,

    Normal,

    Light,

    Black,
}

impl TextWeight {
    pub const fn value(self) -> u16 {
        match self {
            Self::Bold => 700,
            Self::Semi => 600,
            Self::Medium => 500,
            Self::Normal => 400,
            Self::Light => 300,
            Self::Black => 900,
        }
    }
}

pub type SeqTextStyle = TextStyle;

#[derive(Clone, Debug, Default)]
pub struct BuiltStyles {
    boxes: std::collections::HashMap<String, BoxStyle>,
    buttons: std::collections::HashMap<String, ButtonStyle>,
    texts: std::collections::HashMap<String, SeqTextStyle>,
}

impl BuiltStyles {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn box_style(&self, name: &str) -> Option<BoxStyle> {
        self.boxes.get(name).copied()
    }

    pub fn button_style(&self, name: &str) -> Option<ButtonStyle> {
        self.buttons.get(name).copied()
    }

    pub fn text_style(&self, name: &str) -> Option<SeqTextStyle> {
        self.texts.get(name).cloned()
    }
}

pub(crate) fn box_style_of(
    file: &str,
    styles: &BuiltStyles,
    node: &SeqNode,
) -> Result<Option<BoxStyle>, SeqError> {
    let Some(arg) = node.args.iter().find(|a| a.name == "style") else {
        return Ok(None);
    };
    let name = match &arg.value {
        SeqValue::Ident(name) | SeqValue::Str(name) => name.clone(),
        _ => {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                "style: to nazwa bloku (style: Card)",
            ));
        }
    };
    styles.box_style(&name).map(Some).ok_or_else(|| {
        SeqError::new(
            file,
            arg.line,
            1,
            format!("unknown style '{name}' (check `style` blocks)"),
        )
    })
}

fn text_style_of(
    file: &str,
    styles: &BuiltStyles,
    node: &SeqNode,
) -> Result<Option<SeqTextStyle>, SeqError> {
    let Some(arg) = node.args.iter().find(|a| a.name == "style") else {
        return Ok(None);
    };
    let name = match &arg.value {
        SeqValue::Ident(name) | SeqValue::Str(name) => name.clone(),
        _ => {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                "style: to nazwa bloku (style: Card)",
            ));
        }
    };
    styles.text_style(&name).map(Some).ok_or_else(|| {
        SeqError::new(
            file,
            arg.line,
            1,
            format!("unknown style '{name}' (check `style` blocks)"),
        )
    })
}

fn apply_seq_text_style(v: Text, s: SeqTextStyle) -> Text {
    v.style(s)
}

pub fn build_styles(
    file: &str,
    theme: &Theme,
    styles: &[StyleDecl],
) -> Result<BuiltStyles, SeqError> {
    let mut out = BuiltStyles::empty();
    let res = Resolver::Static;
    for s in styles {
        match s.target.as_str() {
            "Box" => {
                let mut v = BoxStyle::new();
                for base in base_chain(file, styles, s)? {
                    apply_box_args(file, theme, &res, base, &mut v)?;
                }
                apply_box_args(file, theme, &res, s, &mut v)?;
                out.boxes.insert(s.name.clone(), v);
            }
            "Button" => {
                let mut v = ButtonStyle::new();
                for base in base_chain(file, styles, s)? {
                    if let Some(preset) = button_preset(&base.base) {
                        v = preset;
                    } else {
                        apply_button_args(file, theme, &res, base, &mut v)?;
                    }
                }
                if let Some(preset) = button_preset(&s.base) {
                    v = preset;
                }
                apply_button_args(file, theme, &res, s, &mut v)?;
                out.buttons.insert(s.name.clone(), v);
            }
            "Text" => {
                let mut v = SeqTextStyle::default();
                for base in base_chain(file, styles, s)? {
                    apply_text_args(file, theme, &res, base, &mut v)?;
                }
                apply_text_args(file, theme, &res, s, &mut v)?;
                out.texts.insert(s.name.clone(), v);
            }

            other => {
                return Err(SeqError::new(
                    file,
                    s.line,
                    1,
                    format!(
                        "style '{}': nieznany typ '{other}' (Box/Button/Text)",
                        s.name
                    ),
                ));
            }
        }
    }
    Ok(out)
}

fn base_chain<'s>(
    file: &str,
    styles: &'s [StyleDecl],
    start: &StyleDecl,
) -> Result<Vec<&'s StyleDecl>, SeqError> {
    let mut chain = Vec::new();
    let mut next = start.base.clone();
    let mut seen = vec![start.name.clone()];
    while let Some(name) = next {
        if start.target == "Button" && button_preset(&Some(name.clone())).is_some() {
            break;
        }
        if seen.contains(&name) {
            return Err(SeqError::new(
                file,
                start.line,
                1,
                format!("style '{}': cykliczne extends", start.name),
            ));
        }
        seen.push(name.clone());
        let Some(decl) = styles.iter().find(|o| o.name == name) else {
            return Err(SeqError::new(
                file,
                start.line,
                1,
                format!("style '{}': nieznana baza '{name}'", start.name),
            ));
        };
        if decl.target != start.target {
            return Err(SeqError::new(
                file,
                start.line,
                1,
                format!(
                    "style '{}': baza '{name}' to {}, a styl to {}",
                    start.name, decl.target, start.target
                ),
            ));
        }
        chain.push(decl);
        next = decl.base.clone();
        if chain.len() > styles.len() {
            return Err(SeqError::new(
                file,
                start.line,
                1,
                format!("style '{}': cykliczne extends", start.name),
            ));
        }
    }
    chain.reverse();
    Ok(chain)
}

fn button_preset(base: &Option<String>) -> Option<ButtonStyle> {
    match base.as_deref() {
        Some("primary") => Some(ButtonStyle::PRIMARY),
        Some("secondary") => Some(ButtonStyle::SECONDARY),
        Some("danger") => Some(ButtonStyle::DANGER),
        Some("ghost") => Some(ButtonStyle::GHOST),
        Some("dark_filled") => Some(ButtonStyle::DARK_FILLED),
        Some("outline_rose") => Some(ButtonStyle::OUTLINE_ROSE),
        _ => None,
    }
}

fn border_side(
    file: &str,
    theme: &Theme,
    style: &str,
    arg: &SeqArg,
) -> Result<(f32, Color), SeqError> {
    match &arg.value {
        SeqValue::Array(items) if items.len() == 2 => {
            let w = match &items[0] {
                SeqValue::Num(n) => *n,
                _ => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("'{style}' in style: [width, color]"),
                    ));
                }
            };
            let c = match &items[1] {
                SeqValue::Color(hex) => hex_color(file, arg.line, hex)?,
                SeqValue::Theme(f) => theme_color(file, arg.line, theme, f)?,
                _ => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("'{style}' in style: [width, color]"),
                    ));
                }
            };
            Ok((w, c))
        }
        _ => Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("'{style}' in style: [width, color]"),
        )),
    }
}

fn shadow_spec(
    file: &str,
    theme: &Theme,
    style: &str,
    arg: &SeqArg,
) -> Result<ShadowSpec, SeqError> {
    let err = || {
        SeqError::new(
            file,
            arg.line,
            1,
            format!("'{style}' w style: [kolor, blur] albo [kolor, blur, ox, oy]"),
        )
    };
    let SeqValue::Array(items) = &arg.value else {
        return Err(err());
    };
    if items.len() != 2 && items.len() != 4 {
        return Err(err());
    }
    let color = match &items[0] {
        SeqValue::Color(hex) => hex_color(file, arg.line, hex)?,
        SeqValue::Theme(f) => theme_color(file, arg.line, theme, f)?,
        _ => return Err(err()),
    };
    let num_at = |i: usize| match &items[i] {
        SeqValue::Num(n) if n.is_finite() => Ok(*n),
        _ => Err(err()),
    };
    let blur = num_at(1)?;
    let mut spec = ShadowSpec::new(color, blur);
    if items.len() == 4 {
        spec = spec.offset(num_at(2)?, num_at(3)?);
    }
    Ok(spec)
}

fn apply_box_args(
    file: &str,
    theme: &Theme,
    res: &Resolver<'_>,
    decl: &StyleDecl,
    v: &mut BoxStyle,
) -> Result<(), SeqError> {
    for arg in &decl.args {
        match arg.name.as_str() {
            "background" => *v = v.background(color(file, theme, arg)?),
            "radius" => *v = v.radius(num(file, arg, res, None)?),
            "border" => {
                let (w, c) = border_pair(file, theme, arg)?;
                *v = v.border(w, c);
            }
            "border_left" => {
                let (w, c) = border_side(file, theme, "border_left", arg)?;
                *v = v.border_left(w, c);
            }
            "border_right" => {
                let (w, c) = border_side(file, theme, "border_right", arg)?;
                *v = v.border_right(w, c);
            }
            "border_top" => {
                let (w, c) = border_side(file, theme, "border_top", arg)?;
                *v = v.border_top(w, c);
            }
            "border_bottom" => {
                let (w, c) = border_side(file, theme, "border_bottom", arg)?;
                *v = v.border_bottom(w, c);
            }
            "border_x" => {
                let (w, c) = border_side(file, theme, "border_x", arg)?;
                *v = v.border_x(w, c);
            }
            "border_y" => {
                let (w, c) = border_side(file, theme, "border_y", arg)?;
                *v = v.border_y(w, c);
            }
            "shadow" => *v = v.shadow(shadow_spec(file, theme, "shadow", arg)?),
            "inset_shadow" => {
                *v = v.inset_shadow(shadow_spec(file, theme, "inset_shadow", arg)?);
            }
            _ => {
                return Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!("nieznane pole '{}' w style Box", arg.name),
                ));
            }
        }
    }
    Ok(())
}

fn apply_button_args(
    file: &str,
    theme: &Theme,
    res: &Resolver<'_>,
    decl: &StyleDecl,
    v: &mut ButtonStyle,
) -> Result<(), SeqError> {
    for arg in &decl.args {
        match arg.name.as_str() {
            "background" => *v = v.background(color(file, theme, arg)?),
            "hover" => *v = v.hover(color(file, theme, arg)?),
            "pressed" => *v = v.pressed(color(file, theme, arg)?),
            "disabled" => *v = v.disabled(color(file, theme, arg)?),
            "text" => *v = v.text(color(file, theme, arg)?),
            "text_disabled" => *v = v.text_disabled(color(file, theme, arg)?),
            "font_size" => *v = v.font_size(num(file, arg, res, None)?),
            "font_weight" => {
                let n = num(file, arg, res, None)?;
                if !(n.is_finite() && n.fract() == 0.0 && (0.0..=1000.0).contains(&n)) {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "'font_weight' in style is an integer 0..=1000",
                    ));
                }
                #[allow(clippy::cast_possible_truncation)]
                let w = n as u16;
                *v = v.font_weight(w);
            }
            "radius" => *v = v.radius(num(file, arg, res, None)?),
            "padding" => match &arg.value {
                SeqValue::Array(items) if items.len() == 2 => {
                    let (Some(x), Some(y)) = (array_num(&items[0]), array_num(&items[1])) else {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "'padding' w style: [x, y]",
                        ));
                    };
                    *v = v.padding_xy(x, y);
                }
                _ => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "'padding' w style: [x, y]",
                    ));
                }
            },
            "border" => {
                let (w, c) = border_pair(file, theme, arg)?;
                *v = v.border(w, c);
            }
            "border_width" => *v = v.border_width(num(file, arg, res, None)?),
            "border_color" => *v = v.border_color(color(file, theme, arg)?),
            _ => {
                return Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!("nieznane pole '{}' w style Button", arg.name),
                ));
            }
        }
    }
    Ok(())
}

fn array_num(value: &SeqValue) -> Option<f32> {
    match value {
        SeqValue::Num(n) if n.is_finite() => Some(*n),
        _ => None,
    }
}

fn apply_text_args(
    file: &str,
    theme: &Theme,
    res: &Resolver<'_>,
    decl: &StyleDecl,
    v: &mut SeqTextStyle,
) -> Result<(), SeqError> {
    for arg in &decl.args {
        match arg.name.as_str() {
            "size" => v.size = Some(num(file, arg, res, None)?),
            "color" => v.color = Some(color(file, theme, arg)?),
            "bold" => v.weight = Some(TextWeight::Bold.value()),
            "semi" => v.weight = Some(TextWeight::Semi.value()),
            "medium" => v.weight = Some(TextWeight::Medium.value()),
            "normal" => v.weight = Some(TextWeight::Normal.value()),
            "light" => v.weight = Some(TextWeight::Light.value()),
            "black" => v.weight = Some(TextWeight::Black.value()),
            "line_height" => v.line_height = Some(num(file, arg, res, None)?),
            _ => {
                return Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!("nieznane pole '{}' w style Text", arg.name),
                ));
            }
        }
    }
    Ok(())
}

pub fn lower_window(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    window: &SeqWindow,
) -> Result<AnyView, SeqError> {
    lower_window_res(file, theme, styles, window, &Resolver::Static)
}

pub fn lower_window_with(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    window: &SeqWindow,
    bindings: &HostBindings,
) -> Result<AnyView, SeqError> {
    lower_window_res(file, theme, styles, window, &Resolver::Host(bindings))
}

fn lower_window_res(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    window: &SeqWindow,
    res: &Resolver<'_>,
) -> Result<AnyView, SeqError> {
    let mut kids = Vec::new();
    for n in &window.nodes {
        kids.push(lower_node(file, theme, styles, n, res, None)?);
    }
    let mut col = Column::new().fill_size();
    for k in kids {
        col = col.child(k);
    }
    Ok(AnyView::new(col))
}

fn lower_node(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    for arg in &node.args {
        if let SeqValue::Callback { action, .. } = &arg.value {
            match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("callback '{action}' wymaga hosta Rust — inkrement 2"),
                    ));
                }
                Resolver::Host(_) => {}
            }
        }
        if let SeqValue::CallSeq { name, .. } = &arg.value {
            match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("funkcja '{name}' (.seq) wymaga hosta Rust (lower_with)"),
                    ));
                }
                Resolver::Host(_) => {}
            }
        }
        if let SeqValue::Dynamic(name) = &arg.value {
            match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("dynamic({name}) wymaga hosta Rust — inkrement 2"),
                    ));
                }

                Resolver::Host(_) => {}
            }
        }
        if let SeqValue::Env(name) = &arg.value {
            match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("env.{name} wymaga hosta Rust (HostBindings::env_..)"),
                    ));
                }
                Resolver::Host(_) => {}
            }
        }
        if let SeqValue::ItemField(field) = &arg.value {
            match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("item.{field} wymaga hosta Rust — wiersze List (inkrement 3)"),
                    ));
                }

                Resolver::Host(_) => {}
            }
        }
    }
    let built = match node.kind.as_str() {
        "Column" => {
            if has_click(node) {
                return maybe_indent(
                    lower_clickable_flex(file, theme, styles, node, res, row, Column::new())?,
                    file,
                    node,
                    res,
                    row,
                );
            }
            let v = apply_flex_box(file, theme, node, Column::new(), res, row)?;
            Ok(boxed_children(file, theme, styles, node, v, res, row)?)
        }
        "Row" => {
            if has_click(node) {
                return maybe_indent(
                    lower_clickable_flex(file, theme, styles, node, res, row, Row::new())?,
                    file,
                    node,
                    res,
                    row,
                );
            }
            let v = apply_flex_box(file, theme, node, Row::new(), res, row)?;
            Ok(boxed_children(file, theme, styles, node, v, res, row)?)
        }
        "Container" => {
            let mut v = Container::new();
            if let Some(s) = box_style_of(file, styles, node)? {
                v = v.box_style(s);
            }
            for arg in &node.args {
                match arg.name.as_str() {
                    "style" => {}
                    "width" => v = v.width(len(file, arg, res, row)?),
                    "height" => v = v.height(len(file, arg, res, row)?),
                    "padding" => v = apply_padding(v, file, arg, res, row)?,
                    "padding_x" => v = v.padding_x(num(file, arg, res, row)?),
                    "padding_y" => v = v.padding_y(num(file, arg, res, row)?),
                    "background" => v = v.background(color(file, theme, arg)?),
                    "flex" => v = v.flex(num(file, arg, res, row)?),
                    "radius" => v = v.radius(num(file, arg, res, row)?),
                    "border" => {
                        let (w, c) = border_pair(file, theme, arg)?;
                        v = v.border(w, c);
                    }
                    "border_left" => {
                        let (w, c) = border_side(file, theme, "border_left", arg)?;
                        v = v.border_left(w, c);
                    }
                    "border_right" => {
                        let (w, c) = border_side(file, theme, "border_right", arg)?;
                        v = v.border_right(w, c);
                    }
                    "border_top" => {
                        let (w, c) = border_side(file, theme, "border_top", arg)?;
                        v = v.border_top(w, c);
                    }
                    "border_bottom" => {
                        let (w, c) = border_side(file, theme, "border_bottom", arg)?;
                        v = v.border_bottom(w, c);
                    }
                    "border_x" => {
                        let (w, c) = border_side(file, theme, "border_x", arg)?;
                        v = v.border_x(w, c);
                    }
                    "border_y" => {
                        let (w, c) = border_side(file, theme, "border_y", arg)?;
                        v = v.border_y(w, c);
                    }
                    "min_width" => v = v.min_width(num(file, arg, res, row)?),
                    "max_width" => v = v.max_width(num(file, arg, res, row)?),
                    "min_height" => v = v.min_height(num(file, arg, res, row)?),
                    "max_height" => v = v.max_height(num(file, arg, res, row)?),
                    "margin" => v = apply_container_margin(v, file, arg, res, row)?,
                    "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
                    "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
                    "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
                    "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
                    "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
                    "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
                    "overflow" => v = v.overflow(overflow_val(file, arg)?),
                    "on_click" => match res {
                        Resolver::Static => {
                            return Err(SeqError::new(
                                file,
                                arg.line,
                                1,
                                "on_click wymaga hosta Rust",
                            ));
                        }
                        Resolver::Host(b) => match &arg.value {
                            SeqValue::Callback { .. } | SeqValue::CallSeq { .. } => {
                                let fire = wire_action(file, arg.line, b, &arg.value, row)?;
                                v = v.on_click(move || fire());
                            }
                            _ => {
                                return Err(SeqError::new(
                                    file,
                                    arg.line,
                                    1,
                                    "on_click: { akcja(args) }",
                                ));
                            }
                        },
                    },
                    "hover" => v = v.hover(color(file, theme, arg)?),
                    "hit" => v = v.hit_testable(flag(file, arg, res, row)?),
                    "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right"
                    | "indent_top" | "indent_bottom" => {}
                    _ => unreachable_checked(file, node, arg)?,
                }
            }
            if let Some(kid) = single_child(file, theme, styles, node, res, row)? {
                v = v.child(kid);
            }
            Ok(AnyView::new(v))
        }
        "Panel" => {
            if !node.children.is_empty() {
                return Err(SeqError::new(
                    file,
                    node.line,
                    1,
                    "Panel is a leaf (only background/frame, no children) - put content in Container or Column",
                ));
            }
            let mut v = Panel::new();
            if let Some(s) = box_style_of(file, styles, node)? {
                v = v.box_style(s);
            }
            for arg in &node.args {
                match arg.name.as_str() {
                    "style" => {}
                    "width" => v = v.width(len(file, arg, res, row)?),
                    "height" => v = v.height(len(file, arg, res, row)?),
                    "padding" => v = apply_padding(v, file, arg, res, row)?,
                    "padding_x" => v = v.padding_x(num(file, arg, res, row)?),
                    "padding_y" => v = v.padding_y(num(file, arg, res, row)?),
                    "background" => v = v.background(color(file, theme, arg)?),
                    "flex" => v = v.flex(num(file, arg, res, row)?),
                    "radius" => v = v.radius(num(file, arg, res, row)?),
                    "border" => {
                        let (w, c) = border_pair(file, theme, arg)?;
                        v = v.border(w, c);
                    }
                    "border_left" => {
                        let (w, c) = border_side(file, theme, "border_left", arg)?;
                        v = v.border_left(w, c);
                    }
                    "border_right" => {
                        let (w, c) = border_side(file, theme, "border_right", arg)?;
                        v = v.border_right(w, c);
                    }
                    "border_top" => {
                        let (w, c) = border_side(file, theme, "border_top", arg)?;
                        v = v.border_top(w, c);
                    }
                    "border_bottom" => {
                        let (w, c) = border_side(file, theme, "border_bottom", arg)?;
                        v = v.border_bottom(w, c);
                    }
                    "border_x" => {
                        let (w, c) = border_side(file, theme, "border_x", arg)?;
                        v = v.border_x(w, c);
                    }
                    "border_y" => {
                        let (w, c) = border_side(file, theme, "border_y", arg)?;
                        v = v.border_y(w, c);
                    }
                    "min_width" => v = v.min_width(num(file, arg, res, row)?),
                    "max_width" => v = v.max_width(num(file, arg, res, row)?),
                    "min_height" => v = v.min_height(num(file, arg, res, row)?),
                    "max_height" => v = v.max_height(num(file, arg, res, row)?),
                    "margin" => v = apply_panel_margin(v, file, arg, res, row)?,
                    "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
                    "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
                    "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
                    "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
                    "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
                    "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
                    "overflow" => v = v.overflow(overflow_val(file, arg)?),
                    "hit" => v = v.hit_testable(flag(file, arg, res, row)?),
                    "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right"
                    | "indent_top" | "indent_bottom" => {}
                    _ => unreachable_checked(file, node, arg)?,
                }
            }
            Ok(AnyView::new(v))
        }
        "ScrollView" => {
            let mut axis = ScrollAxis::Vertical;
            for arg in &node.args {
                match arg.name.as_str() {
                    "axis" => {
                        axis = match ident(file, arg)?.as_str() {
                            "vertical" => ScrollAxis::Vertical,
                            "horizontal" => ScrollAxis::Horizontal,
                            "both" => ScrollAxis::Both,
                            other => {
                                return Err(SeqError::new(
                                    file,
                                    arg.line,
                                    1,
                                    format!("axis to vertical/horizontal/both, nie '{other}'"),
                                ));
                            }
                        };
                    }
                    "width" | "height" | "padding" | "padding_x" | "padding_y" | "background" => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            format!(
                                "propka '{}' on ScrollView in increment 1 - wrap content in Column",
                                arg.name
                            ),
                        ));
                    }
                    "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right"
                    | "indent_top" | "indent_bottom" => {}
                    _ => unreachable_checked(file, node, arg)?,
                }
            }
            let inner = match node.children.len() {
                0 => AnyView::new(Column::new()),
                1 => lower_node(file, theme, styles, &node.children[0], res, row)?,
                _ => {
                    let mut col = Column::new();
                    for c in &node.children {
                        let kid = lower_node(file, theme, styles, c, res, row)?;
                        col = match child_flex_weight(file, res, row, c)? {
                            Some(w) => col.child_flex(kid, w),
                            None => col.child(kid),
                        };
                    }
                    AnyView::new(col)
                }
            };
            let mut v = ScrollView::new(inner);
            v = match axis {
                ScrollAxis::Vertical => v.vertical(),
                ScrollAxis::Horizontal => v.horizontal(),
                _ => v.axis(axis),
            };
            Ok(AnyView::new(v))
        }
        "Text" => lower_text(file, theme, styles, node, res, row),
        "Button" => lower_button(file, styles, node, res, row),
        "Spacer" => {
            let mut flex = 1.0;
            let mut hit: Option<bool> = None;
            let mut width: Option<Length> = None;
            let mut height: Option<Length> = None;
            let mut min_width: Option<f32> = None;
            let mut max_width: Option<f32> = None;
            let mut min_height: Option<f32> = None;
            let mut max_height: Option<f32> = None;
            for arg in &node.args {
                match arg.name.as_str() {
                    "flex" => flex = num(file, arg, res, row)?,
                    "width" => width = Some(len(file, arg, res, row)?),
                    "height" => height = Some(len(file, arg, res, row)?),
                    "min_width" => min_width = Some(num(file, arg, res, row)?),
                    "max_width" => max_width = Some(num(file, arg, res, row)?),
                    "min_height" => min_height = Some(num(file, arg, res, row)?),
                    "max_height" => max_height = Some(num(file, arg, res, row)?),
                    "hit" => hit = Some(flag(file, arg, res, row)?),
                    "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right"
                    | "indent_top" | "indent_bottom" => {}
                    _ => unreachable_checked(file, node, arg)?,
                }
            }
            let mut v = Spacer::flex(flex);
            if let Some(on) = hit {
                v = v.hit_testable(on);
            }
            if let Some(w) = width {
                v = v.width(w);
            }
            if let Some(h) = height {
                v = v.height(h);
            }
            if let Some(x) = min_width {
                v = v.min_width(x);
            }
            if let Some(x) = max_width {
                v = v.max_width(x);
            }
            if let Some(x) = min_height {
                v = v.min_height(x);
            }
            if let Some(x) = max_height {
                v = v.max_height(x);
            }
            Ok(AnyView::new(v))
        }
        "Icon" => {
            let name = opt_str(file, node, "name", res, row)?.ok_or_else(|| {
                SeqError::new(file, node.line, 1, "Icon wymaga name: \"...\"".to_string())
            })?;
            let mut v = Icon::new(name);
            for arg in &node.args {
                match arg.name.as_str() {
                    "name" => {}
                    "size" => v = v.size(num(file, arg, res, row)?),
                    "color" => v = v.color(color(file, theme, arg)?),
                    "width" => v = v.width(len(file, arg, res, row)?),
                    "height" => v = v.height(len(file, arg, res, row)?),
                    "min_width" => v = v.min_width(num(file, arg, res, row)?),
                    "max_width" => v = v.max_width(num(file, arg, res, row)?),
                    "min_height" => v = v.min_height(num(file, arg, res, row)?),
                    "max_height" => v = v.max_height(num(file, arg, res, row)?),
                    "margin" => v = apply_icon_margin(v, file, arg, res, row)?,
                    "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
                    "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
                    "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
                    "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
                    "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
                    "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
                    "flex" => {}
                    "hit" => v = v.hit_testable(flag(file, arg, res, row)?),
                    "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right"
                    | "indent_top" | "indent_bottom" => {}
                    _ => unreachable_checked(file, node, arg)?,
                }
            }
            Ok(AnyView::new(v))
        }
        "Image" => {
            let path = opt_str(file, node, "path", res, row)?
                .or_else(|| node.label.clone())
                .ok_or_else(|| {
                    SeqError::new(file, node.line, 1, "Image wymaga path: \"...\"".to_string())
                })?;
            let mut v = Image::file(path);
            for arg in &node.args {
                match arg.name.as_str() {
                    "path" => {}
                    "width" => v = v.width(len(file, arg, res, row)?),
                    "height" => v = v.height(len(file, arg, res, row)?),
                    "min_width" => v = v.min_width(num(file, arg, res, row)?),
                    "max_width" => v = v.max_width(num(file, arg, res, row)?),
                    "min_height" => v = v.min_height(num(file, arg, res, row)?),
                    "max_height" => v = v.max_height(num(file, arg, res, row)?),
                    "margin" => v = apply_image_margin(v, file, arg, res, row)?,
                    "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
                    "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
                    "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
                    "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
                    "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
                    "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),

                    "flex" => {}
                    "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right"
                    | "indent_top" | "indent_bottom" => {}
                    _ => unreachable_checked(file, node, arg)?,
                }
            }
            Ok(AnyView::new(v))
        }
        "List" => lower_list(file, theme, styles, node, res),
        "VirtualList" => lower_virtual_list(file, theme, styles, node, res, row),
        "SplitPanel" => lower_split(file, theme, styles, node, res, row),
        "Canvas" => lower_canvas(file, theme, node, res, row),
        "Checkbox" => lower_checkbox(file, theme, node, res, row),
        "Switch" => lower_switch(file, theme, node, res, row),
        "TextField" => lower_textfield(file, theme, node, res, row),
        "TextArea" => lower_textarea(file, theme, node, res, row),
        "Field" => lower_form_field(file, theme, node, res, row),
        "NumberField" => lower_number_field(file, theme, node, res, row),
        "DateField" => lower_date_field(file, theme, node, res, row),
        "MultiSelect" => lower_multi_select(file, theme, node, res, row),
        "Slider" => lower_slider(file, theme, node, res, row),
        "DragValue" => lower_drag_value(file, theme, node, res, row),
        "Dropdown" => lower_dropdown(file, theme, node, res, row),
        "Progress" => lower_progress(file, theme, node, res, row),
        "Stack" => lower_stack(file, theme, styles, node, res, row),
        "Grid" => lower_grid(file, theme, styles, node, res, row),
        "Wrap" => lower_wrap(file, theme, styles, node, res, row),
        "Masonry" => lower_masonry(file, theme, styles, node, res, row),
        "If" => lower_if(file, theme, styles, node, res, row),
        "Width" => lower_width(file, theme, styles, node, res, row),
        "Height" => lower_height(file, theme, styles, node, res, row),
        "Opacity" => lower_opacity(file, theme, styles, node, res, row),
        "Scale" => lower_scale(file, theme, styles, node, res, row),
        "Rotate" => lower_rotate(file, theme, styles, node, res, row),
        "Translate" => lower_translate(file, theme, styles, node, res, row),
        "Pulse" => lower_pulse(file, theme, styles, node, res, row),
        "Shake" => lower_shake(file, theme, styles, node, res, row),
        "Async" => lower_async(file, theme, styles, node, res, row),
        "Effect" => lower_effect(file, node, res, row),
        other => Err(SeqError::new(
            file,
            node.line,
            1,
            format!("nieznany komponent '{other}'"),
        )),
    };
    maybe_indent(built?, file, node, res, row)
}

fn apply_flex_box<V: FlexBoxOps>(
    file: &str,
    theme: &Theme,
    node: &SeqNode,
    mut v: V,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<V, SeqError> {
    for arg in &node.args {
        v = match arg.name.as_str() {
            "width" => v.set_width(len(file, arg, res, row)?),
            "height" => v.set_height(len(file, arg, res, row)?),
            "padding" => v.set_padding(padding_val(file, arg, res, row)?),
            "padding_x" => v.set_padding_x(num(file, arg, res, row)?),
            "padding_y" => v.set_padding_y(num(file, arg, res, row)?),
            "background" => v.set_background(color(file, theme, arg)?),
            "flex" => v.set_flex(num(file, arg, res, row)?),
            "gap" => v.set_gap(num(file, arg, res, row)?),
            "main_gap" => v.set_gap(num(file, arg, res, row)?),
            "cross_gap" => v.set_cross_gap(num(file, arg, res, row)?),
            "wrap" => v.set_wrap(flag(file, arg, res, row)?),
            "align_content" => v.set_align(align_val(file, arg)?),
            "align" => v.set_align(align_val(file, arg)?),
            "justify" => v.set_justify(justify_val(file, arg)?),
            "min_width" => v.set_min_width(num(file, arg, res, row)?),
            "max_width" => v.set_max_width(num(file, arg, res, row)?),
            "min_height" => v.set_min_height(num(file, arg, res, row)?),
            "max_height" => v.set_max_height(num(file, arg, res, row)?),
            "margin" => v.set_margin(margin_val(file, arg, res, row)?),
            "margin_x" => v.set_margin_x(num(file, arg, res, row)?),
            "margin_y" => v.set_margin_y(num(file, arg, res, row)?),
            "margin_left" => v.set_margin_left(num(file, arg, res, row)?),
            "margin_right" => v.set_margin_right(num(file, arg, res, row)?),
            "margin_top" => v.set_margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v.set_margin_bottom(num(file, arg, res, row)?),
            "overflow" => v.set_overflow(overflow_val(file, arg)?),
            "hit" => v.set_hit_testable(flag(file, arg, res, row)?),
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => v,
            _ => return unreachable_checked(file, node, arg),
        };
    }
    Ok(v)
}

fn lower_text(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    if let Some(t_arg) = find_arg(node, "t") {
        match res {
            Resolver::Static => {
                return Err(SeqError::new(
                    file,
                    t_arg.line,
                    1,
                    "t: wymaga hosta Rust (HostBindings::i18n)",
                ));
            }
            Resolver::Host(b) => match &t_arg.value {
                SeqValue::Str(key) => {
                    let i18n = b.get_i18n().ok_or_else(|| {
                        SeqError::new(file, t_arg.line, 1, "t: wymaga HostBindings::i18n()")
                    })?;
                    let key = key.clone();
                    let mut v = Text::with(move || i18n.t(&key));
                    v = apply_text_box(file, styles, &mut v, node, res, row, theme)?;
                    return Ok(AnyView::new(v));
                }
                _ => {
                    return Err(SeqError::new(
                        file,
                        t_arg.line,
                        1,
                        "t: is the key in quotation marks (t: \"save\")",
                    ));
                }
            },
        }
    }

    for arg in &node.args {
        if arg.name == "dynamic" {
            return match res {
                Resolver::Static => Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    "dynamic: wymaga hosta Rust — inkrement 2",
                )),
                Resolver::Host(b) => match &arg.value {
                    SeqValue::Dynamic(name) | SeqValue::Ident(name) => {
                        let read = display_of(file, arg.line, b, name)?;
                        let mut v = Text::with(move || read());
                        v = apply_text_box(file, styles, &mut v, node, res, row, theme)?;
                        Ok(AnyView::new(v))
                    }
                    SeqValue::Env(name) => {
                        let read = display_env_of(file, arg.line, b, name)?;
                        let mut v = Text::with(move || read());
                        v = apply_text_box(file, styles, &mut v, node, res, row, theme)?;
                        Ok(AnyView::new(v))
                    }
                    SeqValue::ItemField(field) => {
                        let read = display_field(file, arg.line, row, field)?;
                        let mut v = Text::with(move || read());
                        v = apply_text_box(file, styles, &mut v, node, res, row, theme)?;
                        Ok(AnyView::new(v))
                    }
                    _ => Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "dynamic: to nazwa propertu (dynamic: tytul)",
                    )),
                },
            };
        }
        if arg.name == "text"
            && let SeqValue::Dynamic(name) | SeqValue::Ident(name) = &arg.value
        {
            return match res {
                Resolver::Static => Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    "dynamic() wymaga hosta Rust — inkrement 2",
                )),
                Resolver::Host(b) => {
                    let read = display_of(file, arg.line, b, name)?;
                    let mut v = Text::with(move || read());
                    v = apply_text_box(file, styles, &mut v, node, res, row, theme)?;
                    Ok(AnyView::new(v))
                }
            };
        }
        if arg.name == "text"
            && let SeqValue::Env(name) = &arg.value
        {
            return match res {
                Resolver::Static => Err(SeqError::new(file, arg.line, 1, "env. wymaga hosta Rust")),
                Resolver::Host(b) => {
                    let read = display_env_of(file, arg.line, b, name)?;
                    let mut v = Text::with(move || read());
                    v = apply_text_box(file, styles, &mut v, node, res, row, theme)?;
                    Ok(AnyView::new(v))
                }
            };
        }
    }
    let mut v = Text::new(
        opt_str(file, node, "text", res, row)?
            .unwrap_or_else(|| node.label.clone().unwrap_or_default()),
    );
    if let Some(s) = text_style_of(file, styles, node)? {
        v = apply_seq_text_style(v, s);
    }
    for arg in &node.args {
        match arg.name.as_str() {
            "text" | "dynamic" | "t" => {}
            "style" => {}
            "size" => v = v.size(num(file, arg, res, row)?),
            "color" => v = v.color(color(file, theme, arg)?),
            "bold" => {
                if flag(file, arg, res, row)? {
                    v = v.bold();
                }
            }
            "semi" => {
                if flag(file, arg, res, row)? {
                    v = v.semibold();
                }
            }
            "medium" => {
                if flag(file, arg, res, row)? {
                    v = v.medium();
                }
            }
            "normal" => {
                if flag(file, arg, res, row)? {
                    v = v.normal();
                }
            }
            "light" => {
                if flag(file, arg, res, row)? {
                    v = v.light();
                }
            }
            "black" => {
                if flag(file, arg, res, row)? {
                    v = v.black();
                }
            }
            "truncate" => v = v.truncate(num(file, arg, res, row)?),

            "wrap" => {
                if flag(file, arg, res, row)? {
                    v = v.wrap();
                }
            }
            "line_height" => v = v.line_height(num(file, arg, res, row)?),
            "line_height_px" => v = v.line_height_px(num(file, arg, res, row)?),
            "align_y" => v = v.align_y(text_valign(file, arg)?),
            "max_lines" => {
                if let Some(n) = count_val(file, node, res, row, "max_lines")? {
                    v = v.max_lines(n.max(1));
                }
            }
            "font_scale" => v = v.font_scale(num(file, arg, res, row)?),

            "flex" => v = v.flex(num(file, arg, res, row)?),
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "margin" => v = apply_text_margin(v, file, arg, res, row)?,
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "hit" => v = v.hit_testable(flag(file, arg, res, row)?),
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

fn apply_text_box(
    file: &str,
    styles: &BuiltStyles,
    v: &mut Text,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    theme: &Theme,
) -> Result<Text, SeqError> {
    let mut out = std::mem::replace(v, Text::new(""));
    if let Some(s) = text_style_of(file, styles, node)? {
        out = apply_seq_text_style(out, s);
    }
    for arg in &node.args {
        match arg.name.as_str() {
            "text" | "dynamic" | "t" => {}
            "style" => {}
            "size" => out = out.size(num(file, arg, res, row)?),
            "color" => out = out.color(color(file, theme, arg)?),
            "bold" => {
                if flag(file, arg, res, row)? {
                    out = out.bold();
                }
            }
            "semi" => {
                if flag(file, arg, res, row)? {
                    out = out.semibold();
                }
            }
            "medium" => {
                if flag(file, arg, res, row)? {
                    out = out.medium();
                }
            }
            "normal" => {
                if flag(file, arg, res, row)? {
                    out = out.normal();
                }
            }
            "light" => {
                if flag(file, arg, res, row)? {
                    out = out.light();
                }
            }
            "black" => {
                if flag(file, arg, res, row)? {
                    out = out.black();
                }
            }
            "truncate" => out = out.truncate(num(file, arg, res, row)?),

            "wrap" => {
                if flag(file, arg, res, row)? {
                    out = out.wrap();
                }
            }
            "line_height" => out = out.line_height(num(file, arg, res, row)?),
            "line_height_px" => out = out.line_height_px(num(file, arg, res, row)?),
            "align_y" => out = out.align_y(text_valign(file, arg)?),
            "max_lines" => {
                if let Some(n) = count_val(file, node, res, row, "max_lines")? {
                    out = out.max_lines(n.max(1));
                }
            }
            "font_scale" => out = out.font_scale(num(file, arg, res, row)?),

            "flex" => out = out.flex(num(file, arg, res, row)?),
            "width" => out = out.width(len(file, arg, res, row)?),
            "height" => out = out.height(len(file, arg, res, row)?),
            "min_width" => out = out.min_width(num(file, arg, res, row)?),
            "max_width" => out = out.max_width(num(file, arg, res, row)?),
            "min_height" => out = out.min_height(num(file, arg, res, row)?),
            "max_height" => out = out.max_height(num(file, arg, res, row)?),
            "margin" => out = apply_text_margin(out, file, arg, res, row)?,
            "margin_x" => out = out.margin_x(num(file, arg, res, row)?),
            "margin_y" => out = out.margin_y(num(file, arg, res, row)?),
            "margin_left" => out = out.margin_left(num(file, arg, res, row)?),
            "margin_right" => out = out.margin_right(num(file, arg, res, row)?),
            "margin_top" => out = out.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => out = out.margin_bottom(num(file, arg, res, row)?),
            "hit" => out = out.hit_testable(flag(file, arg, res, row)?),
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(out)
}

fn apply_button_margin(
    v: Button,
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Button, SeqError> {
    match margin_val(file, arg, res, row)? {
        MarginVal::All(n) => Ok(v.margin(n)),
        MarginVal::Xy(x, y) => Ok(v.margin_xy(x, y)),
    }
}

fn button_px(
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Option<f32>, SeqError> {
    match &arg.value {
        SeqValue::Length(LengthVal::Px(n)) => Ok(Some(*n)),
        SeqValue::Length(LengthVal::Fit) => Ok(None),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("'{}' to px()/fit", arg.name),
            )),
            Resolver::Host(b) => Ok(Some(prop_num_env(file, arg.line, b, prop)?)),
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("'{}' to px()/fit", arg.name),
            )),
            Resolver::Host(b) => Ok(Some(env_num(file, arg.line, b, name)?)),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("'{}' to px()/fit", arg.name),
            )),
            Resolver::Host(_) => Ok(Some(row_num(file, arg.line, row, field)?)),
        },
        _ => Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("'{}' is px()/fit (stretch by flex: in parent)", arg.name),
        )),
    }
}

fn apply_icon_margin(
    v: Icon,
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Icon, SeqError> {
    match margin_val(file, arg, res, row)? {
        MarginVal::All(n) => Ok(v.margin(n)),
        MarginVal::Xy(x, y) => Ok(v.margin_xy(x, y)),
    }
}

fn apply_image_margin(
    v: Image,
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Image, SeqError> {
    match margin_val(file, arg, res, row)? {
        MarginVal::All(n) => Ok(v.margin(n)),
        MarginVal::Xy(x, y) => Ok(v.margin_xy(x, y)),
    }
}

fn apply_text_margin(
    v: Text,
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Text, SeqError> {
    match margin_val(file, arg, res, row)? {
        MarginVal::All(n) => Ok(v.margin(n)),
        MarginVal::Xy(x, y) => Ok(v.margin_xy(x, y)),
    }
}

fn lower_clip(
    file: &str,
    theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<DrawLayer, SeqError> {
    let r = rect_src_of(file, res, row, node)?;
    let radius = opt_num(file, res, row, node, "radius")?;
    let has_radius = find_arg(node, "radius").is_some();
    let visible = match find_arg(node, "visible") {
        None => BoolSrc::Lit(true),
        Some(arg) => dyn_bool(file, arg.line, res, row, &arg.value, "visible")?,
    };
    let mut layers = Vec::new();
    let touch_list = vec![r.touch_rc(), radius.touch_rc(), visible.touch_rc()];
    for c in &node.children {
        layers.push(lower_layer(file, theme, c, res, row)?);
    }
    Ok(DrawLayer::Clip {
        rect: r,
        radius,
        has_radius,
        visible,
        touch_extra: touches(&touch_list),
        layers,
    })
}

fn lower_button(
    file: &str,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let mut v = Button::new(node.label.clone().unwrap_or_default());
    if let Some(s) = button_style_of(file, styles, node)? {
        v = v.style(s);
    }
    for arg in &node.args {
        match arg.name.as_str() {
            "style" => {}
            "enabled" => v = v.disabled(!flag(file, arg, res, row)?),
            "font_size" => v = v.font_size(num(file, arg, res, row)?),
            "width" => {
                if let Some(px) = button_px(file, arg, res, row)? {
                    v = v.width(px);
                }
            }
            "height" => {
                if let Some(px) = button_px(file, arg, res, row)? {
                    v = v.height(px);
                }
            }
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "margin" => v = apply_button_margin(v, file, arg, res, row)?,
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "flex" => {}
            "on_click" => match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "on_click wymaga hosta Rust — inkrement 2",
                    ));
                }
                Resolver::Host(b) => match &arg.value {
                    SeqValue::Callback { .. } | SeqValue::CallSeq { .. } => {
                        let fire = wire_action(file, arg.line, b, &arg.value, row)?;
                        v = v.on_click(move || fire());
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "on_click: { akcja(args) }",
                        ));
                    }
                },
            },
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

fn without_placement(node: &SeqNode) -> SeqNode {
    SeqNode {
        kind: node.kind.clone(),
        line: node.line,
        label: node.label.clone(),
        args: node
            .args
            .iter()
            .filter(|a| {
                !matches!(
                    a.name.as_str(),
                    "at" | "offset" | "z" | "col" | "row" | "col_span" | "row_span"
                )
            })
            .cloned()
            .collect(),
        children: node.children.clone(),
    }
}

fn stack_pos_of(file: &str, node: &SeqNode) -> Result<StackPos, SeqError> {
    match find_arg(node, "at") {
        None => Ok(StackPos::default()),
        Some(arg) => match &arg.value {
            SeqValue::Ident(name) => match name.as_str() {
                "top" => Ok(StackPos::top()),
                "bottom" => Ok(StackPos::bottom()),
                "left" => Ok(StackPos::left()),
                "right" => Ok(StackPos::right()),
                "top_left" => Ok(StackPos::top_left()),
                "top_right" => Ok(StackPos::top_right()),
                "bottom_left" => Ok(StackPos::bottom_left()),
                "bottom_right" => Ok(StackPos::bottom_right()),
                "fill" => Ok(StackPos::fill()),
                "center" => Ok(StackPos::default()),
                other => Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!(
                        "at to top/bottom/left/right/top_left/top_right/bottom_left/bottom_right/fill/center, nie '{other}'"
                    ),
                )),
            },
            _ => Err(SeqError::new(
                file,
                arg.line,
                1,
                "at: to nazwa (at: top_left)",
            )),
        },
    }
}

fn stack_offset_of(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
) -> Result<(f32, f32), SeqError> {
    match find_arg(node, "offset") {
        None => Ok((0.0, 0.0)),
        Some(arg) => match &arg.value {
            SeqValue::Tuple(items) if items.len() == 2 => {
                let x = dyn_f32(file, arg.line, res, row, &items[0], "offset.dx")?;
                let y = dyn_f32(file, arg.line, res, row, &items[1], "offset.dy")?;
                Ok((x.val(), y.val()))
            }
            _ => Err(SeqError::new(
                file,
                arg.line,
                1,
                "offset: to krotka (dx, dy)",
            )),
        },
    }
}

fn stack_z_of(
    file: &str,
    res: &Resolver<'_>,
    _row: Option<&SeqRowData>,
    node: &SeqNode,
) -> Result<i32, SeqError> {
    match find_arg(node, "z") {
        None => Ok(0),
        Some(arg) => match &arg.value {
            SeqValue::Num(n) => {
                if n.is_finite() {
                    #[allow(clippy::cast_possible_truncation)]
                    Ok(*n as i32)
                } else {
                    Err(SeqError::new(file, arg.line, 1, "'z' must be finished"))
                }
            }
            SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
                Resolver::Static => Err(SeqError::new(file, arg.line, 1, "'z' to liczba")),
                Resolver::Host(b) => {
                    let v = prop_num_env(file, arg.line, b, prop)?;
                    #[allow(clippy::cast_possible_truncation)]
                    Ok(v as i32)
                }
            },
            _ => Err(SeqError::new(file, arg.line, 1, "'z' is an integer")),
        },
    }
}

fn lower_stack(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let mut v = Stack::new();
    for arg in &node.args {
        match arg.name.as_str() {
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "padding" => match padding_val(file, arg, res, row)? {
                PaddingVal::All(n) => v = v.padding(n),
                PaddingVal::Xy(x, y) => v = v.padding_xy(x, y),
            },
            "padding_x" => v = v.padding_x(num(file, arg, res, row)?),
            "padding_y" => v = v.padding_y(num(file, arg, res, row)?),
            "background" => v = v.background(color(file, theme, arg)?),
            "flex" => v = v.flex(num(file, arg, res, row)?),
            "radius" => v = v.radius(num(file, arg, res, row)?),
            "border" => {
                let (w, c) = border_pair(file, theme, arg)?;
                v = v.border(w, c);
            }
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "overflow" => v = v.overflow(overflow_val(file, arg)?),
            "hit" => v = v.hit_testable(flag(file, arg, res, row)?),
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    for c in &node.children {
        let pos = stack_pos_of(file, c)?;
        let (dx, dy) = stack_offset_of(file, res, row, c)?;
        let z = stack_z_of(file, res, row, c)?;
        let pos = pos.offset(dx, dy).z(z);
        let clean = without_placement(c);
        let kid = lower_node(file, theme, styles, &clean, res, row)?;

        let _ = child_flex_weight(file, res, row, c)?;
        v = v.child_at(kid, pos);
    }
    Ok(AnyView::new(v))
}

fn track_of(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
) -> Result<Track, SeqError> {
    match value {
        SeqValue::Length(l) => match *l {
            LengthVal::Fit => Ok(Track::Fit),
            LengthVal::Px(n) => Ok(Track::Px(n)),
            LengthVal::Fill(w) => Ok(Track::Fill(w)),
            LengthVal::Percent(p) => Ok(Track::Percent(p)),
        },
        SeqValue::Num(n) => Ok(Track::Px(*n)),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(file, line, 1, "tor to px()/fill()/fit")),
            Resolver::Host(b) => Ok(Track::Px(prop_num_env(file, line, b, prop)?)),
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(file, line, 1, "tor to px()/fill()/fit")),
            Resolver::Host(b) => Ok(Track::Px(env_num(file, line, b, name)?)),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(file, line, 1, "tor to px()/fill()/fit")),
            Resolver::Host(_) => Ok(Track::Px(row_num(file, line, row, field)?)),
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            "tor to px(n)/fill(w)/fit/percent(n)",
        )),
    }
}

fn tracks_of(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
    name: &str,
) -> Result<Vec<Track>, SeqError> {
    match find_arg(node, name) {
        None => Ok(vec![Track::Fill(1.0)]),
        Some(arg) => match &arg.value {
            SeqValue::Array(items) if !items.is_empty() => items
                .iter()
                .map(|it| track_of(file, arg.line, res, row, it))
                .collect(),
            _ => Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("'{name}' to [px(100), fill(1), ...] (min. 1 tor)"),
            )),
        },
    }
}

fn grid_int(
    file: &str,
    res: &Resolver<'_>,
    _row: Option<&SeqRowData>,
    node: &SeqNode,
    name: &str,
) -> Result<Option<usize>, SeqError> {
    match find_arg(node, name) {
        None => Ok(None),
        Some(arg) => match &arg.value {
            SeqValue::Num(n) => {
                if n.is_finite() && *n >= 0.0 {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    Ok(Some(*n as usize))
                } else {
                    Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("'{name}' to liczba >= 0"),
                    ))
                }
            }
            SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
                Resolver::Static => Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!("'{name}' to liczba"),
                )),
                Resolver::Host(b) => {
                    let v = prop_num_env(file, arg.line, b, prop)?;
                    if v.is_finite() && v >= 0.0 {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        Ok(Some(v as usize))
                    } else {
                        Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            format!("'{name}' to liczba >= 0"),
                        ))
                    }
                }
            },
            _ => Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("'{name}' to liczba"),
            )),
        },
    }
}

fn lower_grid(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let columns = tracks_of(file, res, row, node, "columns")?;
    let mut v = Grid::new(columns);
    for arg in &node.args {
        match arg.name.as_str() {
            "columns" => {}
            "rows" => {
                v = match &arg.value {
                    SeqValue::Array(items) if !items.is_empty() => {
                        let tracks: Vec<Track> = items
                            .iter()
                            .map(|it| track_of(file, arg.line, res, row, it))
                            .collect::<Result<_, _>>()?;
                        v.rows(tracks)
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "'rows' to [px(40), fill(1), ...]",
                        ));
                    }
                };
            }
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "padding" => match padding_val(file, arg, res, row)? {
                PaddingVal::All(n) => v = v.padding(n),
                PaddingVal::Xy(x, y) => v = v.padding_xy(x, y),
            },
            "padding_x" => v = v.padding_x(num(file, arg, res, row)?),
            "padding_y" => v = v.padding_y(num(file, arg, res, row)?),
            "background" => v = v.background(color(file, theme, arg)?),
            "flex" => v = v.flex(num(file, arg, res, row)?),
            "radius" => v = v.radius(num(file, arg, res, row)?),
            "border" => {
                let (w, c) = border_pair(file, theme, arg)?;
                v = v.border(w, c);
            }
            "gap" => v = v.gap(num(file, arg, res, row)?),
            "column_gap" => v = v.column_gap(num(file, arg, res, row)?),
            "row_gap" => v = v.row_gap(num(file, arg, res, row)?),
            "auto_flow" => {
                v = match ident(file, arg)?.as_str() {
                    "row" => v.auto_flow(GridFlow::Row),
                    "column" => v.column_flow(),
                    other => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            format!("auto_flow to row/column, nie '{other}'"),
                        ));
                    }
                };
            }
            "align" => v = v.align_items(align_val(file, arg)?),
            "justify" => v = v.justify_items(align_val(file, arg)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "overflow" => v = v.overflow(overflow_val(file, arg)?),
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    for c in &node.children {
        let clean = without_placement(c);
        let kid = lower_node(file, theme, styles, &clean, res, row)?;
        let _ = child_flex_weight(file, res, row, c)?;
        let col = grid_int(file, res, row, c, "col")?;
        let row_i = grid_int(file, res, row, c, "row")?;
        let col_span = grid_int(file, res, row, c, "col_span")?.unwrap_or(1).max(1);
        let row_span = grid_int(file, res, row, c, "row_span")?.unwrap_or(1).max(1);
        match (col, row_i) {
            (Some(cc), Some(rr)) => {
                v = v.child_at(
                    kid,
                    GridPlacement::new().at(cc, rr).span(col_span, row_span),
                );
            }
            (Some(cc), None) => {
                v = v.child_at(
                    kid,
                    GridPlacement::new().column(cc).span(col_span, row_span),
                );
            }
            (None, Some(rr)) => {
                v = v.child_at(kid, GridPlacement::new().row(rr).span(col_span, row_span));
            }
            (None, None) if col_span > 1 || row_span > 1 => {
                v = v.child_at(kid, GridPlacement::new().span(col_span, row_span));
            }
            (None, None) => {
                v = v.child(kid);
            }
        }
    }
    Ok(AnyView::new(v))
}

fn lower_wrap(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let mut v = Wrap::new();
    for arg in &node.args {
        match arg.name.as_str() {
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "padding" => match padding_val(file, arg, res, row)? {
                PaddingVal::All(n) => v = v.padding(n),
                PaddingVal::Xy(x, y) => v = v.padding_xy(x, y),
            },
            "padding_x" => v = v.padding_x(num(file, arg, res, row)?),
            "padding_y" => v = v.padding_y(num(file, arg, res, row)?),
            "background" => v = v.background(color(file, theme, arg)?),
            "flex" => v = v.flex(num(file, arg, res, row)?),
            "axis" => {
                v = match ident(file, arg)?.as_str() {
                    "horizontal" => v.axis(Axis::Horizontal),
                    "vertical" => v.axis(Axis::Vertical),
                    other => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            format!("axis to horizontal/vertical, nie '{other}'"),
                        ));
                    }
                };
            }
            "gap" => v = v.gap(num(file, arg, res, row)?),
            "column_gap" => v = v.column_gap(num(file, arg, res, row)?),
            "row_gap" => v = v.row_gap(num(file, arg, res, row)?),
            "align" => v = v.align_items(align_val(file, arg)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "overflow" => v = v.overflow(overflow_val(file, arg)?),
            "hit" => v = v.hit_testable(flag(file, arg, res, row)?),
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    for c in &node.children {
        let clean = without_placement(c);
        let kid = lower_node(file, theme, styles, &clean, res, row)?;
        let _ = child_flex_weight(file, res, row, c)?;
        v = v.child(kid);
    }
    Ok(AnyView::new(v))
}

fn lower_masonry(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let columns = count_val(file, node, res, row, "columns")?
        .unwrap_or(2)
        .max(1);
    let mut v = Masonry::new(columns);
    for arg in &node.args {
        match arg.name.as_str() {
            "columns" => {}
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "padding" => match padding_val(file, arg, res, row)? {
                PaddingVal::All(n) => v = v.padding(n),
                PaddingVal::Xy(x, y) => v = v.padding_xy(x, y),
            },
            "padding_x" => v = v.padding_x(num(file, arg, res, row)?),
            "padding_y" => v = v.padding_y(num(file, arg, res, row)?),
            "background" => v = v.background(color(file, theme, arg)?),
            "flex" => v = v.flex(num(file, arg, res, row)?),
            "gap" => v = v.gap(num(file, arg, res, row)?),
            "column_gap" => v = v.column_gap(num(file, arg, res, row)?),
            "row_gap" => v = v.row_gap(num(file, arg, res, row)?),
            "align" => v = v.align_items(align_val(file, arg)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "overflow" => v = v.overflow(overflow_val(file, arg)?),
            "hit" => v = v.hit_testable(flag(file, arg, res, row)?),
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    for c in &node.children {
        let clean = without_placement(c);
        let kid = lower_node(file, theme, styles, &clean, res, row)?;
        let _ = child_flex_weight(file, res, row, c)?;
        v = v.child(kid);
    }
    Ok(AnyView::new(v))
}

fn when_signal(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
) -> Result<Signal<bool>, SeqError> {
    match value {
        SeqValue::Bool(b) => Ok(Signal::new(*b)),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                "'when' z propertu wymaga hosta",
            )),
            Resolver::Host(b) => {
                if let Some(sig) = b.get_bool(prop) {
                    Ok(sig)
                } else if let Some(m) = b.get_memo_bool(prop) {
                    let sig = Signal::new(m.get_untracked());
                    let s2 = sig.clone();
                    effect(move || {
                        let v = m.get();
                        s2.set(v);
                    })
                    .forget();
                    Ok(sig)
                } else {
                    Err(prop_err(file, line, prop, "Bool"))
                }
            }
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                "'when' from environment requires host",
            )),
            Resolver::Host(b) => b.get_env_bool(name).ok_or_else(|| {
                SeqError::new(
                    file,
                    line,
                    1,
                    format!("unknown env.{name} or wrong type (here you need: Bool)"),
                )
            }),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                "'when' z wiersza wymaga hosta",
            )),
            Resolver::Host(_) => match row_value(file, line, row, field)? {
                RowVal::Sig(FieldSignal::Bool(sig)) => Ok(sig),
                _ => Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("pole wiersza '{field}' to nie Bool"),
                )),
            },
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            "'when' to true/false albo dynamic(nazwa)",
        )),
    }
}

fn lower_if(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let when_arg = find_arg(node, "when").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "If wymaga when: true/false/dynamic()".to_string(),
        )
    })?;

    if let SeqValue::Bool(b) = &when_arg.value {
        if !b {
            return Ok(AnyView::new(Column::new()));
        }
        return match node.children.as_slice() {
            [] => Ok(AnyView::new(Column::new())),
            [single] => lower_node(file, theme, styles, single, res, row),
            _ => {
                let mut col = Column::new();
                for c in &node.children {
                    let kid = lower_node(file, theme, styles, c, res, row)?;
                    col = match child_flex_weight(file, res, row, c)? {
                        Some(w) => col.child_flex(kid, w),
                        None => col.child(kid),
                    };
                }
                Ok(AnyView::new(col))
            }
        };
    }
    let bindings = match res {
        Resolver::Static => {
            return Err(SeqError::new(
                file,
                when_arg.line,
                1,
                "If(when: prop) wymaga hosta Rust (HostBindings::prop_bool)",
            ));
        }
        Resolver::Host(b) => (*b).clone(),
    };
    let cond = when_signal(file, when_arg.line, res, row, &when_arg.value)?;
    let template = node.children.clone();
    let file_owned = file.to_string();
    let theme_owned = *theme;
    let styles_owned = styles.clone();
    let branch = move || -> AnyView {
        let res = Resolver::Host(&bindings);
        match template.as_slice() {
            [] => AnyView::new(Column::new()),
            [single] => lower_node(&file_owned, &theme_owned, &styles_owned, single, &res, row)
                .unwrap_or_else(|e| AnyView::new(Text::new(format!("seq: {e}")))),
            _ => {
                let mut col = Column::new();
                for c in &template {
                    match lower_node(&file_owned, &theme_owned, &styles_owned, c, &res, row) {
                        Ok(kid) => {
                            let w = child_flex_weight(&file_owned, &res, row, c).unwrap_or(None);
                            col = match w {
                                Some(w) => col.child_flex(kid, w),
                                None => col.child(kid),
                            };
                        }
                        Err(e) => {
                            col = col.child(Text::new(format!("seq: {e}")));
                        }
                    }
                }
                AnyView::new(col)
            }
        }
    };

    let _ = cond.get();
    let mut view = when(cond, branch());
    if let Some(arg) = find_arg(node, "enter") {
        let spec = anim_spec_of(file, arg)?;
        view = view.enter(Motion::from(spec));
    }
    if let Some(arg) = find_arg(node, "exit") {
        let spec = anim_spec_of(file, arg)?;
        view = view.exit(Motion::from(spec));
    }
    Ok(AnyView::new(view))
}

fn lower_animation_child(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    match node.children.as_slice() {
        [] => Ok(AnyView::new(Column::new())),
        [single] => lower_node(file, theme, styles, single, res, row),
        many => {
            let mut col = Column::new();
            for c in many {
                let kid = lower_node(file, theme, styles, c, res, row)?;
                col = col.child(kid);
            }
            Ok(AnyView::new(col))
        }
    }
}

fn anim_spec_of(file: &str, arg: &SeqArg) -> Result<AnimSpec, SeqError> {
    let name = match &arg.value {
        SeqValue::Ident(s) | SeqValue::Dynamic(s) => s.as_str(),
        _ => {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                "animate/enter/exit: nazwa presetu (fast/smooth/snappy/gentle/slow)".to_string(),
            ));
        }
    };
    Ok(match name {
        "fast" => AnimSpec::FAST,
        "smooth" => AnimSpec::SMOOTH,
        "snappy" => AnimSpec::SNAPPY,
        "gentle" => AnimSpec::GENTLE,
        "slow" => AnimSpec::SLOW,
        _ => {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("nieznany preset animacji '{name}'"),
            ));
        }
    })
}

fn lower_opacity(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let child = lower_animation_child(file, theme, styles, node, res, row)?;
    let sig = if let Some(arg) = find_arg(node, "value") {
        bridge_f32(file, arg.line, res, row, &arg.value, "value")?
    } else {
        Signal::new(1.0_f32)
    };
    let mut v = LiveOpacity::new(child, Live::from(sig));
    if let Some(arg) = find_arg(node, "animate") {
        v = v.animate(anim_spec_of(file, arg)?);
    }
    Ok(AnyView::new(v))
}

fn lower_scale(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let child = lower_animation_child(file, theme, styles, node, res, row)?;
    let sig = if let Some(arg) = find_arg(node, "value") {
        bridge_f32(file, arg.line, res, row, &arg.value, "value")?
    } else {
        Signal::new(1.0_f32)
    };
    let mut v = Scaled::new(child, Live::from(sig));
    if let Some(arg) = find_arg(node, "animate") {
        v = v.animate(anim_spec_of(file, arg)?);
    }
    Ok(AnyView::new(v))
}

fn lower_rotate(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let child = lower_animation_child(file, theme, styles, node, res, row)?;
    let sig = if let Some(arg) = find_arg(node, "value") {
        bridge_f32(file, arg.line, res, row, &arg.value, "value")?
    } else {
        Signal::new(0.0_f32)
    };
    let mut v = Rotated::new(child, Live::from(sig));
    if let Some(arg) = find_arg(node, "animate") {
        v = v.animate(anim_spec_of(file, arg)?);
    }
    Ok(AnyView::new(v))
}

fn lower_width(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let child = lower_animation_child(file, theme, styles, node, res, row)?;
    let sig = if let Some(arg) = find_arg(node, "value") {
        bridge_f32(file, arg.line, res, row, &arg.value, "value")?
    } else {
        Signal::new(0.0_f32)
    };
    let mut v = AnimatedSize::width(child, Live::from(sig));
    if let Some(arg) = find_arg(node, "animate") {
        v = v.animate(anim_spec_of(file, arg)?);
    }
    Ok(AnyView::new(v))
}

fn lower_height(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let child = lower_animation_child(file, theme, styles, node, res, row)?;
    let sig = if let Some(arg) = find_arg(node, "value") {
        bridge_f32(file, arg.line, res, row, &arg.value, "value")?
    } else {
        Signal::new(0.0_f32)
    };
    let mut v = AnimatedSize::height(child, Live::from(sig));
    if let Some(arg) = find_arg(node, "animate") {
        v = v.animate(anim_spec_of(file, arg)?);
    }
    Ok(AnyView::new(v))
}

fn lower_translate(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let child = lower_animation_child(file, theme, styles, node, res, row)?;
    let x_sig = if let Some(arg) = find_arg(node, "x") {
        bridge_f32(file, arg.line, res, row, &arg.value, "x")?
    } else {
        Signal::new(0.0_f32)
    };
    let y_sig = if let Some(arg) = find_arg(node, "y") {
        bridge_f32(file, arg.line, res, row, &arg.value, "y")?
    } else {
        Signal::new(0.0_f32)
    };
    let x_live = x_sig.clone();
    let y_live = y_sig.clone();
    let live = Live::<Vec2>::derived(move || Vec2::new(x_live.get(), y_live.get()));
    let mut v = LiveOffset::new(child, live);
    if let Some(arg) = find_arg(node, "animate") {
        v = v.animate(anim_spec_of(file, arg)?);
    }
    Ok(AnyView::new(v))
}

fn lower_pulse(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let child = lower_animation_child(file, theme, styles, node, res, row)?;
    let mut v = Pulse::new(child);
    if let Some(arg) = find_arg(node, "min") {
        v = v.min_opacity(num(file, arg, res, row)?);
    }
    if let Some(arg) = find_arg(node, "period") {
        v = v.period_duration(std::time::Duration::from_secs_f32(num(
            file, arg, res, row,
        )?));
    }
    Ok(AnyView::new(v))
}

fn lower_shake(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let child = lower_animation_child(file, theme, styles, node, res, row)?;
    let mut v = Shake::new(child);
    if let Some(arg) = find_arg(node, "amplitude") {
        v = v.amplitude(num(file, arg, res, row)?);
    }
    if let Some(arg) = find_arg(node, "period") {
        v = v.period_duration(std::time::Duration::from_secs_f32(num(
            file, arg, res, row,
        )?));
    }
    if let Some(arg) = find_arg(node, "duration") {
        v = v.total_duration(std::time::Duration::from_secs_f32(num(
            file, arg, res, row,
        )?));
    }
    if let Some(arg) = find_arg(node, "delay") {
        v = v.delay_duration(std::time::Duration::from_secs_f32(num(
            file, arg, res, row,
        )?));
    }
    Ok(AnyView::new(v))
}

fn lower_async(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let source = find_arg(node, "source").ok_or_else(|| {
        SeqError::new(file, node.line, 1, "Async wymaga source: nazwa".to_string())
    })?;
    let source_name = match &source.value {
        SeqValue::Ident(name) => name.clone(),
        _ => {
            return Err(SeqError::new(
                file,
                source.line,
                1,
                "source: to nazwa zasobu z hosta (source: users)",
            ));
        }
    };
    let bindings = match res {
        Resolver::Static => {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                "Async(source:) wymaga hosta Rust (HostBindings::resource())",
            ));
        }
        Resolver::Host(b) => (*b).clone(),
    };
    let resource = bindings.get_resource(&source_name).ok_or_else(|| {
        SeqError::new(
            file,
            source.line,
            1,
            format!("unknown resource '{source_name}' (HostBindings::resource())"),
        )
    })?;

    let mut loading: Vec<SeqNode> = Vec::new();
    let mut data: Vec<SeqNode> = Vec::new();
    let mut error: Vec<SeqNode> = Vec::new();
    let mut idle: Vec<SeqNode> = Vec::new();
    let mut cancelled: Vec<SeqNode> = Vec::new();
    for c in &node.children {
        match c.kind.as_str() {
            "Loading" => loading.extend(c.children.clone()),
            "Data" => data.extend(c.children.clone()),
            "Error" => error.extend(c.children.clone()),
            "Idle" => idle.extend(c.children.clone()),
            "Cancelled" => cancelled.extend(c.children.clone()),
            other => {
                return Err(SeqError::new(
                    file,
                    c.line,
                    1,
                    format!("interior Async is Loading/Data/Error/Idle/Cancelled, not '{other}'"),
                ));
            }
        }
    }

    let data_sig = Signal::new(String::new());
    let error_sig = Signal::new(String::new());
    {
        let res_sync = resource.clone();
        let data_sig = data_sig.clone();
        let error_sig = error_sig.clone();
        effect(move || match res_sync.state().get() {
            sequoia_async::AsyncState::Ready(v) => data_sig.set(v.clone()),
            sequoia_async::AsyncState::Failed(e) => error_sig.set(e.clone()),
            _ => {}
        })
        .forget();
    }
    let lower_once = |items: &[SeqNode], extra: Option<(&str, Signal<String>)>| -> AnyView {
        let mut b2 = bindings.clone();
        if let Some((k, sig)) = extra {
            b2 = b2.prop_str(k, sig);
        }
        let res = Resolver::Host(&b2);
        match items {
            [] => AnyView::new(Column::new()),
            [single] => lower_node(file, theme, styles, single, &res, row)
                .unwrap_or_else(|e| AnyView::new(Text::new(format!("seq: {e}")))),
            _ => {
                let mut col = Column::new();
                for n in items {
                    match lower_node(file, theme, styles, n, &res, row) {
                        Ok(kid) => {
                            let w = child_flex_weight(file, &res, row, n).unwrap_or(None);
                            col = match w {
                                Some(w) => col.child_flex(kid, w),
                                None => col.child(kid),
                            };
                        }
                        Err(e) => {
                            col = col.child(Text::new(format!("seq: {e}")));
                        }
                    }
                }
                AnyView::new(col)
            }
        }
    };
    let loading_view = lower_once(&loading, None);
    let data_view = lower_once(&data, Some(("data", data_sig)));
    let error_view = lower_once(&error, Some(("error", error_sig)));
    let mut view = AsyncView::new(resource);
    {
        let v = loading_view.clone();
        view = view.loading(move || v.clone());
    }
    {
        let v = data_view.clone();
        view = view.data(move |_: &String| v.clone());
    }
    {
        let v = error_view.clone();
        view = view.error(move |_: &str| v.clone());
    }
    if !idle.is_empty() {
        let v = lower_once(&idle, None);
        view = view.idle(move || v.clone());
    }
    if !cancelled.is_empty() {
        let v = lower_once(&cancelled, None);
        view = view.cancelled(move || v.clone());
    }
    Ok(AnyView::new(view))
}

fn lower_effect(
    file: &str,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let track = find_arg(node, "track").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "Effect wymaga track: dynamic()".to_string(),
        )
    })?;
    let on_run = find_arg(node, "on_run").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "Effect wymaga on_run: { akcja() }".to_string(),
        )
    })?;
    let bindings = match res {
        Resolver::Static => {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                "Effect wymaga hosta Rust",
            ));
        }
        Resolver::Host(b) => (*b).clone(),
    };
    let touch = track_touch(file, track.line, &bindings, row, &track.value)?;
    let fire: Rc<dyn Fn()> = match &on_run.value {
        SeqValue::Callback { action, args } => {
            if !args.is_empty() {
                return Err(SeqError::new(
                    file,
                    on_run.line,
                    1,
                    "on_run: no arguments (read status via track:)",
                ));
            }
            bindings.get_action0(action).ok_or_else(|| {
                SeqError::new(
                    file,
                    on_run.line,
                    1,
                    format!("unknown callback '{action}' (missing on host: on)"),
                )
            })?
        }
        SeqValue::CallSeq { name, args } => {
            if !args.is_empty() {
                return Err(SeqError::new(
                    file,
                    on_run.line,
                    1,
                    "on_run: no arguments (read status via track:)",
                ));
            }
            seq_fire(file, on_run.line, &bindings, name, Vec::new())?
        }
        _ => {
            return Err(SeqError::new(
                file,
                on_run.line,
                1,
                "on_run: { akcja(args) }",
            ));
        }
    };
    let seen = std::rc::Rc::new(std::cell::Cell::new(false));
    effect(move || {
        touch();
        if seen.get() {
            fire();
        } else {
            seen.set(true);
        }
    })
    .forget();
    Ok(AnyView::new(Column::new()))
}

fn track_touch(
    file: &str,
    line: usize,
    b: &HostBindings,
    row: Option<&SeqRowData>,
    value: &SeqValue,
) -> Result<Rc<dyn Fn()>, SeqError> {
    match value {
        SeqValue::Bool(_) | SeqValue::Num(_) | SeqValue::Str(_) | SeqValue::Color(_) => {
            Ok(Rc::new(|| {}))
        }
        SeqValue::Ident(name) | SeqValue::Dynamic(name) => {
            if let Some(sig) = b.get_str(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(m) = b.get_memo_str(name) {
                Ok(Rc::new(move || {
                    m.get();
                }))
            } else if let Some(sig) = b.get_bool(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(m) = b.get_memo_bool(name) {
                Ok(Rc::new(move || {
                    m.get();
                }))
            } else if let Some(sig) = b.get_int(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(m) = b.get_memo_int(name) {
                Ok(Rc::new(move || {
                    m.get();
                }))
            } else if let Some(sig) = b.get_float(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(m) = b.get_memo_float(name) {
                Ok(Rc::new(move || {
                    m.get();
                }))
            } else if let Some(sig) = b.get_env_str(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(sig) = b.get_env_bool(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(sig) = b.get_env_int(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(sig) = b.get_env_float(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else {
                Err(prop_err(file, line, name, "String/Bool/Int/Float"))
            }
        }
        SeqValue::Env(name) => {
            if let Some(sig) = b.get_env_str(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(sig) = b.get_env_bool(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(sig) = b.get_env_int(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else if let Some(sig) = b.get_env_float(name) {
                Ok(Rc::new(move || {
                    sig.get();
                }))
            } else {
                Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("unknown env.{name} (missing on host)"),
                ))
            }
        }
        SeqValue::ItemField(field) => match row_value(file, line, row, field)? {
            RowVal::Sig(FieldSignal::Str(sig)) => Ok(Rc::new(move || {
                sig.get();
            })),
            RowVal::Sig(FieldSignal::Bool(sig)) => Ok(Rc::new(move || {
                sig.get();
            })),
            RowVal::Sig(FieldSignal::Int(sig)) => Ok(Rc::new(move || {
                sig.get();
            })),
            RowVal::Sig(FieldSignal::Float(sig)) => Ok(Rc::new(move || {
                sig.get();
            })),
            RowVal::Id(_) => Ok(Rc::new(|| {})),
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            "'track' to dynamic(nazwa)/env.nazwa/item.pole",
        )),
    }
}

fn lower_checkable(
    file: &str,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    is_switch: bool,
) -> Result<AnyView, SeqError> {
    let what = if is_switch { "Switch" } else { "Checkbox" };
    let checked_arg = find_arg(node, "checked").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            format!("{what} wymaga checked: true/false/prop"),
        )
    })?;
    let sig = writable_bool(
        file,
        checked_arg.line,
        res,
        row,
        &checked_arg.value,
        "checked",
    )?;
    let label = opt_str(file, node, "label", res, row)?;
    let mut font_size: Option<f32> = None;
    let mut disabled = false;
    let mut margin: Option<MarginVal> = None;
    let mut margin_x: Option<f32> = None;
    let mut margin_y: Option<f32> = None;
    let mut margin_left: Option<f32> = None;
    let mut margin_right: Option<f32> = None;
    let mut margin_top: Option<f32> = None;
    let mut margin_bottom: Option<f32> = None;
    let mut toggle: Option<Rc<dyn Fn()>> = None;
    for arg in &node.args {
        match arg.name.as_str() {
            "checked" => {}
            "label" => {}
            "font_size" => font_size = Some(num(file, arg, res, row)?),
            "enabled" => disabled = !flag(file, arg, res, row)?,
            "margin" => margin = Some(margin_val(file, arg, res, row)?),
            "margin_x" => margin_x = Some(num(file, arg, res, row)?),
            "margin_y" => margin_y = Some(num(file, arg, res, row)?),
            "margin_left" => margin_left = Some(num(file, arg, res, row)?),
            "margin_right" => margin_right = Some(num(file, arg, res, row)?),
            "margin_top" => margin_top = Some(num(file, arg, res, row)?),
            "margin_bottom" => margin_bottom = Some(num(file, arg, res, row)?),
            "on_toggle" => match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "on_toggle wymaga hosta Rust",
                    ));
                }
                Resolver::Host(b) => match &arg.value {
                    SeqValue::Callback { .. } | SeqValue::CallSeq { .. } => {
                        let live = LiveVal::Bool(sig.clone());
                        toggle = Some(wire_live_action(
                            file,
                            arg.line,
                            b,
                            &arg.value,
                            &live,
                            "on_toggle",
                        )?);
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "on_toggle: { akcja() } albo { akcja(value) }",
                        ));
                    }
                },
            },
            "flex" => {}
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    if is_switch {
        let mut v = Switch::new(sig);
        if let Some(label) = label {
            v = v.label(label);
        }
        if let Some(size) = font_size {
            v = v.font_size(size);
        }
        v = v.disabled(disabled);
        if let Some(fire) = toggle {
            v = v.on_toggle(move |_| fire());
        }
        v = apply_switch_margin(
            v,
            margin,
            margin_x,
            margin_y,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
        );
        Ok(AnyView::new(v))
    } else {
        let mut v = Checkbox::new(sig);
        if let Some(label) = label {
            v = v.label(label);
        }
        if let Some(size) = font_size {
            v = v.font_size(size);
        }
        v = v.disabled(disabled);
        if let Some(fire) = toggle {
            v = v.on_toggle(move |_| fire());
        }
        v = apply_checkbox_margin(
            v,
            margin,
            margin_x,
            margin_y,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
        );
        Ok(AnyView::new(v))
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_checkbox_margin(
    mut v: Checkbox,
    margin: Option<MarginVal>,
    margin_x: Option<f32>,
    margin_y: Option<f32>,
    margin_left: Option<f32>,
    margin_right: Option<f32>,
    margin_top: Option<f32>,
    margin_bottom: Option<f32>,
) -> Checkbox {
    if let Some(m) = margin {
        v = match m {
            MarginVal::All(n) => v.margin(n),
            MarginVal::Xy(x, y) => v.margin_xy(x, y),
        };
    }
    if let Some(x) = margin_x {
        v = v.margin_x(x);
    }
    if let Some(y) = margin_y {
        v = v.margin_y(y);
    }
    if let Some(x) = margin_left {
        v = v.margin_left(x);
    }
    if let Some(x) = margin_right {
        v = v.margin_right(x);
    }
    if let Some(y) = margin_top {
        v = v.margin_top(y);
    }
    if let Some(y) = margin_bottom {
        v = v.margin_bottom(y);
    }
    v
}

#[allow(clippy::too_many_arguments)]
fn apply_switch_margin(
    mut v: Switch,
    margin: Option<MarginVal>,
    margin_x: Option<f32>,
    margin_y: Option<f32>,
    margin_left: Option<f32>,
    margin_right: Option<f32>,
    margin_top: Option<f32>,
    margin_bottom: Option<f32>,
) -> Switch {
    if let Some(m) = margin {
        v = match m {
            MarginVal::All(n) => v.margin(n),
            MarginVal::Xy(x, y) => v.margin_xy(x, y),
        };
    }
    if let Some(x) = margin_x {
        v = v.margin_x(x);
    }
    if let Some(y) = margin_y {
        v = v.margin_y(y);
    }
    if let Some(x) = margin_left {
        v = v.margin_left(x);
    }
    if let Some(x) = margin_right {
        v = v.margin_right(x);
    }
    if let Some(y) = margin_top {
        v = v.margin_top(y);
    }
    if let Some(y) = margin_bottom {
        v = v.margin_bottom(y);
    }
    v
}

fn lower_checkbox(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    lower_checkable(file, node, res, row, false)
}

fn lower_switch(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    lower_checkable(file, node, res, row, true)
}

fn lower_textfield(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let value_arg = find_arg(node, "value").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "TextField wymaga value: prop".to_string(),
        )
    })?;
    let sig = writable_str(file, value_arg.line, res, row, &value_arg.value, "value")?;
    let mut v = TextField::new(sig.clone());
    for arg in &node.args {
        match arg.name.as_str() {
            "value" => {}
            "placeholder" => {
                if let Some(text) = opt_str(file, node, "placeholder", res, row)? {
                    v = v.placeholder(text);
                }
            }
            "font_size" => v = v.font_size(num(file, arg, res, row)?),
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "on_submit" => match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "on_submit wymaga hosta Rust",
                    ));
                }
                Resolver::Host(b) => match &arg.value {
                    SeqValue::Callback { .. } | SeqValue::CallSeq { .. } => {
                        let live = LiveVal::Str(sig.clone());
                        let fire =
                            wire_live_action(file, arg.line, b, &arg.value, &live, "on_submit")?;
                        v = v.on_submit(move |_| fire());
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "on_submit: { akcja() } albo { akcja(value) }",
                        ));
                    }
                },
            },
            "flex" => {}
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn count_val(
    file: &str,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    name: &str,
) -> Result<Option<usize>, SeqError> {
    let Some(arg) = find_arg(node, name) else {
        return Ok(None);
    };
    let err = || SeqError::new(file, arg.line, 1, format!("'{name}' is an integer >= 0"));
    match &arg.value {
        SeqValue::Num(n) => {
            if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 {
                Ok(Some(*n as usize))
            } else {
                Err(err())
            }
        }
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("'{name}' z propertu wymaga hosta"),
            )),
            Resolver::Host(b) => b
                .get_int(prop)
                .map(|sig| sig.get_untracked())
                .or_else(|| b.get_memo_int(prop).map(|m| m.get_untracked()))
                .or_else(|| b.get_env_int(prop).map(|sig| sig.get_untracked()))
                .map(|v| Some(v.max(0) as usize))
                .ok_or_else(|| prop_err(file, arg.line, prop, "Int")),
        },
        SeqValue::Env(env_name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("'{name}' from environment requires host"),
            )),
            Resolver::Host(b) => b
                .get_env_int(env_name)
                .map(|sig| Some(sig.get_untracked().max(0) as usize))
                .ok_or_else(|| {
                    SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("unknown env.{env_name} or wrong type (here you need: Int)"),
                    )
                }),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("'{name}' z wiersza wymaga hosta"),
            )),
            Resolver::Host(_) => match row_value(file, arg.line, row, field)? {
                RowVal::Sig(FieldSignal::Int(sig)) => Ok(Some(sig.get_untracked().max(0) as usize)),
                RowVal::Id(id) => Ok(Some(id.max(0) as usize)),
                _ => Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!("pole wiersza '{field}' to nie Int"),
                )),
            },
        },
        _ => Err(err()),
    }
}

fn label_arg(
    file: &str,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    kind: &str,
) -> Result<String, SeqError> {
    opt_str(file, node, "label", res, row)?
        .ok_or_else(|| SeqError::new(file, node.line, 1, format!("{kind} wymaga label: \"...\"")))
}

fn str_list_arg(file: &str, node: &SeqNode, name: &str) -> Result<Vec<String>, SeqError> {
    let arg = find_arg(node, name).ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            format!("{0} wymaga {name}: [\"...\", ...]", node.kind),
        )
    })?;
    match &arg.value {
        SeqValue::Array(items) if !items.is_empty() => items
            .iter()
            .map(|it| match it {
                SeqValue::Str(s) => Ok(s.clone()),
                _ => Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!("'{name}' to [\"...\", ...] (same napisy)"),
                )),
            })
            .collect(),
        _ => Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("'{name}' to niepusta [\"...\", ...]"),
        )),
    }
}

fn lower_textarea(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let value_arg = find_arg(node, "value").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "TextArea wymaga value: prop".to_string(),
        )
    })?;
    let sig = writable_str(file, value_arg.line, res, row, &value_arg.value, "value")?;
    let mut v = TextArea::new(sig.clone());
    for arg in &node.args {
        match arg.name.as_str() {
            "value" => {}
            "placeholder" => {
                if let Some(text) = opt_str(file, node, "placeholder", res, row)? {
                    v = v.placeholder(text);
                }
            }
            "max_length" => {
                if let Some(n) = count_val(file, node, res, row, "max_length")? {
                    v = v.max_length(n);
                }
            }
            "min_lines" => {
                if let Some(n) = count_val(file, node, res, row, "min_lines")? {
                    v = v.min_lines(n.max(1));
                }
            }
            "max_lines" => {
                if let Some(n) = count_val(file, node, res, row, "max_lines")? {
                    v = v.max_lines(n.max(1));
                }
            }
            "font_size" => v = v.font_size(num(file, arg, res, row)?),
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "flex" => {}
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "on_edit_end" => match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "on_edit_end wymaga hosta Rust",
                    ));
                }
                Resolver::Host(b) => match &arg.value {
                    SeqValue::Callback { .. } | SeqValue::CallSeq { .. } => {
                        let live = LiveVal::Str(sig.clone());
                        let fire =
                            wire_live_action(file, arg.line, b, &arg.value, &live, "on_edit_end")?;
                        v = v.on_edit_end(move |_| fire());
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "on_edit_end: { akcja() } albo { akcja(value) }",
                        ));
                    }
                },
            },
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

fn lower_form_field(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let value_arg = find_arg(node, "value")
        .ok_or_else(|| SeqError::new(file, node.line, 1, "Field wymaga value: prop".to_string()))?;
    let sig = writable_str(file, value_arg.line, res, row, &value_arg.value, "value")?;
    let label = label_arg(file, node, res, row, "Field")?;
    let mut v = Field::new(sig.clone(), label);
    for arg in &node.args {
        match arg.name.as_str() {
            "value" | "label" => {}
            "placeholder" => {
                if let Some(text) = opt_str(file, node, "placeholder", res, row)? {
                    v = v.placeholder(text);
                }
            }
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

fn lower_number_field(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let value_arg = find_arg(node, "value").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "NumberField wymaga value: prop".to_string(),
        )
    })?;
    let sig = bridge_f32(file, value_arg.line, res, row, &value_arg.value, "value")?;
    let label = label_arg(file, node, res, row, "NumberField")?;
    let mut v = NumberField::new(sig.clone(), label);
    let mut min = f32::NEG_INFINITY;
    let mut max = f32::INFINITY;
    for arg in &node.args {
        match arg.name.as_str() {
            "value" | "label" => {}
            "min" => min = num(file, arg, res, row)?,
            "max" => max = num(file, arg, res, row)?,
            "step" => v = v.step(num(file, arg, res, row)?),
            "precision" => {
                if let Some(n) = count_val(file, node, res, row, "precision")? {
                    v = v.precision(n);
                }
            }
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    if min > max {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            "NumberField wymaga min <= max",
        ));
    }
    v = v.min(min).max(max);
    Ok(AnyView::new(v))
}

fn lower_date_field(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let value_arg = find_arg(node, "value").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "DateField wymaga value: prop".to_string(),
        )
    })?;
    let sig = writable_str(file, value_arg.line, res, row, &value_arg.value, "value")?;
    let label = label_arg(file, node, res, row, "DateField")?;
    let v = DateField::new(sig.clone(), label);
    for arg in &node.args {
        match arg.name.as_str() {
            "value" | "label" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

fn lower_multi_select(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let options = str_list_arg(file, node, "options")?;
    let label = label_arg(file, node, res, row, "MultiSelect")?;
    let mut v = MultiSelect::new(options.clone(), label);
    if let Some(n) = count_val(file, node, res, row, "min_selected")? {
        if n > options.len() {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                format!(
                    "min_selected ({n}) over number of options ({})",
                    options.len()
                ),
            ));
        }
        v = v.min_selected(n);
    }
    for arg in &node.args {
        match arg.name.as_str() {
            "options" | "label" | "min_selected" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

fn lower_drag_value(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let value_arg = find_arg(node, "value").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "DragValue wymaga value: prop".to_string(),
        )
    })?;
    let sig = bridge_f32(file, value_arg.line, res, row, &value_arg.value, "value")?;
    let mut v = DragValue::new(sig.clone());
    let mut min = f32::NEG_INFINITY;
    let mut max = f32::INFINITY;
    for arg in &node.args {
        match arg.name.as_str() {
            "value" => {}
            "min" => min = num(file, arg, res, row)?,
            "max" => max = num(file, arg, res, row)?,
            "step" => v = v.step(num(file, arg, res, row)?),
            "precision" => {
                if let Some(n) = count_val(file, node, res, row, "precision")? {
                    v = v.precision(n);
                }
            }
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "flex" => {}
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "on_commit" => match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "on_commit wymaga hosta Rust",
                    ));
                }
                Resolver::Host(b) => match &arg.value {
                    SeqValue::Callback { .. } | SeqValue::CallSeq { .. } => {
                        let live = LiveVal::F32(sig.clone());
                        let fire =
                            wire_live_action(file, arg.line, b, &arg.value, &live, "on_commit")?;
                        v = v.on_commit(move |_| fire());
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "on_commit: { akcja() } albo { akcja(value) }",
                        ));
                    }
                },
            },
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    if min > max {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            "DragValue wymaga min <= max",
        ));
    }
    v = v.min(min).max(max);
    Ok(AnyView::new(v))
}

fn lower_slider(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let value_arg = find_arg(node, "value").ok_or_else(|| {
        SeqError::new(file, node.line, 1, "Slider wymaga value: prop".to_string())
    })?;
    let sig = bridge_f32(file, value_arg.line, res, row, &value_arg.value, "value")?;
    let mut min = 0.0;
    let mut max = 1.0;
    if let Some(arg) = find_arg(node, "min") {
        min = num(file, arg, res, row)?;
    }
    if let Some(arg) = find_arg(node, "max") {
        max = num(file, arg, res, row)?;
    }
    if !(min.is_finite() && max.is_finite() && min < max) {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            "Slider requires min < max (finished)",
        ));
    }
    let mut v = Slider1D::new(sig.clone(), min, max);
    for arg in &node.args {
        match arg.name.as_str() {
            "value" | "min" | "max" => {}
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "on_change" => match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "on_change wymaga hosta Rust",
                    ));
                }
                Resolver::Host(b) => match &arg.value {
                    SeqValue::Callback { .. } | SeqValue::CallSeq { .. } => {
                        let live = LiveVal::F32(sig.clone());
                        let fire =
                            wire_live_action(file, arg.line, b, &arg.value, &live, "on_change")?;
                        v = v.on_commit(move |_| fire());
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "on_change: { akcja() } albo { akcja(value) }",
                        ));
                    }
                },
            },
            "flex" => {}
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

fn lower_dropdown(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let sel_arg = find_arg(node, "selected").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "Dropdown wymaga selected: prop".to_string(),
        )
    })?;
    let sig = bridge_usize(file, sel_arg.line, res, row, &sel_arg.value, "selected")?;
    let items_arg = find_arg(node, "items").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "Dropdown wymaga items: [\"A\", \"B\"]".to_string(),
        )
    })?;
    let options = match &items_arg.value {
        SeqValue::Array(items) => items
            .iter()
            .map(|it| match it {
                SeqValue::Str(s) => Ok(s.clone()),
                _ => Err(SeqError::new(
                    file,
                    items_arg.line,
                    1,
                    "items: to [\"tekst\", ...]",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(SeqError::new(
                file,
                items_arg.line,
                1,
                "items: to [\"tekst\", ...]",
            ));
        }
    };
    let mut v = DropdownButton::new(sig.clone(), options);
    for arg in &node.args {
        match arg.name.as_str() {
            "selected" | "items" => {}
            "placeholder" => {
                if let Some(text) = opt_str(file, node, "placeholder", res, row)? {
                    v = v.placeholder(text);
                }
            }
            "font_size" => v = v.font_size(num(file, arg, res, row)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "enabled" => v = v.disabled(!flag(file, arg, res, row)?),
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "on_select" => match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "on_select wymaga hosta Rust",
                    ));
                }
                Resolver::Host(b) => match &arg.value {
                    SeqValue::Callback { .. } | SeqValue::CallSeq { .. } => {
                        let live = LiveVal::Usize(sig.clone());
                        let fire =
                            wire_live_action(file, arg.line, b, &arg.value, &live, "on_select")?;
                        v = v.on_select(move |_| fire());
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "on_select: { akcja() } albo { akcja(value) }",
                        ));
                    }
                },
            },
            "flex" => {}
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

fn lower_progress(
    file: &str,
    _theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let value_arg = find_arg(node, "value").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "Progress wymaga value: prop".to_string(),
        )
    })?;
    let sig = live_f32(file, value_arg.line, res, row, &value_arg.value, "value")?;
    let mut v = ProgressBar::new(sig);
    for arg in &node.args {
        match arg.name.as_str() {
            "value" => {}
            "height" => v = v.height(num(file, arg, res, row)?),
            "track" => v = v.track(color(file, _theme, arg)?),
            "fill" => v = v.fill(color(file, _theme, arg)?),
            "flex" => {}
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    Ok(AnyView::new(v))
}

fn writable_bool(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<Signal<bool>, SeqError> {
    match value {
        SeqValue::Bool(b) => Ok(Signal::new(*b)),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z propertu wymaga hosta"),
            )),
            Resolver::Host(b) => {
                if let Some(sig) = b.get_bool(prop) {
                    Ok(sig)
                } else if b.get_memo_bool(prop).is_some() {
                    Err(SeqError::new(
                        file,
                        line,
                        1,
                        format!("'{what}' z memo jest tylko do odczytu — podepnij prop Bool"),
                    ))
                } else {
                    Err(prop_err(file, line, prop, "Bool"))
                }
            }
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' from environment requires host"),
            )),
            Resolver::Host(b) => b.get_env_bool(name).ok_or_else(|| {
                SeqError::new(
                    file,
                    line,
                    1,
                    format!("unknown env.{name} or wrong type (here you need: Bool)"),
                )
            }),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z wiersza wymaga hosta"),
            )),
            Resolver::Host(_) => match row_value(file, line, row, field)? {
                RowVal::Sig(FieldSignal::Bool(sig)) => Ok(sig),
                _ => Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("pole wiersza '{field}' to nie Bool"),
                )),
            },
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to true/false albo nazwa propertu"),
        )),
    }
}

fn writable_str(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<Signal<String>, SeqError> {
    match value {
        SeqValue::Str(s) => Ok(Signal::new(s.clone())),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z propertu wymaga hosta"),
            )),
            Resolver::Host(b) => {
                if let Some(sig) = b.get_str(prop) {
                    Ok(sig)
                } else if b.get_memo_str(prop).is_some() {
                    Err(SeqError::new(
                        file,
                        line,
                        1,
                        format!("'{what}' z memo jest tylko do odczytu — podepnij prop String"),
                    ))
                } else {
                    Err(prop_err(file, line, prop, "String"))
                }
            }
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' from environment requires host"),
            )),
            Resolver::Host(b) => b.get_env_str(name).ok_or_else(|| {
                SeqError::new(
                    file,
                    line,
                    1,
                    format!("unknown env.{name} or wrong type (here you need: String)"),
                )
            }),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z wiersza wymaga hosta"),
            )),
            Resolver::Host(_) => match row_value(file, line, row, field)? {
                RowVal::Sig(FieldSignal::Str(sig)) => Ok(sig),
                _ => Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("pole wiersza '{field}' to tekst (String), nie '{field}'"),
                )),
            },
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to napis albo nazwa propertu"),
        )),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn bridge_f32(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<Signal<f32>, SeqError> {
    match value {
        SeqValue::Num(n) => Ok(Signal::new(*n)),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z propertu wymaga hosta"),
            )),
            Resolver::Host(b) => {
                if let Some(host) = b.get_float(prop) {
                    Ok(sync_f64_f32(&host))
                } else if let Some(host) = b.get_int(prop) {
                    Ok(sync_i64_f32(&host))
                } else if b.get_memo_float(prop).is_some() || b.get_memo_int(prop).is_some() {
                    Err(SeqError::new(
                        file,
                        line,
                        1,
                        format!("'{what}' z memo jest tylko do odczytu — podepnij prop Float/Int"),
                    ))
                } else {
                    Err(prop_err(file, line, prop, "Float/Int"))
                }
            }
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' from environment requires host"),
            )),
            Resolver::Host(b) => {
                if let Some(host) = b.get_env_float(name) {
                    Ok(sync_f64_f32(&host))
                } else if let Some(host) = b.get_env_int(name) {
                    Ok(sync_i64_f32(&host))
                } else {
                    Err(SeqError::new(
                        file,
                        line,
                        1,
                        format!("unknown env.{name} or wrong type (here you need: Float/Int)"),
                    ))
                }
            }
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z wiersza wymaga hosta"),
            )),
            Resolver::Host(_) => match row_value(file, line, row, field)? {
                RowVal::Sig(FieldSignal::Float(host)) => Ok(sync_f64_f32(&host)),
                RowVal::Sig(FieldSignal::Int(host)) => Ok(sync_i64_f32(&host)),
                _ => Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("pole wiersza '{field}' to nie liczba (Float/Int)"),
                )),
            },
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to liczba albo nazwa propertu"),
        )),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn sync_f64_f32(host: &Signal<f64>) -> Signal<f32> {
    let local = Signal::new(host.get_untracked() as f32);
    let (h2, l2) = (host.clone(), local.clone());
    effect(move || l2.set(h2.get() as f32)).forget();
    let (h3, l3) = (host.clone(), local.clone());
    effect(move || h3.set(l3.get() as f64)).forget();
    local
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn sync_i64_f32(host: &Signal<i64>) -> Signal<f32> {
    let local = Signal::new(host.get_untracked() as f32);
    let (h2, l2) = (host.clone(), local.clone());
    effect(move || l2.set(h2.get() as f32)).forget();
    let (h3, l3) = (host.clone(), local.clone());
    effect(move || h3.set(l3.get() as i64)).forget();
    local
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn live_f32(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<Signal<f32>, SeqError> {
    match value {
        SeqValue::Num(n) => Ok(Signal::new(*n)),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z propertu wymaga hosta"),
            )),
            Resolver::Host(b) => {
                if let Some(host) = b.get_float(prop) {
                    let local = Signal::new(host.get_untracked() as f32);
                    let l2 = local.clone();
                    effect(move || l2.set(host.get() as f32)).forget();
                    Ok(local)
                } else if let Some(host) = b.get_int(prop) {
                    let local = Signal::new(host.get_untracked() as f32);
                    let l2 = local.clone();
                    effect(move || l2.set(host.get() as f32)).forget();
                    Ok(local)
                } else if let Some(m) = b.get_memo_float(prop) {
                    let local = Signal::new(m.get_untracked() as f32);
                    let l2 = local.clone();
                    effect(move || l2.set(m.get() as f32)).forget();
                    Ok(local)
                } else if let Some(m) = b.get_memo_int(prop) {
                    let local = Signal::new(m.get_untracked() as f32);
                    let l2 = local.clone();
                    effect(move || l2.set(m.get() as f32)).forget();
                    Ok(local)
                } else {
                    Err(prop_err(file, line, prop, "Float/Int"))
                }
            }
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' from environment requires host"),
            )),
            Resolver::Host(b) => {
                if let Some(host) = b.get_env_float(name) {
                    let local = Signal::new(host.get_untracked() as f32);
                    let l2 = local.clone();
                    effect(move || l2.set(host.get() as f32)).forget();
                    Ok(local)
                } else if let Some(host) = b.get_env_int(name) {
                    let local = Signal::new(host.get_untracked() as f32);
                    let l2 = local.clone();
                    effect(move || l2.set(host.get() as f32)).forget();
                    Ok(local)
                } else {
                    Err(SeqError::new(
                        file,
                        line,
                        1,
                        format!("unknown env.{name} or wrong type (here you need: Float/Int)"),
                    ))
                }
            }
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z wiersza wymaga hosta"),
            )),
            Resolver::Host(_) => match row_value(file, line, row, field)? {
                RowVal::Sig(FieldSignal::Float(host)) => {
                    let local = Signal::new(host.get_untracked() as f32);
                    let l2 = local.clone();
                    effect(move || l2.set(host.get() as f32)).forget();
                    Ok(local)
                }
                RowVal::Sig(FieldSignal::Int(host)) => {
                    let local = Signal::new(host.get_untracked() as f32);
                    let l2 = local.clone();
                    effect(move || l2.set(host.get() as f32)).forget();
                    Ok(local)
                }
                _ => Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("pole wiersza '{field}' to nie liczba (Float/Int)"),
                )),
            },
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to liczba albo nazwa propertu"),
        )),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn bridge_usize(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<Signal<usize>, SeqError> {
    match value {
        SeqValue::Num(n) => {
            if n.is_finite() && *n >= 0.0 {
                Ok(Signal::new(*n as usize))
            } else {
                Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("'{what}' to liczba >= 0"),
                ))
            }
        }
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z propertu wymaga hosta"),
            )),
            Resolver::Host(b) => {
                if let Some(host) = b.get_int(prop) {
                    let local = Signal::new(host.get_untracked().max(0) as usize);
                    let (h2, l2) = (host.clone(), local.clone());
                    effect(move || l2.set(h2.get().max(0) as usize)).forget();
                    let (h3, l3) = (host.clone(), local.clone());
                    effect(move || h3.set(l3.get() as i64)).forget();
                    Ok(local)
                } else if b.get_memo_int(prop).is_some() {
                    Err(SeqError::new(
                        file,
                        line,
                        1,
                        format!("'{what}' z memo jest tylko do odczytu — podepnij prop Int"),
                    ))
                } else {
                    Err(prop_err(file, line, prop, "Int"))
                }
            }
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' from environment requires host"),
            )),
            Resolver::Host(b) => {
                if let Some(host) = b.get_env_int(name) {
                    let local = Signal::new(host.get_untracked().max(0) as usize);
                    let (h2, l2) = (host.clone(), local.clone());
                    effect(move || l2.set(h2.get().max(0) as usize)).forget();
                    let (h3, l3) = (host.clone(), local.clone());
                    effect(move || h3.set(l3.get() as i64)).forget();
                    Ok(local)
                } else {
                    Err(SeqError::new(
                        file,
                        line,
                        1,
                        format!("unknown env.{name} or wrong type (here you need: Int)"),
                    ))
                }
            }
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z wiersza wymaga hosta"),
            )),
            Resolver::Host(_) => match row_value(file, line, row, field)? {
                RowVal::Sig(FieldSignal::Int(host)) => {
                    let local = Signal::new(host.get_untracked().max(0) as usize);
                    let (h2, l2) = (host.clone(), local.clone());
                    effect(move || l2.set(h2.get().max(0) as usize)).forget();
                    let (h3, l3) = (host.clone(), local.clone());
                    effect(move || h3.set(l3.get() as i64)).forget();
                    Ok(local)
                }
                _ => Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("pole wiersza '{field}' to nie Int"),
                )),
            },
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to liczba albo nazwa propertu"),
        )),
    }
}

#[derive(Clone)]
enum LiveVal {
    Bool(Signal<bool>),
    Str(Signal<String>),
    Usize(Signal<usize>),
    F32(Signal<f32>),
}

impl LiveVal {
    #[allow(clippy::cast_possible_truncation)]
    fn current(&self) -> Value {
        match self {
            Self::Bool(s) => Value::Bool(s.get_untracked()),
            Self::Str(s) => Value::Str(Rc::new(s.get_untracked())),
            Self::Usize(s) => Value::Int(s.get_untracked() as i64),
            Self::F32(s) => Value::Float(f64::from(s.get_untracked())),
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn wire_live_action(
    file: &str,
    line: usize,
    b: &HostBindings,
    value: &SeqValue,
    live: &LiveVal,
    what: &str,
) -> Result<Rc<dyn Fn()>, SeqError> {
    if let SeqValue::CallSeq { name, args } = value {
        if args.is_empty() {
            return seq_fire(file, line, b, name, Vec::new());
        }
        if args.len() == 1 && matches!(&args[0], SeqValue::Ident(n) if n == "value") {
            return seq_fire_live(file, line, b, name, live);
        }
        return Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to {{ fn() }} albo {{ fn(value) }}"),
        ));
    }
    let SeqValue::Callback { action, args } = value else {
        return Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}': {{ akcja() }}"),
        ));
    };
    if args.is_empty() {
        return b.get_action0(action).ok_or_else(|| {
            SeqError::new(
                file,
                line,
                1,
                format!("unknown callback '{action}' (missing on host)"),
            )
        });
    }
    if args.len() == 1 && matches!(&args[0], SeqValue::Ident(name) if name == "value") {
        return match live {
            LiveVal::Bool(sig) => {
                let f = b.get_action_bool(action).ok_or_else(|| {
                    SeqError::new(
                        file,
                        line,
                        1,
                        format!("callback '{action}' called from Bool and the host does not have it (on_bool)"),
                    )
                })?;
                let sig = sig.clone();
                Ok(Rc::new(move || f(sig.get())))
            }
            LiveVal::Str(sig) => {
                let f = b.get_action_str(action).ok_or_else(|| {
                    SeqError::new(
                        file,
                        line,
                        1,
                        format!("callback '{action}' called with text, but the host does not have it (on_str)"),
                    )
                })?;
                let sig = sig.clone();
                Ok(Rc::new(move || f(sig.get())))
            }
            LiveVal::Usize(sig) => {
                let f = b.get_action_int(action).ok_or_else(|| {
                    SeqError::new(
                        file,
                        line,
                        1,
                        format!(
                            "callback '{action}' called from Int and host doesn't have it (on_int)"
                        ),
                    )
                })?;
                let sig = sig.clone();
                Ok(Rc::new(move || f(sig.get_untracked() as i64)))
            }
            LiveVal::F32(sig) => {
                let f = b.get_action_float(action).ok_or_else(|| {
                    SeqError::new(
                        file,
                        line,
                        1,
                        format!("callback '{action}' called from Float and host does not have it (on_float)"),
                    )
                })?;
                let sig = sig.clone();
                Ok(Rc::new(move || f(f64::from(sig.get()))))
            }
        };
    }
    Err(SeqError::new(
        file,
        line,
        1,
        format!("'{what}' to {{ akcja() }} albo {{ akcja(value) }}"),
    ))
}

fn display_of(
    file: &str,
    line: usize,
    b: &HostBindings,
    name: &str,
) -> Result<Rc<dyn Fn() -> String>, SeqError> {
    if let Some(sig) = b.get_str(name) {
        Ok(Rc::new(move || sig.get()))
    } else if let Some(m) = b.get_memo_str(name) {
        Ok(Rc::new(move || m.get()))
    } else if let Some(sig) = b.get_bool(name) {
        Ok(Rc::new(move || sig.get().to_string()))
    } else if let Some(m) = b.get_memo_bool(name) {
        Ok(Rc::new(move || m.get().to_string()))
    } else if let Some(sig) = b.get_int(name) {
        Ok(Rc::new(move || sig.get().to_string()))
    } else if let Some(m) = b.get_memo_int(name) {
        Ok(Rc::new(move || m.get().to_string()))
    } else if let Some(sig) = b.get_float(name) {
        Ok(Rc::new(move || sig.get().to_string()))
    } else if let Some(m) = b.get_memo_float(name) {
        Ok(Rc::new(move || m.get().to_string()))
    } else if let Some(sig) = b.get_env_str(name) {
        Ok(Rc::new(move || sig.get()))
    } else if let Some(sig) = b.get_env_bool(name) {
        Ok(Rc::new(move || sig.get().to_string()))
    } else if let Some(sig) = b.get_env_int(name) {
        Ok(Rc::new(move || sig.get().to_string()))
    } else if let Some(sig) = b.get_env_float(name) {
        Ok(Rc::new(move || sig.get().to_string()))
    } else {
        Err(SeqError::new(
            file,
            line,
            1,
            format!("unknown prop '{name}' (missing on host)"),
        ))
    }
}

fn display_env_of(
    file: &str,
    line: usize,
    b: &HostBindings,
    name: &str,
) -> Result<Rc<dyn Fn() -> String>, SeqError> {
    if let Some(sig) = b.get_env_str(name) {
        Ok(Rc::new(move || sig.get()))
    } else if let Some(sig) = b.get_env_bool(name) {
        Ok(Rc::new(move || sig.get().to_string()))
    } else if let Some(sig) = b.get_env_int(name) {
        Ok(Rc::new(move || sig.get().to_string()))
    } else if let Some(sig) = b.get_env_float(name) {
        Ok(Rc::new(move || sig.get().to_string()))
    } else {
        Err(SeqError::new(
            file,
            line,
            1,
            format!("unknown env.{name} (missing on host: HostBindings::env_..)"),
        ))
    }
}

fn display_field(
    file: &str,
    line: usize,
    row: Option<&SeqRowData>,
    field: &str,
) -> Result<Rc<dyn Fn() -> String>, SeqError> {
    match row_value(file, line, row, field)? {
        RowVal::Sig(FieldSignal::Str(sig)) => Ok(Rc::new(move || sig.get())),
        RowVal::Sig(FieldSignal::Bool(sig)) => Ok(Rc::new(move || sig.get().to_string())),
        RowVal::Sig(FieldSignal::Int(sig)) => Ok(Rc::new(move || sig.get().to_string())),
        RowVal::Sig(FieldSignal::Float(sig)) => Ok(Rc::new(move || sig.get().to_string())),
        RowVal::Id(id) => Ok(Rc::new(move || id.to_string())),
    }
}

enum ArgVal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

impl ArgVal {
    fn to_value(&self) -> Value {
        match self {
            Self::Int(i) => Value::Int(*i),
            Self::Float(f) => Value::Float(*f),
            Self::Str(s) => Value::Str(Rc::new(s.clone())),
            Self::Bool(b) => Value::Bool(*b),
        }
    }
}

fn arg_val(
    file: &str,
    line: usize,
    arg: &SeqValue,
    row: Option<&SeqRowData>,
) -> Result<ArgVal, SeqError> {
    match arg {
        SeqValue::Num(n) => {
            if n.fract() == 0.0 {
                #[allow(clippy::cast_possible_truncation)]
                Ok(ArgVal::Int(*n as i64))
            } else {
                Ok(ArgVal::Float(f64::from(*n)))
            }
        }
        SeqValue::Str(s) => Ok(ArgVal::Str(s.clone())),
        SeqValue::Bool(x) => Ok(ArgVal::Bool(*x)),
        SeqValue::ItemField(field) => match row_value(file, line, row, field)? {
            RowVal::Id(id) => Ok(ArgVal::Int(id)),
            RowVal::Sig(FieldSignal::Int(sig)) => Ok(ArgVal::Int(sig.get_untracked())),
            RowVal::Sig(FieldSignal::Float(sig)) => Ok(ArgVal::Float(sig.get_untracked())),
            RowVal::Sig(FieldSignal::Str(sig)) => Ok(ArgVal::Str(sig.get_untracked())),
            RowVal::Sig(FieldSignal::Bool(sig)) => Ok(ArgVal::Bool(sig.get_untracked())),
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            "argument to liczba/tekst/Bool/value/item.pole".to_string(),
        )),
    }
}

fn seq_target(
    file: &str,
    line: usize,
    b: &HostBindings,
    name: &str,
) -> Result<(Rc<crate::eval::SeqRuntime>, HostBindings, String), SeqError> {
    let rt = b.get_seq().ok_or_else(|| {
        SeqError::new(
            file,
            line,
            1,
            format!("funkcja '{name}' wymaga hosta (lower_with z runtime .seq)"),
        )
    })?;
    if !rt.has_fn(name) {
        return Err(SeqError::new(
            file,
            line,
            1,
            format!("nieznana funkcja '{name}'"),
        ));
    }
    Ok((rt, b.clone(), name.to_string()))
}

fn seq_fire(
    file: &str,
    line: usize,
    b: &HostBindings,
    name: &str,
    const_args: Vec<Value>,
) -> Result<Rc<dyn Fn()>, SeqError> {
    let (rt, bb, n) = seq_target(file, line, b, name)?;
    Ok(Rc::new(move || {
        if let Err(e) = rt.call(&bb, &n, const_args.clone()) {
            eprintln!("seq: {e}");
        }
    }))
}

fn seq_fire_live(
    file: &str,
    line: usize,
    b: &HostBindings,
    name: &str,
    live: &LiveVal,
) -> Result<Rc<dyn Fn()>, SeqError> {
    let (rt, bb, n) = seq_target(file, line, b, name)?;
    let live = live.clone();
    Ok(Rc::new(move || {
        let v = live.current();
        if let Err(e) = rt.call(&bb, &n, vec![v]) {
            eprintln!("seq: {e}");
        }
    }))
}

fn seq_fire_pt(
    file: &str,
    line: usize,
    b: &HostBindings,
    name: &str,
) -> Result<Rc<dyn Fn(f64, f64)>, SeqError> {
    let (rt, bb, n) = seq_target(file, line, b, name)?;
    Ok(Rc::new(move |x, y| {
        if let Err(e) = rt.call(&bb, &n, vec![Value::Float(x), Value::Float(y)]) {
            eprintln!("seq: {e}");
        }
    }))
}

fn seq_fire_name(
    file: &str,
    line: usize,
    b: &HostBindings,
    name: &str,
) -> Result<Rc<dyn Fn(f64, f64, String)>, SeqError> {
    let (rt, bb, n) = seq_target(file, line, b, name)?;
    Ok(Rc::new(move |_x, _y, hit: String| {
        if let Err(e) = rt.call(&bb, &n, vec![Value::Str(Rc::new(hit))]) {
            eprintln!("seq: {e}");
        }
    }))
}

fn seq_fire_pt_named(
    file: &str,
    line: usize,
    b: &HostBindings,
    name: &str,
) -> Result<Rc<dyn Fn(f64, f64, String)>, SeqError> {
    let (rt, bb, n) = seq_target(file, line, b, name)?;
    Ok(Rc::new(move |x, y, hit: String| {
        if let Err(e) = rt.call(
            &bb,
            &n,
            vec![Value::Float(x), Value::Float(y), Value::Str(Rc::new(hit))],
        ) {
            eprintln!("seq: {e}");
        }
    }))
}

fn seq_fire_wheel(
    file: &str,
    line: usize,
    b: &HostBindings,
    name: &str,
) -> Result<crate::bindings::WheelAction, SeqError> {
    let (rt, bb, n) = seq_target(file, line, b, name)?;
    Ok(Rc::new(move |x, y, d| {
        if let Err(e) = rt.call(
            &bb,
            &n,
            vec![Value::Float(x), Value::Float(y), Value::Float(d)],
        ) {
            eprintln!("seq: {e}");
        }
    }))
}

fn wire_action(
    file: &str,
    line: usize,
    b: &HostBindings,
    value: &SeqValue,
    row: Option<&SeqRowData>,
) -> Result<Rc<dyn Fn()>, SeqError> {
    if let SeqValue::CallSeq { name, args } = value {
        if args.len() > 1 {
            return Err(SeqError::new(
                file,
                line,
                1,
                format!("function '{name}' with 2+ arguments - later"),
            ));
        }
        if args.is_empty() {
            return seq_fire(file, line, b, name, Vec::new());
        }
        let arg = arg_val(file, line, &args[0], row)?;
        return seq_fire(file, line, b, name, vec![arg.to_value()]);
    }
    let SeqValue::Callback { action, args } = value else {
        return Err(SeqError::new(file, line, 1, "{ akcja(args) }"));
    };
    if args.len() > 1 {
        return Err(SeqError::new(
            file,
            line,
            1,
            format!("callback '{action}' with 2+ arguments - later"),
        ));
    }
    if args.is_empty() {
        let f = b.get_action0(action).ok_or_else(|| {
            SeqError::new(
                file,
                line,
                1,
                format!("unknown callback '{action}' (missing on host)"),
            )
        })?;
        return Ok(Rc::new(move || f()));
    }
    let arg = arg_val(file, line, &args[0], row)?;
    match arg {
        ArgVal::Int(i) => {
            if let Some(f) = b.get_action_int(action) {
                return Ok(Rc::new(move || f(i)));
            }

            #[allow(clippy::cast_precision_loss)]
            let x = i as f64;
            if let Some(f) = b.get_action_float(action) {
                return Ok(Rc::new(move || f(x)));
            }
            Err(SeqError::new(
                file,
                line,
                1,
                format!(
                    "callback '{action}' called from Int and host doesn't have it (on_int/on_float)"
                ),
            ))
        }
        ArgVal::Float(x) => {
            let f = b.get_action_float(action).ok_or_else(|| {
                SeqError::new(
                    file,
                    line,
                    1,
                    format!(
                        "callback '{action}' called from Float and host does not have it (on_float)"
                    ),
                )
            })?;
            Ok(Rc::new(move || f(x)))
        }
        ArgVal::Str(s) => {
            let f = b.get_action_str(action).ok_or_else(|| {
                SeqError::new(
                    file,
                    line,
                    1,
                    format!("callback '{action}' called with text, but the host does not have it (on_str)"),
                )
            })?;
            Ok(Rc::new(move || f(s.clone())))
        }
        ArgVal::Bool(x) => {
            let f = b.get_action_bool(action).ok_or_else(|| {
                SeqError::new(
                    file,
                    line,
                    1,
                    format!("callback '{action}' called from Bool and the host does not have it (on_bool)"),
                )
            })?;
            Ok(Rc::new(move || f(x)))
        }
    }
}

fn lower_list(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
) -> Result<AnyView, SeqError> {
    let items = find_arg(node, "items").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "List wymaga items: nazwa_propertu".to_string(),
        )
    })?;
    let items_prop = match &items.value {
        SeqValue::Ident(name) => name.clone(),
        _ => {
            return Err(SeqError::new(
                file,
                items.line,
                1,
                "items: to nazwa propertu z hosta (items: warstwy)",
            ));
        }
    };
    if let Some(key) = find_arg(node, "key") {
        match &key.value {
            SeqValue::Ident(name) if name == "id" => {}
            _ => {
                return Err(SeqError::new(
                    file,
                    key.line,
                    1,
                    "key: tylko id w inkrementcie 3",
                ));
            }
        }
    }
    let bindings = match res {
        Resolver::Static => {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                "List(items:) wymaga hosta Rust (HostBindings::rows())",
            ));
        }
        Resolver::Host(b) => (*b).clone(),
    };
    let get_rows = bindings.get_rows(&items_prop).ok_or_else(|| {
        SeqError::new(
            file,
            items.line,
            1,
            format!("nieznany prop '{items_prop}' (HostBindings::rows())"),
        )
    })?;
    let template = node.children.clone();
    let file_owned = file.to_string();
    let theme_owned = *theme;
    let styles_owned = styles.clone();
    let list = sequoia_view::List::new(move || get_rows())
        .key(|row: &SeqRowData| {
            #[allow(clippy::cast_sign_loss)]
            let _ = row.id as u64;
            row.id as u64
        })
        .item(move |row: &SeqRowData| {
            let res = Resolver::Host(&bindings);
            match lower_row_template(
                &file_owned,
                &theme_owned,
                &styles_owned,
                &res,
                row,
                &template,
            ) {
                Ok(v) => v,
                Err(e) => AnyView::new(Text::new(format!("seq: {e}"))),
            }
        });
    Ok(AnyView::new(list))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn lower_virtual_list(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let items = find_arg(node, "items").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "VirtualList wymaga items: nazwa_propertu".to_string(),
        )
    })?;
    let items_prop = match &items.value {
        SeqValue::Ident(name) => name.clone(),
        _ => {
            return Err(SeqError::new(
                file,
                items.line,
                1,
                "items: to nazwa propertu z hosta (items: warstwy)",
            ));
        }
    };
    if let Some(key) = find_arg(node, "key") {
        match &key.value {
            SeqValue::Ident(name) if name == "id" => {}
            _ => {
                return Err(SeqError::new(
                    file,
                    key.line,
                    1,
                    "key: tylko id w inkrementcie 3",
                ));
            }
        }
    }
    let mut item_height: Option<f32> = None;
    if let Some(arg) = find_arg(node, "item_height") {
        let h = num(file, arg, res, row)?;
        if !h.is_finite() || h <= 0.0 {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                "'item_height' to dodatnia liczba",
            ));
        }
        item_height = Some(h);
    }

    let mut overscan: Option<usize> = None;
    if let Some(arg) = find_arg(node, "overscan") {
        let n = num(file, arg, res, row)?;
        if !n.is_finite() || n < 0.0 {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                "'overscan' to liczba >= 0",
            ));
        }
        overscan = Some(n as usize);
    }
    let bindings = match res {
        Resolver::Static => {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                "VirtualList(items:) wymaga hosta Rust (HostBindings::rows())",
            ));
        }
        Resolver::Host(b) => (*b).clone(),
    };
    let get_rows = bindings.get_rows(&items_prop).ok_or_else(|| {
        SeqError::new(
            file,
            items.line,
            1,
            format!("nieznany prop '{items_prop}' (HostBindings::rows())"),
        )
    })?;
    let template = node.children.clone();
    let file_owned = file.to_string();
    let theme_owned = *theme;
    let styles_owned = styles.clone();
    let mut list = VirtualList::new(move || get_rows())
        .key(|row: &SeqRowData| {
            #[allow(clippy::cast_sign_loss)]
            let _ = row.id as u64;
            row.id as u64
        })
        .item(move |row: &SeqRowData| {
            let res = Resolver::Host(&bindings);
            match lower_row_template(
                &file_owned,
                &theme_owned,
                &styles_owned,
                &res,
                row,
                &template,
            ) {
                Ok(v) => v,
                Err(e) => AnyView::new(Text::new(format!("seq: {e}"))),
            }
        });
    if let Some(h) = item_height {
        list = list.item_height(h);
    }
    if let Some(n) = overscan {
        list = list.overscan(n);
    }
    Ok(AnyView::new(list))
}

fn lower_row_template(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    res: &Resolver<'_>,
    row: &SeqRowData,
    template: &[SeqNode],
) -> Result<AnyView, SeqError> {
    match template {
        [] => Ok(AnyView::new(Column::new())),
        [single] => lower_node(file, theme, styles, single, res, Some(row)),
        _ => {
            let mut col = Column::new();
            for n in template {
                col = col.child(lower_node(file, theme, styles, n, res, Some(row))?);
            }
            Ok(AnyView::new(col))
        }
    }
}

#[derive(Clone)]
enum NumSrc {
    Lit(f32),
    Dyn(Rc<dyn Fn() -> f32>),
}

impl NumSrc {
    fn val(&self) -> f32 {
        match self {
            Self::Lit(v) => *v,
            Self::Dyn(f) => f(),
        }
    }

    fn touch(&self) {
        if let Self::Dyn(f) = self {
            f();
        }
    }

    fn touch_rc(&self) -> Rc<dyn Fn()> {
        let this = self.clone();
        Rc::new(move || this.touch())
    }
}

#[derive(Clone)]
enum BoolSrc {
    Lit(bool),
    Dyn(Rc<dyn Fn() -> bool>),
}

impl BoolSrc {
    fn val(&self) -> bool {
        match self {
            Self::Lit(v) => *v,
            Self::Dyn(f) => f(),
        }
    }

    fn touch(&self) {
        if let Self::Dyn(f) = self {
            f();
        }
    }

    fn touch_rc(&self) -> Rc<dyn Fn()> {
        let this = self.clone();
        Rc::new(move || this.touch())
    }
}

#[derive(Clone)]
struct PtSrc {
    x: NumSrc,
    y: NumSrc,
}

impl PtSrc {
    fn val(&self) -> Point {
        Point::new(self.x.val(), self.y.val())
    }

    fn touch(&self) {
        self.x.touch();
        self.y.touch();
    }

    fn touch_rc(&self) -> Rc<dyn Fn()> {
        let this = self.clone();
        Rc::new(move || this.touch())
    }
}

#[derive(Clone)]
struct RectSrc {
    x: NumSrc,
    y: NumSrc,
    w: NumSrc,
    h: NumSrc,
}

impl RectSrc {
    fn val(&self) -> Rect {
        Rect::from_xywh(self.x.val(), self.y.val(), self.w.val(), self.h.val())
    }

    fn touch(&self) {
        self.x.touch();
        self.y.touch();
        self.w.touch();
        self.h.touch();
    }

    fn touch_rc(&self) -> Rc<dyn Fn()> {
        let this = self.clone();
        Rc::new(move || this.touch())
    }
}

#[derive(Clone)]
struct ShapeDraw {
    draw: Rc<dyn Fn(&mut Painter)>,
    touch: Rc<dyn Fn()>,
    hits: Vec<HitTest>,
}

type HitTest = (String, Rc<dyn Fn(Point) -> bool>);

fn dist_seg(p: Point, a: Point, b: Point) -> f32 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let len2 = abx * abx + aby * aby;
    if len2 <= f32::EPSILON {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * abx + (p.y - a.y) * aby) / len2).clamp(0.0, 1.0);
    let dx = p.x - (a.x + abx * t);
    let dy = p.y - (a.y + aby * t);
    (dx * dx + dy * dy).sqrt()
}

fn point_in_poly(p: Point, pts: &[Point]) -> bool {
    let mut inside = false;
    let n = pts.len();
    let mut j = n.saturating_sub(1);
    for i in 0..n {
        let a = pts[i];
        let b = pts[j];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn cubic(p0: Point, c0: Point, c1: Point, p1: Point, t: f32) -> Point {
    let u = 1.0 - t;
    Point::new(
        u * u * u * p0.x + 3.0 * u * u * t * c0.x + 3.0 * u * t * t * c1.x + t * t * t * p1.x,
        u * u * u * p0.y + 3.0 * u * u * t * c0.y + 3.0 * u * t * t * c1.y + t * t * t * p1.y,
    )
}

fn dyn_f32(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<NumSrc, SeqError> {
    match value {
        SeqValue::Num(n) => {
            if n.is_finite() {
                Ok(NumSrc::Lit(*n))
            } else {
                Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("'{what}' must be finished"),
                ))
            }
        }
        SeqValue::Ident(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z propertu wymaga hosta — inkrement 2"),
            )),
            Resolver::Host(b) => {
                if let Some(cell) = b.get_tick_angle(prop) {
                    return Ok(NumSrc::Dyn(Rc::new(move || cell.get())));
                }
                if let Some(sig) = b.get_float(prop) {
                    Ok(NumSrc::Dyn(Rc::new(move || sig.get() as f32)))
                } else if let Some(sig) = b.get_int(prop) {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                    let _ = ();
                    Ok(NumSrc::Dyn(Rc::new(move || sig.get() as f32)))
                } else if let Some(m) = b.get_memo_float(prop) {
                    Ok(NumSrc::Dyn(Rc::new(move || m.get() as f32)))
                } else if let Some(m) = b.get_memo_int(prop) {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                    let _ = ();
                    Ok(NumSrc::Dyn(Rc::new(move || m.get() as f32)))
                } else {
                    Err(prop_err(file, line, prop, "Float/Int"))
                }
            }
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z wiersza wymaga hosta"),
            )),
            Resolver::Host(_) => match row_value(file, line, row, field)? {
                RowVal::Sig(FieldSignal::Int(sig)) => {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                    let _ = ();
                    Ok(NumSrc::Dyn(Rc::new(move || sig.get() as f32)))
                }
                RowVal::Sig(FieldSignal::Float(sig)) => {
                    Ok(NumSrc::Dyn(Rc::new(move || sig.get() as f32)))
                }
                RowVal::Id(id) => {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                    let _ = ();
                    Ok(NumSrc::Lit(id as f32))
                }
                _ => Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("pole wiersza '{field}' to nie liczba (Int/Float/id)"),
                )),
            },
        },
        SeqValue::Dynamic(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("dynamic({name}) wymaga hosta Rust"),
            )),
            Resolver::Host(b) => {
                if let Some(cell) = b.get_tick_angle(name) {
                    return Ok(NumSrc::Dyn(Rc::new(move || cell.get())));
                }
                let sig = b.get_float(name).or_else(|| None);
                match sig {
                    Some(sig) => Ok(NumSrc::Dyn(Rc::new(move || sig.get() as f32))),
                    None => match b.get_int(name) {
                        Some(sig) => {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                            let _ = ();
                            Ok(NumSrc::Dyn(Rc::new(move || sig.get() as f32)))
                        }
                        None => match b.get_memo_float(name) {
                            Some(m) => Ok(NumSrc::Dyn(Rc::new(move || m.get() as f32))),
                            None => match b.get_memo_int(name) {
                                Some(m) => {
                                    #[allow(
                                        clippy::cast_possible_truncation,
                                        clippy::cast_precision_loss
                                    )]
                                    let _ = ();
                                    Ok(NumSrc::Dyn(Rc::new(move || m.get() as f32)))
                                }
                                None => Err(prop_err(file, line, name, "Float/Int")),
                            },
                        },
                    },
                }
            }
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("env.{name} wymaga hosta Rust"),
            )),
            Resolver::Host(b) => {
                if let Some(sig) = b.get_env_float(name) {
                    Ok(NumSrc::Dyn(Rc::new(move || sig.get() as f32)))
                } else if let Some(sig) = b.get_env_int(name) {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                    let _ = ();
                    Ok(NumSrc::Dyn(Rc::new(move || sig.get() as f32)))
                } else {
                    Err(SeqError::new(
                        file,
                        line,
                        1,
                        format!("unknown env.{name} or wrong type (here you need: Float/Int)"),
                    ))
                }
            }
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to liczba albo dynamic(nazwa)"),
        )),
    }
}

fn dyn_bool(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<BoolSrc, SeqError> {
    match value {
        SeqValue::Bool(b) => Ok(BoolSrc::Lit(*b)),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z propertu wymaga hosta"),
            )),
            Resolver::Host(b) => b
                .get_bool(prop)
                .map(|sig| BoolSrc::Dyn(Rc::new(move || sig.get())))
                .or_else(|| {
                    b.get_memo_bool(prop)
                        .map(|m| BoolSrc::Dyn(Rc::new(move || m.get())))
                })
                .ok_or_else(|| prop_err(file, line, prop, "Bool")),
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' from environment requires host"),
            )),
            Resolver::Host(b) => b
                .get_env_bool(name)
                .map(|sig| BoolSrc::Dyn(Rc::new(move || sig.get())))
                .ok_or_else(|| {
                    SeqError::new(
                        file,
                        line,
                        1,
                        format!("unknown env.{name} or wrong type (here you need: Bool)"),
                    )
                }),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{what}' z wiersza wymaga hosta"),
            )),
            Resolver::Host(_) => match row_value(file, line, row, field)? {
                RowVal::Sig(FieldSignal::Bool(sig)) => Ok(BoolSrc::Dyn(Rc::new(move || sig.get()))),
                _ => Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("pole wiersza '{field}' to nie Bool"),
                )),
            },
        },
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to true/false albo dynamic(nazwa)"),
        )),
    }
}

fn pt_src(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<PtSrc, SeqError> {
    match value {
        SeqValue::Tuple(items) if items.len() == 2 => Ok(PtSrc {
            x: dyn_f32(file, line, res, row, &items[0], what)?,
            y: dyn_f32(file, line, res, row, &items[1], what)?,
        }),
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to krotka (x, y)"),
        )),
    }
}

fn rect_src(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<RectSrc, SeqError> {
    match value {
        SeqValue::Tuple(items) if items.len() == 4 => Ok(RectSrc {
            x: dyn_f32(file, line, res, row, &items[0], what)?,
            y: dyn_f32(file, line, res, row, &items[1], what)?,
            w: dyn_f32(file, line, res, row, &items[2], what)?,
            h: dyn_f32(file, line, res, row, &items[3], what)?,
        }),
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to krotka (x, y, w, h)"),
        )),
    }
}

fn points_src(
    file: &str,
    line: usize,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    value: &SeqValue,
    what: &str,
) -> Result<Vec<PtSrc>, SeqError> {
    match value {
        SeqValue::Array(items) if !items.is_empty() => items
            .iter()
            .map(|it| pt_src(file, line, res, row, it, what))
            .collect(),
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{what}' to [(x, y), ...] (min. 1 punkt)"),
        )),
    }
}

fn uv_src_of(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
) -> Result<[NumSrc; 4], SeqError> {
    match find_arg(node, "uv") {
        None => Ok([
            NumSrc::Lit(0.0),
            NumSrc::Lit(0.0),
            NumSrc::Lit(1.0),
            NumSrc::Lit(1.0),
        ]),
        Some(arg) => match &arg.value {
            SeqValue::Tuple(items) if items.len() == 4 => Ok([
                dyn_f32(file, arg.line, res, row, &items[0], "uv")?,
                dyn_f32(file, arg.line, res, row, &items[1], "uv")?,
                dyn_f32(file, arg.line, res, row, &items[2], "uv")?,
                dyn_f32(file, arg.line, res, row, &items[3], "uv")?,
            ]),
            _ => Err(SeqError::new(
                file,
                arg.line,
                1,
                "'uv' to krotka (u0, v0, u1, v1)",
            )),
        },
    }
}

fn shape_color(file: &str, theme: &Theme, node: &SeqNode) -> Result<Color, SeqError> {
    for name in ["fill", "color"] {
        if let Some(arg) = find_arg(node, name) {
            return color(file, theme, arg);
        }
    }
    Err(SeqError::new(
        file,
        node.line,
        1,
        format!("{} wymaga fill: albo color:", node.kind),
    ))
}

fn shape_visible(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
) -> Result<BoolSrc, SeqError> {
    match find_arg(node, "visible") {
        None => Ok(BoolSrc::Lit(true)),
        Some(arg) => dyn_bool(file, arg.line, res, row, &arg.value, "visible"),
    }
}

fn shape_name(
    file: &str,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Option<String>, SeqError> {
    match find_arg(node, "name") {
        None => Ok(None),
        Some(arg) => match &arg.value {
            SeqValue::Str(s) => Ok(Some(s.clone())),
            SeqValue::ItemField(field) => match res {
                Resolver::Static => Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    "name: z wiersza wymaga hosta",
                )),
                Resolver::Host(_) => match row_value(file, arg.line, row, field)? {
                    RowVal::Id(id) => Ok(Some(id.to_string())),
                    RowVal::Sig(FieldSignal::Str(sig)) => Ok(Some(sig.get_untracked())),
                    RowVal::Sig(FieldSignal::Int(sig)) => Ok(Some(sig.get_untracked().to_string())),
                    RowVal::Sig(FieldSignal::Float(sig)) => {
                        Ok(Some(sig.get_untracked().to_string()))
                    }
                    RowVal::Sig(FieldSignal::Bool(sig)) => {
                        Ok(Some(sig.get_untracked().to_string()))
                    }
                },
            },
            _ => Err(SeqError::new(
                file,
                arg.line,
                1,
                "name: is a string in quotation marks or item.field",
            )),
        },
    }
}

fn shape_num(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
    name: &str,
) -> Result<NumSrc, SeqError> {
    match find_arg(node, name) {
        None => Err(SeqError::new(
            file,
            node.line,
            1,
            format!("{} wymaga {name}:", node.kind),
        )),
        Some(arg) => dyn_f32(file, arg.line, res, row, &arg.value, name),
    }
}

fn shape_pt(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
    name: &str,
) -> Result<PtSrc, SeqError> {
    match find_arg(node, name) {
        None => Err(SeqError::new(
            file,
            node.line,
            1,
            format!("{} wymaga {name}: (x, y)", node.kind),
        )),
        Some(arg) => pt_src(file, arg.line, res, row, &arg.value, name),
    }
}

fn rect_src_of(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
) -> Result<RectSrc, SeqError> {
    if let Some(arg) = find_arg(node, "rect") {
        return rect_src(file, arg.line, res, row, &arg.value, "rect");
    }
    Ok(RectSrc {
        x: shape_num(file, res, row, node, "x")?,
        y: shape_num(file, res, row, node, "y")?,
        w: shape_num(file, res, row, node, "w")?,
        h: shape_num(file, res, row, node, "h")?,
    })
}

fn opt_num(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
    name: &str,
) -> Result<NumSrc, SeqError> {
    match find_arg(node, name) {
        None => Ok(NumSrc::Lit(0.0)),
        Some(arg) => dyn_f32(file, arg.line, res, row, &arg.value, name),
    }
}

fn opt_pt(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
    name: &str,
) -> Result<Option<PtSrc>, SeqError> {
    match find_arg(node, name) {
        None => Ok(None),
        Some(arg) => pt_src(file, arg.line, res, row, &arg.value, name).map(Some),
    }
}

fn shape_str(file: &str, node: &SeqNode, name: &str) -> Result<String, SeqError> {
    match find_arg(node, name) {
        Some(arg) => match &arg.value {
            SeqValue::Str(s) => Ok(s.clone()),
            _ => Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("{} wymaga {name}: \"...\"", node.kind),
            )),
        },
        None => Err(SeqError::new(
            file,
            node.line,
            1,
            format!("{} wymaga {name}: \"...\"", node.kind),
        )),
    }
}

fn points_src_of(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
) -> Result<Vec<PtSrc>, SeqError> {
    match find_arg(node, "points") {
        None => Err(SeqError::new(
            file,
            node.line,
            1,
            format!("{} wymaga points: [(x, y), ...]", node.kind),
        )),
        Some(arg) => points_src(file, arg.line, res, row, &arg.value, "points"),
    }
}
fn touches(parts: &[Rc<dyn Fn()>]) -> Rc<dyn Fn()> {
    let parts = parts.to_vec();
    Rc::new(move || {
        for p in &parts {
            p();
        }
    })
}

trait Touch {
    fn touch_rc(&self) -> Rc<dyn Fn()>;
}

impl Touch for NumSrc {
    fn touch_rc(&self) -> Rc<dyn Fn()> {
        NumSrc::touch_rc(self)
    }
}

impl Touch for PtSrc {
    fn touch_rc(&self) -> Rc<dyn Fn()> {
        PtSrc::touch_rc(self)
    }
}

impl Touch for RectSrc {
    fn touch_rc(&self) -> Rc<dyn Fn()> {
        RectSrc::touch_rc(self)
    }
}

impl Touch for Vec<PtSrc> {
    fn touch_rc(&self) -> Rc<dyn Fn()> {
        let this = self.clone();
        Rc::new(move || {
            for p in &this {
                p.touch();
            }
        })
    }
}

impl Touch for Rc<dyn Fn()> {
    fn touch_rc(&self) -> Rc<dyn Fn()> {
        self.clone()
    }
}

fn touch2(a: &impl Touch, b: &impl Touch) -> Rc<dyn Fn()> {
    touches(&[a.touch_rc(), b.touch_rc()])
}

fn touch3(a: &impl Touch, b: &impl Touch, c: &impl Touch) -> Rc<dyn Fn()> {
    touches(&[a.touch_rc(), b.touch_rc(), c.touch_rc()])
}

fn touch4(a: &impl Touch, b: &impl Touch, c: &impl Touch, d: &impl Touch) -> Rc<dyn Fn()> {
    touches(&[a.touch_rc(), b.touch_rc(), c.touch_rc(), d.touch_rc()])
}

fn touch4e(a: &impl Touch, b: &impl Touch, c: &impl Touch, d: &impl Touch) -> Rc<dyn Fn()> {
    touch4(a, b, c, d)
}

#[allow(clippy::too_many_arguments)]
fn touch5(
    a: &impl Touch,
    b: &impl Touch,
    c: &impl Touch,
    d: &impl Touch,
    e: &impl Touch,
) -> Rc<dyn Fn()> {
    touches(&[
        a.touch_rc(),
        b.touch_rc(),
        c.touch_rc(),
        d.touch_rc(),
        e.touch_rc(),
    ])
}

fn touch_pts(pts: &Vec<PtSrc>, vis: &Rc<dyn Fn()>) -> Rc<dyn Fn()> {
    touches(&[pts.touch_rc(), vis.clone()])
}

fn touch_pts_w(pts: &Vec<PtSrc>, w: &NumSrc, vis: &Rc<dyn Fn()>) -> Rc<dyn Fn()> {
    touches(&[pts.touch_rc(), w.touch_rc(), vis.clone()])
}

fn only_color(file: &str, theme: &Theme, node: &SeqNode) -> Result<Color, SeqError> {
    shape_color(file, theme, node)
}

fn dist_polyline(p: Point, pts: &[Point]) -> f32 {
    if pts.is_empty() {
        return f32::INFINITY;
    }
    if pts.len() == 1 {
        let dx = p.x - pts[0].x;
        let dy = p.y - pts[0].y;
        return (dx * dx + dy * dy).sqrt();
    }
    pts.windows(2)
        .map(|w| dist_seg(p, w[0], w[1]))
        .fold(f32::INFINITY, f32::min)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn sample_cubic(pts: &[Point]) -> Vec<Point> {
    const N: u32 = 16;
    let (p0, c0, c1, p1) = (pts[0], pts[1], pts[2], pts[3]);
    (0..=N)
        .map(|i| cubic(p0, c0, c1, p1, i as f32 / N as f32))
        .collect()
}

#[derive(Clone)]
enum GestureWire {
    Null(Rc<dyn Fn()>),

    Pt(Rc<dyn Fn(f64, f64)>),

    PtNamed(Rc<dyn Fn(f64, f64, String)>),

    Wheel(crate::bindings::WheelAction),
}

fn fire_gesture(wire: &GestureWire, x: f64, y: f64, hit: Option<&str>) {
    match wire {
        GestureWire::Null(f) => f(),
        GestureWire::Pt(f) => f(x, y),
        GestureWire::PtNamed(f) => f(x, y, hit.unwrap_or("").to_string()),
        GestureWire::Wheel(f) => f(x, y, 0.0),
    }
}

fn wire_gesture(
    file: &str,
    line: usize,
    b: &HostBindings,
    value: &SeqValue,
) -> Result<GestureWire, SeqError> {
    let (action, args) = match value {
        SeqValue::Callback { action, args } => (action.as_str(), args.as_slice()),
        SeqValue::CallSeq { name, args } => {
            return match args.len() {
                0 => seq_fire(file, line, b, name, Vec::new()).map(GestureWire::Null),
                1 => seq_fire_name(file, line, b, name).map(GestureWire::PtNamed),
                2 => seq_fire_pt(file, line, b, name).map(GestureWire::Pt),
                3 => {
                    let third = match &args[2] {
                        SeqValue::Ident(n) => n.as_str(),
                        _ => {
                            return Err(SeqError::new(
                                file,
                                line,
                                1,
                                format!("'{name}': trzeci argument gestu to `name` albo `d`"),
                            ));
                        }
                    };
                    if third == "d" {
                        seq_fire_wheel(file, line, b, name).map(GestureWire::Wheel)
                    } else {
                        seq_fire_pt_named(file, line, b, name).map(GestureWire::PtNamed)
                    }
                }
                _ => Err(SeqError::new(
                    file,
                    line,
                    1,
                    format!("'{name}': gest to fn(), fn(name) albo fn(x, y[, name|d])"),
                )),
            };
        }
        _ => {
            return Err(SeqError::new(
                file,
                line,
                1,
                "gest: { akcja(x, y) } albo { fn(x, y) }",
            ));
        }
    };
    let ident_args = args
        .iter()
        .map(|a| match a {
            SeqValue::Ident(name) => Ok(name.clone()),
            _ => Err(SeqError::new(
                file,
                line,
                1,
                format!("'{action}': argumenty gestu to nazwy (x, y[, name])"),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    match ident_args.len() {
        0 => b
            .get_action0(action)
            .map(GestureWire::Null)
            .ok_or_else(|| unknown_gesture(file, line, action)),
        2 => b
            .get_action_point(action)
            .map(GestureWire::Pt)
            .ok_or_else(|| unknown_gesture(file, line, action)),
        3 => {
            if let Some(f) = b.get_action_point_named(action) {
                Ok(GestureWire::PtNamed(f))
            } else if let Some(f) = b.get_action_wheel(action) {
                Ok(GestureWire::Wheel(f))
            } else {
                Err(unknown_gesture(file, line, action))
            }
        }
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("'{action}': gest to 0, 2 albo 3 argumenty (x, y[, name|d])"),
        )),
    }
}

fn unknown_gesture(file: &str, line: usize, action: &str) -> SeqError {
    SeqError::new(
        file,
        line,
        1,
        format!(
            "unknown callback '{action}' (missing on host: on/on_point/on_point_named/on_wheel)"
        ),
    )
}

fn canvas_cursor(file: &str, line: usize, name: &str) -> Result<CursorIcon, SeqError> {
    match name {
        "default" => Ok(CursorIcon::Default),
        "pointer" => Ok(CursorIcon::Pointer),
        "crosshair" => Ok(CursorIcon::Crosshair),
        "grab" => Ok(CursorIcon::Grab),
        "grabbing" => Ok(CursorIcon::Grabbing),
        "text" => Ok(CursorIcon::Text),
        other => Err(SeqError::new(
            file,
            line,
            1,
            format!("cursor to default/pointer/crosshair/grab/grabbing/text, nie '{other}'"),
        )),
    }
}

fn lower_split(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let value_arg = find_arg(node, "value").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "SplitPanel wymaga value: prop".to_string(),
        )
    })?;
    let sig = bridge_f32(file, value_arg.line, res, row, &value_arg.value, "value")?;
    let mut v = SplitPanel::new(sig);
    for arg in &node.args {
        match arg.name.as_str() {
            "value" => {}
            "axis" => {
                v = match ident(file, arg)?.as_str() {
                    "horizontal" => v.horizontal(),
                    "vertical" => v.vertical(),
                    other => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            format!("axis to horizontal/vertical, nie '{other}'"),
                        ));
                    }
                };
            }
            "handle" => v = v.handle_width(num(file, arg, res, row)?),
            "min_first" => v = v.min_first(num(file, arg, res, row)?),
            "min_second" => v = v.min_second(num(file, arg, res, row)?),
            "handle_color" => v = v.handle_color(color(file, theme, arg)?),
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "padding" => match padding_val(file, arg, res, row)? {
                PaddingVal::All(n) => v = v.padding(n),
                PaddingVal::Xy(x, y) => v = v.padding_xy(x, y),
            },
            "padding_x" => v = v.padding_x(num(file, arg, res, row)?),
            "padding_y" => v = v.padding_y(num(file, arg, res, row)?),
            "background" => v = v.background(color(file, theme, arg)?),
            "flex" => v = v.flex(num(file, arg, res, row)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "margin" => match margin_val(file, arg, res, row)? {
                MarginVal::All(n) => v = v.margin(n),
                MarginVal::Xy(x, y) => v = v.margin_xy(x, y),
            },
            "margin_x" => v = v.margin_x(num(file, arg, res, row)?),
            "margin_y" => v = v.margin_y(num(file, arg, res, row)?),
            "margin_left" => v = v.margin_left(num(file, arg, res, row)?),
            "margin_right" => v = v.margin_right(num(file, arg, res, row)?),
            "margin_top" => v = v.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => v = v.margin_bottom(num(file, arg, res, row)?),
            "overflow" => v = v.overflow(overflow_val(file, arg)?),
            "hit" => v = v.hit_testable(flag(file, arg, res, row)?),
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }
    if node.children.len() != 2 {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            "SplitPanel requires exactly 2 children: first { } and second { }",
        ));
    }

    for c in &node.children {
        let _ = child_flex_weight(file, res, row, c)?;
    }
    let first = lower_node(file, theme, styles, &node.children[0], res, row)?;
    let second = lower_node(file, theme, styles, &node.children[1], res, row)?;
    v = v.first(first);
    v = v.second(second);
    Ok(AnyView::new(v))
}

const DRAG_THRESHOLD_SQ: f32 = 64.0;

#[allow(clippy::too_many_lines)]
fn lower_canvas(
    file: &str,
    theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    let mut v = Canvas::new();
    let mut click: Option<GestureWire> = None;
    let mut drag: Option<GestureWire> = None;
    let mut wheel: Option<crate::bindings::WheelAction> = None;
    let mut pinch: Option<crate::bindings::PinchAction> = None;
    let mut press: Option<GestureWire> = None;

    let tick_name: Option<String> = match find_arg(node, "tick") {
        None => {
            if find_arg(node, "running").is_some() || find_arg(node, "speed").is_some() {
                return Err(SeqError::new(
                    file,
                    node.line,
                    1,
                    "running:/speed: require tick: name".to_string(),
                ));
            }
            None
        }
        Some(arg) => match &arg.value {
            SeqValue::Ident(name) => Some(name.clone()),
            _ => {
                return Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    "tick: to nazwa (tick: kat)".to_string(),
                ));
            }
        },
    };
    let mut owned_bindings: Option<HostBindings> = None;
    let mut tick_state: Option<TickState> = None;
    if let Some(name) = tick_name {
        match res {
            Resolver::Static => {
                return Err(SeqError::new(
                    file,
                    node.line,
                    1,
                    "tick: wymaga hosta Rust".to_string(),
                ));
            }
            Resolver::Host(b) => {
                let angle = Rc::new(Cell::new(0.0f32));
                let mut cloned = (*b).clone();
                cloned = cloned.tick_angle(&name, angle.clone());
                let running = match find_arg(node, "running") {
                    None => BoolSrc::Lit(true),
                    Some(arg) => dyn_bool(file, arg.line, res, row, &arg.value, "running")?,
                };
                let speed = match find_arg(node, "speed") {
                    None => NumSrc::Lit(45.0),
                    Some(arg) => dyn_f32(file, arg.line, res, row, &arg.value, "speed")?,
                };
                owned_bindings = Some(cloned);
                tick_state = Some(TickState {
                    angle,
                    last: Rc::new(std::cell::RefCell::new(None)),
                    running,
                    speed,
                });
            }
        }
    }

    let tick_res;
    let res: &Resolver<'_> = match owned_bindings.as_ref() {
        Some(b) => {
            tick_res = Resolver::Host(b);
            &tick_res
        }
        None => res,
    };
    for arg in &node.args {
        match arg.name.as_str() {
            "width" => v = v.width(len(file, arg, res, row)?),
            "height" => v = v.height(len(file, arg, res, row)?),
            "flex" => v = v.flex(num(file, arg, res, row)?),
            "min_width" => v = v.min_width(num(file, arg, res, row)?),
            "max_width" => v = v.max_width(num(file, arg, res, row)?),
            "min_height" => v = v.min_height(num(file, arg, res, row)?),
            "max_height" => v = v.max_height(num(file, arg, res, row)?),
            "background" => v = v.background(color(file, theme, arg)?),
            "transparent" => {
                if flag(file, arg, res, row)? {
                    v = v.no_background();
                }
            }
            "cursor" => v = v.cursor(canvas_cursor(file, arg.line, &ident(file, arg)?)?),
            "focusable" => v = v.focusable(flag(file, arg, res, row)?),
            "grid" => v = v.grid(num(file, arg, res, row)?),
            "grid_color" => {
                let step = match find_arg(node, "grid") {
                    Some(g) => num(file, g, res, row)?,
                    None => {
                        return Err(SeqError::new(file, arg.line, 1, "grid_color wymaga grid:"));
                    }
                };
                v = v.grid_colored(step, color(file, theme, arg)?);
            }
            "axes" => {
                if flag(file, arg, res, row)? {
                    v = v.axes();
                }
            }
            "axes_color" => v = v.axes_colored(color(file, theme, arg)?),
            "clip" => v = v.clip(flag(file, arg, res, row)?),
            "allow_overflow" => v = v.allow_overflow(num(file, arg, res, row)?),
            "camera" => match &arg.value {
                SeqValue::Tuple(items) if items.len() == 2 => {
                    let pos = pt_src(file, arg.line, res, row, &items[0], "camera.pos")?;
                    let zoom = dyn_f32(file, arg.line, res, row, &items[1], "camera.zoom")?;

                    match (&pos.x, &pos.y, &zoom) {
                        (NumSrc::Lit(x), NumSrc::Lit(y), NumSrc::Lit(z)) => {
                            v = v.camera(Vec2 { x: *x, y: *y }, *z);
                        }
                        _ => {
                            return Err(SeqError::new(
                                file,
                                arg.line,
                                1,
                                "dynamic camera later (only numbers today)",
                            ));
                        }
                    }
                }
                _ => {
                    return Err(SeqError::new(file, arg.line, 1, "camera: ((x, y), zoom)"));
                }
            },
            "capture" => v = v.capture(flag(file, arg, res, row)?),

            "tick" | "speed" | "running" => {}
            "on_click_at" | "on_drag" | "on_wheel" | "on_press" | "on_pinch" => match res {
                Resolver::Static => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("{} wymaga hosta Rust", arg.name),
                    ));
                }
                Resolver::Host(b) => match &arg.value {
                    SeqValue::Callback { action, args } => {
                        if arg.name == "on_wheel" {
                            let idents = args
                                .iter()
                                .map(|a| match a {
                                    SeqValue::Ident(n) => Ok(n.clone()),
                                    _ => Err(SeqError::new(
                                        file,
                                        arg.line,
                                        1,
                                        "on_wheel: { akcja(x, y, d) }",
                                    )),
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            if idents.len() != 3 {
                                return Err(SeqError::new(
                                    file,
                                    arg.line,
                                    1,
                                    "on_wheel: { akcja(x, y, d) }",
                                ));
                            }
                            wheel = Some(b.get_action_wheel(action).ok_or_else(|| {
                                SeqError::new(
                                    file,
                                    arg.line,
                                    1,
                                    format!(
                                        "unknown callback '{action}' (missing on host: on_wheel)"
                                    ),
                                )
                            })?);
                        } else if arg.name == "on_pinch" {
                            let idents = args
                                .iter()
                                .map(|a| match a {
                                    SeqValue::Ident(n) => Ok(n.clone()),
                                    _ => Err(SeqError::new(
                                        file,
                                        arg.line,
                                        1,
                                        "on_pinch: { akcja(x, y, s) }",
                                    )),
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            if idents.len() != 3 {
                                return Err(SeqError::new(
                                    file,
                                    arg.line,
                                    1,
                                    "on_pinch: { akcja(x, y, s) }",
                                ));
                            }
                            pinch = Some(b.get_action_pinch(action).ok_or_else(|| {
                                SeqError::new(
                                    file,
                                    arg.line,
                                    1,
                                    format!(
                                        "unknown callback '{action}' (missing on host: on_pinch)"
                                    ),
                                )
                            })?);
                        } else {
                            let w = wire_gesture(file, arg.line, b, &arg.value)?;
                            if arg.name == "on_click_at" {
                                click = Some(w);
                            } else if arg.name == "on_press" {
                                press = Some(w);
                            } else {
                                drag = Some(w);
                            }
                        }
                    }
                    SeqValue::CallSeq { name, args } => {
                        if !args.iter().all(|a| matches!(a, SeqValue::Ident(_))) {
                            return Err(SeqError::new(
                                file,
                                arg.line,
                                1,
                                format!("{}: {{ fn(x, y[, name|d]) }}", arg.name),
                            ));
                        }
                        match arg.name.as_str() {
                            "on_wheel" => match args.len() {
                                0 => {
                                    let f = seq_fire(file, arg.line, b, name, Vec::new())?;
                                    wheel = Some(Rc::new(move |_x, _y, _d| f()));
                                }
                                3 => wheel = Some(seq_fire_wheel(file, arg.line, b, name)?),
                                _ => {
                                    return Err(SeqError::new(
                                        file,
                                        arg.line,
                                        1,
                                        format!("on_wheel: {{ {name}(x, y, d) }}"),
                                    ));
                                }
                            },
                            "on_pinch" => match args.len() {
                                0 => {
                                    let f = seq_fire(file, arg.line, b, name, Vec::new())?;
                                    pinch = Some(Rc::new(move |_x, _y, _s| f()));
                                }
                                3 => pinch = Some(seq_fire_wheel(file, arg.line, b, name)?),
                                _ => {
                                    return Err(SeqError::new(
                                        file,
                                        arg.line,
                                        1,
                                        format!("on_pinch: {{ {name}(x, y, s) }}"),
                                    ));
                                }
                            },
                            _ => {
                                let w = wire_gesture(file, arg.line, b, &arg.value)?;
                                if arg.name == "on_click_at" {
                                    click = Some(w);
                                } else if arg.name == "on_press" {
                                    press = Some(w);
                                } else {
                                    drag = Some(w);
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            format!("{}: {{ akcja(x, y[, name]) }}", arg.name),
                        ));
                    }
                },
            },
            "indent" | "indent_x" | "indent_y" | "indent_left" | "indent_right" | "indent_top"
            | "indent_bottom" => {}
            _ => unreachable_checked(file, node, arg)?,
        }
    }

    let mut layers: Vec<DrawLayer> = Vec::new();
    for c in &node.children {
        layers.push(lower_layer(file, theme, c, res, row)?);
    }
    let draw_layers = layers.clone();

    let track_tick = tick_state
        .as_ref()
        .map(|t| touches(&[t.running.touch_rc(), t.speed.touch_rc()]));
    v = v.on_draw(move |g: &mut Painter| {
        if let Some(t) = tick_state.as_ref() {
            let now = std::time::Instant::now();
            if t.running.val() {
                let prev = t.last.borrow_mut().replace(now);
                let dt = prev
                    .map(|p| now.duration_since(p).as_secs_f32())
                    .unwrap_or(0.0)
                    .clamp(0.0, MAX_ANIM_DT);
                t.angle.set((t.angle.get() + dt * t.speed.val()) % 360.0);
                g.ctx().request_frame();
            } else {
                *t.last.borrow_mut() = None;
            }
        }
        DrawLayer::draw_all(&draw_layers, g);
    });
    {
        let track_layers = layers.clone();
        v = v.track(move || {
            DrawLayer::touch_all(&track_layers);
            if let Some(touch) = track_tick.as_ref() {
                touch();
            }
        });
    }
    if click.is_some() || drag.is_some() || wheel.is_some() || press.is_some() || pinch.is_some() {
        let pick_layers = layers.clone();
        let down: Rc<Cell<Option<(f32, f32)>>> = Rc::new(Cell::new(None));
        let dragged = Rc::new(Cell::new(false));

        let down_name: Rc<std::cell::RefCell<Option<String>>> =
            Rc::new(std::cell::RefCell::new(None));
        v = v.on_pointer(move |at: Point, ev: &PointerEvent, _ctx: &mut EventCtx| {
            match ev {
                PointerEvent::Down {
                    button: PointerButton::Left,
                    pos,
                    ..
                } => {
                    down.set(Some((pos.x, pos.y)));
                    dragged.set(false);
                    let picked = DrawLayer::pick(&pick_layers, *pos);
                    *down_name.borrow_mut() = picked.clone();
                    if let Some(p) = press.as_ref() {
                        fire_gesture(p, f64::from(pos.x), f64::from(pos.y), picked.as_deref());
                    }
                }
                PointerEvent::Move { pos, .. } => {
                    if let Some((x0, y0)) = down.get() {
                        let dx = pos.x - x0;
                        let dy = pos.y - y0;
                        if dx * dx + dy * dy > DRAG_THRESHOLD_SQ {
                            dragged.set(true);
                            if let Some(d) = drag.as_ref() {
                                fire_gesture(
                                    d,
                                    f64::from(pos.x),
                                    f64::from(pos.y),
                                    down_name.borrow().as_deref(),
                                );
                            }
                        }
                    }
                }
                PointerEvent::Up {
                    button: PointerButton::Left,
                    pos,
                    ..
                } => {
                    let name = down_name.take();
                    if down.take().is_some()
                        && !dragged.replace(false)
                        && let Some(c) = click.as_ref()
                    {
                        fire_gesture(c, f64::from(pos.x), f64::from(pos.y), name.as_deref());
                    }
                }
                PointerEvent::Wheel { delta, pos, .. } => {
                    if let Some(w) = wheel.as_ref() {
                        w(f64::from(pos.x), f64::from(pos.y), f64::from(delta.y));
                    }
                }
                PointerEvent::Pinch { delta, pos, .. } => {
                    if let Some(p) = pinch.as_ref() {
                        p(f64::from(pos.x), f64::from(pos.y), f64::from(*delta));
                    }
                }

                _ => {}
            }
            let _ = at;
        });
    }
    Ok(AnyView::new(v))
}

#[derive(Clone)]
enum DrawLayer {
    Static(ShapeDraw),
    Dyn(DynShapes),
    Group {
        origin: PtSrc,
        rot: NumSrc,
        has_rot: bool,
        scale: NumSrc,
        pivot: Option<PtSrc>,
        touch_extra: Rc<dyn Fn()>,
        layers: Vec<DrawLayer>,
    },
    Clip {
        rect: RectSrc,
        radius: NumSrc,
        has_radius: bool,
        visible: BoolSrc,
        touch_extra: Rc<dyn Fn()>,
        layers: Vec<DrawLayer>,
    },
}

type RowTemplate = Rc<dyn Fn(&SeqRowData) -> Vec<DrawLayer>>;

#[derive(Clone)]
struct DynShapes {
    rows: crate::bindings::RowsFn,
    render: RowTemplate,
}

impl DrawLayer {
    fn draw_all(layers: &[DrawLayer], g: &mut Painter) {
        for layer in layers {
            match layer {
                DrawLayer::Static(s) => (s.draw)(g),
                DrawLayer::Dyn(d) => {
                    let mut all = Vec::new();
                    for row in (d.rows)() {
                        all.extend((d.render)(&row));
                    }
                    Self::draw_all(&all, g);
                }
                DrawLayer::Group {
                    origin,
                    rot,
                    has_rot,
                    scale,
                    pivot,
                    layers,
                    ..
                } => {
                    let o = origin.val();

                    let mut painter_tr = sequoia_core::Transform::translate(o.x, o.y);
                    let z = scale.val();
                    if z != 1.0 {
                        painter_tr = painter_tr.then(sequoia_core::Transform::scale(z, z));
                    }

                    let list_local: Option<sequoia_core::Transform> = if *has_rot
                        && rot.val() != 0.0
                    {
                        let base = sequoia_core::Transform::translate(o.x, o.y);
                        let rotated = match pivot {
                            Some(pv) => {
                                sequoia_core::Transform::rotate_around_degrees(rot.val(), pv.val())
                                    .then(base)
                            }
                            None => base.then(sequoia_core::Transform::rotate_degrees(rot.val())),
                        };
                        Some(if z != 1.0 {
                            rotated.then(sequoia_core::Transform::scale(z, z))
                        } else {
                            rotated
                        })
                    } else {
                        None
                    };
                    g.with_transform(painter_tr, |g| {
                        if let Some(lt) = list_local {
                            let og = g.origin_global();
                            let global = sequoia_core::Transform::translate(-og.x, -og.y)
                                .then(lt)
                                .then(sequoia_core::Transform::translate(og.x, og.y));
                            g.push_list_transform(global);
                            Self::draw_all(layers, g);
                            g.pop_list_transform();
                        } else {
                            Self::draw_all(layers, g);
                        }
                    });
                }
                DrawLayer::Clip {
                    rect,
                    radius,
                    has_radius,
                    visible,
                    layers,
                    ..
                } => {
                    if !visible.val() {
                        continue;
                    }
                    if *has_radius {
                        let rr = radius.val();
                        g.clipped_rounded(rect.val(), rr, |g| {
                            Self::draw_all(layers, g);
                        });
                    } else {
                        g.clipped(rect.val(), |g| {
                            Self::draw_all(layers, g);
                        });
                    }
                }
            }
        }
    }

    fn touch_all(layers: &[DrawLayer]) {
        for layer in layers {
            match layer {
                DrawLayer::Static(s) => (s.touch)(),
                DrawLayer::Dyn(d) => {
                    let mut all = Vec::new();
                    for row in (d.rows)() {
                        all.extend((d.render)(&row));
                    }
                    Self::touch_all(&all);
                }
                DrawLayer::Group {
                    touch_extra,
                    layers,
                    ..
                }
                | DrawLayer::Clip {
                    touch_extra,
                    layers,
                    ..
                } => {
                    (touch_extra)();
                    Self::touch_all(layers);
                }
            }
        }
    }

    fn pick(layers: &[DrawLayer], p: Point) -> Option<String> {
        Self::pick_in(layers, p, 0.0, 0.0, 1.0)
    }

    fn pick_in(layers: &[DrawLayer], p: Point, ax: f32, ay: f32, az: f32) -> Option<String> {
        if az == 0.0 {
            return None;
        }
        let local = Point::new((p.x - ax) / az, (p.y - ay) / az);
        for layer in layers.iter().rev() {
            match layer {
                DrawLayer::Static(s) => {
                    for (name, test) in s.hits.iter().rev() {
                        if test(local) {
                            return Some(name.clone());
                        }
                    }
                }
                DrawLayer::Dyn(d) => {
                    let rows = (d.rows)();
                    let mut all = Vec::new();
                    for row in rows.iter().rev() {
                        all.extend((d.render)(row));
                    }
                    if let Some(hit) = Self::pick_in(&all, p, ax, ay, az) {
                        return Some(hit);
                    }
                }
                DrawLayer::Group {
                    origin,
                    scale,
                    layers,
                    ..
                } => {
                    let o = origin.val();
                    let z = scale.val();
                    if let Some(hit) =
                        Self::pick_in(layers, p, ax + o.x * az, ay + o.y * az, az * z)
                    {
                        return Some(hit);
                    }
                }
                DrawLayer::Clip { layers, .. } => {
                    if let Some(hit) = Self::pick_in(layers, p, ax, ay, az) {
                        return Some(hit);
                    }
                }
            }
        }
        None
    }
}

fn lower_layer(
    file: &str,
    theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<DrawLayer, SeqError> {
    match node.kind.as_str() {
        "List" => lower_shape_list(file, theme, node, res),
        "Group" => lower_group(file, theme, node, res, row),
        "Clip" => lower_clip(file, theme, node, res, row),
        _ => Ok(DrawLayer::Static(lower_shape(file, theme, node, res, row)?)),
    }
}

fn lower_group(
    file: &str,
    theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<DrawLayer, SeqError> {
    let origin = opt_pt(file, res, row, node, "translate")?.unwrap_or(PtSrc {
        x: NumSrc::Lit(0.0),
        y: NumSrc::Lit(0.0),
    });
    let rot = opt_num(file, res, row, node, "rotate")?;
    let has_rot = find_arg(node, "rotate").is_some();
    let scale = match find_arg(node, "scale") {
        None => NumSrc::Lit(1.0),
        Some(arg) => dyn_f32(file, arg.line, res, row, &arg.value, "scale")?,
    };

    let pivot = opt_pt(file, res, row, node, "pivot")?;
    let mut layers = Vec::new();
    let mut touch_list = vec![origin.touch_rc(), rot.touch_rc(), scale.touch_rc()];
    if let Some(pv) = &pivot {
        touch_list.push(pv.touch_rc());
    }
    for c in &node.children {
        layers.push(lower_layer(file, theme, c, res, row)?);
    }
    Ok(DrawLayer::Group {
        origin,
        rot,
        has_rot,
        scale,
        pivot,
        touch_extra: touches(&touch_list),
        layers,
    })
}
fn lower_shape_list(
    file: &str,
    theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
) -> Result<DrawLayer, SeqError> {
    let items = find_arg(node, "items").ok_or_else(|| {
        SeqError::new(
            file,
            node.line,
            1,
            "List wymaga items: nazwa_propertu".to_string(),
        )
    })?;
    let items_prop = match &items.value {
        SeqValue::Ident(name) => name.clone(),
        _ => {
            return Err(SeqError::new(
                file,
                items.line,
                1,
                "items: to nazwa propertu z hosta (items: wezly)",
            ));
        }
    };
    if let Some(key) = find_arg(node, "key") {
        match &key.value {
            SeqValue::Ident(name) if name == "id" => {}
            _ => {
                return Err(SeqError::new(
                    file,
                    key.line,
                    1,
                    "key: tylko id w inkrementcie 3",
                ));
            }
        }
    }
    let bindings = match res {
        Resolver::Static => {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                "List(items:) wymaga hosta Rust (HostBindings::rows())",
            ));
        }
        Resolver::Host(b) => (*b).clone(),
    };
    let get_rows = bindings.get_rows(&items_prop).ok_or_else(|| {
        SeqError::new(
            file,
            items.line,
            1,
            format!("nieznany prop '{items_prop}' (HostBindings::rows())"),
        )
    })?;
    let template = node.children.clone();
    let file_owned = file.to_string();
    let theme_owned = *theme;

    if let Some(first) = get_rows().first() {
        let res = Resolver::Host(&bindings);
        for n in &template {
            lower_layer(&file_owned, &theme_owned, n, &res, Some(first))?;
        }
    }
    let render = Rc::new(move |row: &SeqRowData| {
        let res = Resolver::Host(&bindings);
        template
            .iter()
            .filter_map(|n| lower_layer(&file_owned, &theme_owned, n, &res, Some(row)).ok())
            .collect::<Vec<_>>()
    });
    Ok(DrawLayer::Dyn(DynShapes {
        rows: get_rows,
        render,
    }))
}

#[derive(Clone)]
enum TextSrc {
    Lit(String),
    Dyn(Rc<dyn Fn() -> String>),
}

impl TextSrc {
    fn val(&self) -> String {
        match self {
            Self::Lit(s) => s.clone(),
            Self::Dyn(f) => f(),
        }
    }

    fn touch_rc(&self) -> Rc<dyn Fn()> {
        let this = self.clone();
        Rc::new(move || {
            this.val();
        })
    }
}

struct TickState {
    angle: Rc<Cell<f32>>,
    last: Rc<std::cell::RefCell<Option<std::time::Instant>>>,
    running: BoolSrc,
    speed: NumSrc,
}

#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn lower_shape(
    file: &str,
    theme: &Theme,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<ShapeDraw, SeqError> {
    let visible = shape_visible(file, res, row, node)?;
    let named = shape_name(file, node, res, row)?;
    let touch_visible: Rc<dyn Fn()> = {
        let visible = visible.clone();
        Rc::new(move || visible.touch())
    };

    let gate = move |draw: Rc<dyn Fn(&mut Painter)>| {
        let visible = visible.clone();
        Rc::new(move |g: &mut Painter| {
            if visible.val() {
                draw(g);
            }
        })
    };

    let mk_hits = |test: Rc<dyn Fn(Point) -> bool>| -> Vec<HitTest> {
        named.clone().map(|name| (name, test)).into_iter().collect()
    };
    match node.kind.as_str() {
        "Grid" => {
            let step = shape_num(file, res, row, node, "step")?;
            let color = shape_color(file, theme, node).unwrap_or(theme.border);
            let t = touches(&[step.touch_rc(), touch_visible.clone()]);
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.grid(step.val(), color);
                })),
                touch: t,
                hits: Vec::new(),
            })
        }
        "Rect" => {
            let r = rect_src_of(file, res, row, node)?;
            let radius = opt_num(file, res, row, node, "radius")?;
            let color = shape_color(file, theme, node)?;
            let t = touches(&[r.touch_rc(), radius.touch_rc(), touch_visible.clone()]);
            let hr = r.clone();
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.rounded(r.val(), radius.val(), Fill::solid(color));
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| hr.val().contains(p))),
            })
        }
        "Circle" => {
            let c = shape_pt(file, res, row, node, "c")?;
            let r = shape_num(file, res, row, node, "r")?;
            let color = shape_color(file, theme, node)?;

            let pr = match find_arg(node, "pick_r") {
                None => r.clone(),
                Some(arg) => dyn_f32(file, arg.line, res, row, &arg.value, "pick_r")?,
            };
            let t = touches(&[
                c.touch_rc(),
                r.touch_rc(),
                pr.touch_rc(),
                touch_visible.clone(),
            ]);
            let (hc, hr, hp) = (c.clone(), r.clone(), pr.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.circle(c.val(), r.val(), Fill::solid(color));
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let c = hc.val();
                    let dx = p.x - c.x;
                    let dy = p.y - c.y;
                    let rr = hp.val().max(hr.val());
                    dx * dx + dy * dy <= rr * rr
                })),
            })
        }
        "Ellipse" => {
            let c = shape_pt(file, res, row, node, "c")?;
            let rx = shape_num(file, res, row, node, "rx")?;
            let ry = shape_num(file, res, row, node, "ry")?;
            let color = shape_color(file, theme, node)?;
            let t = touch4e(&c, &rx, &ry, &touch_visible);
            let (hc, hx, hy) = (c.clone(), rx.clone(), ry.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.ellipse(c.val(), rx.val(), ry.val(), color);
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let c = hc.val();
                    let (rx, ry) = (hx.val(), hy.val());
                    if rx <= 0.0 || ry <= 0.0 {
                        return false;
                    }
                    let dx = (p.x - c.x) / rx;
                    let dy = (p.y - c.y) / ry;
                    dx * dx + dy * dy <= 1.0
                })),
            })
        }
        "Ring" => {
            let c = shape_pt(file, res, row, node, "c")?;
            let r = shape_num(file, res, row, node, "r")?;
            let w = shape_num(file, res, row, node, "width")?;
            let color = only_color(file, theme, node)?;
            let t = touch4e(&c, &r, &w, &touch_visible);
            let (hc, hr, hw) = (c.clone(), r.clone(), w.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.ring(c.val(), r.val(), w.val(), color);
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let c = hc.val();
                    let dx = p.x - c.x;
                    let dy = p.y - c.y;
                    ((dx * dx + dy * dy).sqrt() - hr.val()).abs() <= hw.val()
                })),
            })
        }
        "Line" => {
            let a = shape_pt(file, res, row, node, "a")?;
            let b = shape_pt(file, res, row, node, "b")?;
            let w = shape_num(file, res, row, node, "width")?;
            let color = only_color(file, theme, node)?;
            let t = touch4(&a, &b, &w, &touch_visible);
            let (ha, hb, hw) = (a.clone(), b.clone(), w.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.line(
                        a.val(),
                        b.val(),
                        sequoia_core::BrushStroke::new(w.val()),
                        color,
                    );
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    dist_seg(p, ha.val(), hb.val()) <= (hw.val() / 2.0).max(3.0)
                })),
            })
        }
        "Arrow" => {
            let a = shape_pt(file, res, row, node, "a")?;
            let b = shape_pt(file, res, row, node, "b")?;
            let w = shape_num(file, res, row, node, "width")?;
            let head = shape_num(file, res, row, node, "head")?;
            let color = only_color(file, theme, node)?;
            let t = touch5(&a, &b, &w, &head, &touch_visible);
            let (ha, hb, hw) = (a.clone(), b.clone(), w.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    let h = head.val();
                    g.arrow(a.val(), b.val(), w.val(), h, h, color);
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    dist_seg(p, ha.val(), hb.val()) <= (hw.val() / 2.0).max(3.0)
                })),
            })
        }
        "Capsule" => {
            let a = shape_pt(file, res, row, node, "a")?;
            let b = shape_pt(file, res, row, node, "b")?;
            let r = shape_num(file, res, row, node, "r")?;
            let color = shape_color(file, theme, node)?;
            let t = touch4e(&a, &b, &r, &touch_visible);
            let (ha, hb, hr) = (a.clone(), b.clone(), r.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.capsule(a.val(), b.val(), r.val(), color);
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    dist_seg(p, ha.val(), hb.val()) <= hr.val()
                })),
            })
        }
        "Polygon" => {
            let pts = points_src_of(file, res, row, node)?;
            let color = shape_color(file, theme, node)?;
            let t = touch_pts(&pts, &touch_visible);
            let hpts = pts.clone();
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    let v: Vec<Point> = pts.iter().map(PtSrc::val).collect();
                    g.polygon(&v, color);
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let v: Vec<Point> = hpts.iter().map(PtSrc::val).collect();
                    point_in_poly(p, &v)
                })),
            })
        }
        "Polyline" => {
            let pts = points_src_of(file, res, row, node)?;
            let w = shape_num(file, res, row, node, "width")?;
            let color = only_color(file, theme, node)?;
            let t = touch_pts_w(&pts, &w, &touch_visible);
            let (hpts, hw) = (pts.clone(), w.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    let v: Vec<Point> = pts.iter().map(PtSrc::val).collect();
                    g.polyline(&v, sequoia_core::BrushStroke::new(w.val()), color);
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let v: Vec<Point> = hpts.iter().map(PtSrc::val).collect();
                    dist_polyline(p, &v) <= (hw.val() / 2.0).max(3.0)
                })),
            })
        }
        "Spline" => {
            let pts = points_src_of(file, res, row, node)?;
            if pts.len() != 4 {
                return Err(SeqError::new(
                    file,
                    node.line,
                    1,
                    "A spline is exactly 4 points (p0, c0, c1, p1)",
                ));
            }
            let w = shape_num(file, res, row, node, "width")?;
            let color = only_color(file, theme, node)?;
            let t = touch_pts_w(&pts, &w, &touch_visible);
            let (hpts, hw) = (pts.clone(), w.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    let v: Vec<Point> = pts.iter().map(PtSrc::val).collect();
                    g.polyline(
                        &sample_cubic(&v),
                        sequoia_core::BrushStroke::new(w.val()),
                        color,
                    );
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let v: Vec<Point> = hpts.iter().map(PtSrc::val).collect();
                    dist_polyline(p, &sample_cubic(&v)) <= (hw.val() / 2.0).max(3.0)
                })),
            })
        }
        "Star" => {
            let c = shape_pt(file, res, row, node, "c")?;
            let outer = shape_num(file, res, row, node, "outer")?;
            let inner = shape_num(file, res, row, node, "inner")?;
            let n = shape_num(file, res, row, node, "n")?;
            let rot = opt_num(file, res, row, node, "rotation")?;
            let color = only_color(file, theme, node)?;
            let t = touch5(&c, &outer, &inner, &n, &touch_visible);
            let (hc, ho) = (c.clone(), outer.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.star(
                        c.val(),
                        outer.val(),
                        inner.val(),
                        n.val() as u32,
                        rot.val(),
                        color,
                    );
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let c = hc.val();
                    let dx = p.x - c.x;
                    let dy = p.y - c.y;
                    dx * dx + dy * dy <= ho.val() * ho.val()
                })),
            })
        }
        "Ngon" => {
            let c = shape_pt(file, res, row, node, "c")?;
            let r = shape_num(file, res, row, node, "r")?;
            let sides = shape_num(file, res, row, node, "sides")?;
            let rot = opt_num(file, res, row, node, "rotation")?;
            let color = only_color(file, theme, node)?;
            let t = touch5(&c, &r, &sides, &rot, &touch_visible);
            let (hc, hr) = (c.clone(), r.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.regular_polygon(c.val(), r.val(), sides.val() as u32, rot.val(), color);
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let c = hc.val();
                    let dx = p.x - c.x;
                    let dy = p.y - c.y;
                    dx * dx + dy * dy <= hr.val() * hr.val()
                })),
            })
        }
        "Cross" => {
            let c = shape_pt(file, res, row, node, "c")?;
            let half = shape_num(file, res, row, node, "half")?;
            let w = shape_num(file, res, row, node, "width")?;
            let color = only_color(file, theme, node)?;
            let t = touch4e(&c, &half, &w, &touch_visible);
            let (hc, hh, hw) = (c.clone(), half.clone(), w.clone());
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.cross(c.val(), half.val(), w.val(), color);
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let c = hc.val();
                    let (dx, dy) = ((p.x - c.x).abs(), (p.y - c.y).abs());
                    let (h, w) = (hh.val(), hw.val() / 2.0);
                    (dx <= h && dy <= w) || (dy <= h && dx <= w)
                })),
            })
        }
        "Text2d" => {
            let color = only_color(file, theme, node)?;
            let at = shape_pt(file, res, row, node, "at")?;
            let size = opt_num(file, res, row, node, "size")?;
            let has_size = find_arg(node, "size").is_some();
            let t = touch3(&at, &size, &touch_visible);

            let text: TextSrc = if let Some(t_arg) = find_arg(node, "t") {
                match &t_arg.value {
                    SeqValue::Str(key) => match res {
                        Resolver::Static => {
                            return Err(SeqError::new(
                                file,
                                t_arg.line,
                                1,
                                "t: wymaga hosta Rust (HostBindings::i18n)",
                            ));
                        }
                        Resolver::Host(b) => {
                            let i18n = b.get_i18n().ok_or_else(|| {
                                SeqError::new(file, t_arg.line, 1, "t: wymaga HostBindings::i18n()")
                            })?;
                            let key = key.clone();
                            TextSrc::Dyn(Rc::new(move || i18n.t(&key)))
                        }
                    },
                    _ => {
                        return Err(SeqError::new(
                            file,
                            t_arg.line,
                            1,
                            "t: is the key in quotation marks (t: \"save\")",
                        ));
                    }
                }
            } else {
                match find_arg(node, "str") {
                    None => {
                        return Err(SeqError::new(
                            file,
                            node.line,
                            1,
                            "Text2d wymaga str: \"...\"",
                        ));
                    }
                    Some(arg) => match &arg.value {
                        SeqValue::Str(s) => TextSrc::Lit(s.clone()),
                        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
                            Resolver::Static => {
                                return Err(SeqError::new(
                                    file,
                                    arg.line,
                                    1,
                                    "str: z propertu wymaga hosta",
                                ));
                            }
                            Resolver::Host(b) => TextSrc::Dyn(display_of(file, arg.line, b, prop)?),
                        },
                        SeqValue::Env(name) => match res {
                            Resolver::Static => {
                                return Err(SeqError::new(
                                    file,
                                    arg.line,
                                    1,
                                    "str: from environment requires host",
                                ));
                            }
                            Resolver::Host(b) => {
                                TextSrc::Dyn(display_env_of(file, arg.line, b, name)?)
                            }
                        },
                        SeqValue::ItemField(field) => match res {
                            Resolver::Static => {
                                return Err(SeqError::new(
                                    file,
                                    arg.line,
                                    1,
                                    "str: z wiersza wymaga hosta",
                                ));
                            }
                            Resolver::Host(_) => {
                                TextSrc::Dyn(display_field(file, arg.line, row, field)?)
                            }
                        },
                        _ => {
                            return Err(SeqError::new(
                                file,
                                arg.line,
                                1,
                                "str: is a string in quotation marks or dynamic(...)",
                            ));
                        }
                    },
                }
            };
            let t = touches(&[t, text.touch_rc()]);
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.text(
                        &text.val(),
                        at.val(),
                        if has_size { size.val() } else { 13.0 },
                        color,
                    );
                })),
                touch: t,
                hits: Vec::new(),
            })
        }
        "Svg" => {
            let d = shape_str(file, node, "d")?;
            let color = shape_color(file, theme, node)?;
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.ctx().draw_svg(&d, None, color);
                })),
                touch: touch_visible,
                hits: Vec::new(),
            })
        }
        "Dots" => {
            let step = shape_num(file, res, row, node, "step")?;
            let color = shape_color(file, theme, node).unwrap_or(theme.border);
            let radius = opt_num(file, res, row, node, "radius")?;
            let has_radius = find_arg(node, "radius").is_some();
            let t = touch3(&step, &radius, &touch_visible);
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    let st = step.val();
                    let r = if has_radius {
                        radius.val()
                    } else {
                        (st / 8.0).max(1.0)
                    };
                    g.dots(st, Vec2::ZERO, r, color);
                })),
                touch: t,
                hits: Vec::new(),
            })
        }
        "Axes" => {
            let color = shape_color(file, theme, node).unwrap_or(theme.border);
            let w = opt_num(file, res, row, node, "width")?;
            let has_w = find_arg(node, "width").is_some();
            let at = opt_pt(file, res, row, node, "at")?.unwrap_or(PtSrc {
                x: NumSrc::Lit(0.0),
                y: NumSrc::Lit(0.0),
            });
            let t = touch3(&w, &at, &touch_visible);
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.axes(at.val(), color, if has_w { w.val() } else { 1.0 });
                })),
                touch: t,
                hits: Vec::new(),
            })
        }
        "Frame" => {
            let r = rect_src_of(file, res, row, node)?;
            let w = shape_num(file, res, row, node, "width")?;
            let color = only_color(file, theme, node)?;
            let t = touch3(&r, &w, &touch_visible);
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.frame(r.val(), color, w.val());
                })),
                touch: t,
                hits: Vec::new(),
            })
        }
        "Marquee" => {
            let r = rect_src_of(file, res, row, node)?;
            let color = only_color(file, theme, node)?;
            let t = touch2(&r, &touch_visible);
            let hr = r.clone();
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    let q = r.val();
                    g.marquee(
                        Point::new(q.left(), q.top()),
                        Point::new(q.right(), q.bottom()),
                        Fill::solid(Color::TRANSPARENT),
                        color,
                    );
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| hr.val().contains(p))),
            })
        }
        "Handle" => {
            let at = shape_pt(file, res, row, node, "at")?;
            let r = opt_num(file, res, row, node, "r")?;
            let has_r = find_arg(node, "r").is_some();
            let color = only_color(file, theme, node)?;
            let t = touch3(&at, &r, &touch_visible);
            let hat = at.clone();
            let hr2 = r.clone();
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    g.knob(at.val(), if has_r { r.val() } else { 4.0 }, color);
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| {
                    let c = hat.val();
                    let rr = if has_r { hr2.val() } else { 4.0 };
                    let dx = p.x - c.x;
                    let dy = p.y - c.y;
                    dx * dx + dy * dy <= rr.max(6.0) * rr.max(6.0)
                })),
            })
        }
        "Image2d" => {
            let src: TextSrc = match find_arg(node, "src") {
                None => {
                    return Err(SeqError::new(
                        file,
                        node.line,
                        1,
                        "Image2d wymaga src: \"...\"",
                    ));
                }
                Some(arg) => match &arg.value {
                    SeqValue::Str(s) => TextSrc::Lit(s.clone()),
                    SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
                        Resolver::Static => {
                            return Err(SeqError::new(
                                file,
                                arg.line,
                                1,
                                "src: z propertu wymaga hosta",
                            ));
                        }
                        Resolver::Host(b) => TextSrc::Dyn(display_of(file, arg.line, b, prop)?),
                    },
                    SeqValue::Env(name) => match res {
                        Resolver::Static => {
                            return Err(SeqError::new(
                                file,
                                arg.line,
                                1,
                                "src: from environment requires host",
                            ));
                        }
                        Resolver::Host(b) => TextSrc::Dyn(display_env_of(file, arg.line, b, name)?),
                    },
                    SeqValue::ItemField(field) => match res {
                        Resolver::Static => {
                            return Err(SeqError::new(
                                file,
                                arg.line,
                                1,
                                "src: z wiersza wymaga hosta",
                            ));
                        }
                        Resolver::Host(_) => {
                            TextSrc::Dyn(display_field(file, arg.line, row, field)?)
                        }
                    },
                    _ => {
                        return Err(SeqError::new(
                            file,
                            arg.line,
                            1,
                            "src: is a string in quotation marks or dynamic(...)",
                        ));
                    }
                },
            };
            let r = rect_src_of(file, res, row, node)?;
            let uv = uv_src_of(file, res, row, node)?;
            let t = touches(&[
                r.touch_rc(),
                src.touch_rc(),
                uv[0].touch_rc(),
                uv[1].touch_rc(),
                uv[2].touch_rc(),
                uv[3].touch_rc(),
                touch_visible.clone(),
            ]);
            let hr = r.clone();
            Ok(ShapeDraw {
                draw: gate(Rc::new(move |g: &mut Painter| {
                    let path = src.val();
                    if path.is_empty() {
                        return;
                    }
                    g.image(
                        r.val(),
                        [uv[0].val(), uv[1].val(), uv[2].val(), uv[3].val()],
                        ImageSource::File(path),
                    );
                })),
                touch: t,
                hits: mk_hits(Rc::new(move |p: Point| hr.val().contains(p))),
            })
        }
        "Clip" | "Group" | "List" => Err(SeqError::new(
            file,
            node.line,
            1,
            format!(
                "'{}' is a Canvas layer, not a shape (internal error)",
                node.kind
            ),
        )),
        other => Err(SeqError::new(
            file,
            node.line,
            1,
            format!("unknown shape '{other}'"),
        )),
    }
}

trait FlexBoxOps {
    fn set_width(self, w: Length) -> Self;
    fn set_height(self, h: Length) -> Self;
    fn set_padding(self, p: PaddingVal) -> Self;
    fn set_padding_x(self, x: f32) -> Self;
    fn set_padding_y(self, y: f32) -> Self;
    fn set_background(self, c: Color) -> Self;
    fn set_flex(self, f: f32) -> Self;
    fn set_gap(self, g: f32) -> Self;
    fn set_cross_gap(self, g: f32) -> Self;
    fn set_wrap(self, w: bool) -> Self;
    fn set_align(self, a: Align) -> Self;
    fn set_justify(self, j: Justify) -> Self;
    fn set_min_width(self, v: f32) -> Self;
    fn set_max_width(self, v: f32) -> Self;
    fn set_min_height(self, v: f32) -> Self;
    fn set_max_height(self, v: f32) -> Self;
    fn set_margin(self, m: MarginVal) -> Self;
    fn set_margin_x(self, x: f32) -> Self;
    fn set_margin_y(self, y: f32) -> Self;
    fn set_margin_left(self, v: f32) -> Self;
    fn set_margin_right(self, v: f32) -> Self;
    fn set_margin_top(self, v: f32) -> Self;
    fn set_margin_bottom(self, v: f32) -> Self;
    fn set_overflow(self, o: Overflow) -> Self;
    fn set_hit_testable(self, on: bool) -> Self;
    fn add_child(self, kid: AnyView) -> Self;
    fn add_child_flex(self, kid: AnyView, flex: f32) -> Self;
}

macro_rules! impl_flex_box {
    ($t:ty) => {
        impl FlexBoxOps for $t {
            fn set_width(self, w: Length) -> Self {
                self.width(w)
            }
            fn set_height(self, h: Length) -> Self {
                self.height(h)
            }
            fn set_padding(self, p: PaddingVal) -> Self {
                match p {
                    PaddingVal::All(v) => self.padding(v),
                    PaddingVal::Xy(x, y) => self.padding_xy(x, y),
                }
            }
            fn set_padding_x(self, x: f32) -> Self {
                self.padding_x(x)
            }
            fn set_padding_y(self, y: f32) -> Self {
                self.padding_y(y)
            }
            fn set_background(self, c: Color) -> Self {
                self.background(c)
            }
            fn set_flex(self, f: f32) -> Self {
                self.flex(f)
            }
            fn set_gap(self, g: f32) -> Self {
                self.gap(g)
            }
            fn set_cross_gap(self, g: f32) -> Self {
                self.cross_gap(g)
            }
            fn set_wrap(self, w: bool) -> Self {
                self.wrap(w)
            }
            fn set_align(self, a: Align) -> Self {
                self.align_items(a)
            }
            fn set_justify(self, j: Justify) -> Self {
                self.justify_content(j)
            }
            fn set_min_width(self, v: f32) -> Self {
                self.min_width(v)
            }
            fn set_max_width(self, v: f32) -> Self {
                self.max_width(v)
            }
            fn set_min_height(self, v: f32) -> Self {
                self.min_height(v)
            }
            fn set_max_height(self, v: f32) -> Self {
                self.max_height(v)
            }
            fn set_margin(self, m: MarginVal) -> Self {
                match m {
                    MarginVal::All(v) => self.margin(v),
                    MarginVal::Xy(x, y) => self.margin_xy(x, y),
                }
            }
            fn set_margin_x(self, x: f32) -> Self {
                self.margin_x(x)
            }
            fn set_margin_y(self, y: f32) -> Self {
                self.margin_y(y)
            }
            fn set_margin_left(self, v: f32) -> Self {
                self.margin_left(v)
            }
            fn set_margin_right(self, v: f32) -> Self {
                self.margin_right(v)
            }
            fn set_margin_top(self, v: f32) -> Self {
                self.margin_top(v)
            }
            fn set_margin_bottom(self, v: f32) -> Self {
                self.margin_bottom(v)
            }
            fn set_overflow(self, o: Overflow) -> Self {
                self.overflow(o)
            }
            fn set_hit_testable(self, on: bool) -> Self {
                self.hit_testable(on)
            }
            fn add_child(self, kid: AnyView) -> Self {
                self.child(kid)
            }
            fn add_child_flex(self, kid: AnyView, flex: f32) -> Self {
                self.child_flex(kid, flex)
            }
        }
    };
}

impl_flex_box!(Column);
impl_flex_box!(Row);

fn has_click(node: &SeqNode) -> bool {
    find_arg(node, "on_click").is_some() || find_arg(node, "hover").is_some()
}

fn is_click_boundary(node: &SeqNode) -> bool {
    match node.kind.as_str() {
        "Button" | "TextField" | "Checkbox" | "Switch" | "Slider" | "Dropdown" | "Canvas" => true,
        "Container" | "Row" | "Column" => has_click(node),
        _ => false,
    }
}

fn can_be_hitless(kind: &str) -> bool {
    matches!(
        kind,
        "Column"
            | "Row"
            | "Container"
            | "Panel"
            | "Stack"
            | "Wrap"
            | "Masonry"
            | "Text"
            | "Icon"
            | "Spacer"
    )
}

fn click_through(node: &SeqNode) -> SeqNode {
    if is_click_boundary(node) {
        return node.clone();
    }
    let mut out = node.clone();
    if can_be_hitless(&node.kind) && !out.args.iter().any(|a| a.name == "hit") {
        out.args.push(SeqArg {
            name: "hit".to_string(),
            line: node.line,
            value: SeqValue::Bool(false),
        });
    }
    out.children = node.children.iter().map(click_through).collect();
    out
}

fn lower_clickable_flex<V>(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    make: V,
) -> Result<AnyView, SeqError>
where
    V: FlexBoxOps + View + 'static,
{
    let mut outer = Container::new();
    if let Some(s) = box_style_of(file, styles, node)? {
        outer = outer.box_style(s);
    }
    for arg in &node.args {
        match arg.name.as_str() {
            "width" => outer = outer.width(len(file, arg, res, row)?),
            "height" => outer = outer.height(len(file, arg, res, row)?),
            "min_width" => outer = outer.min_width(num(file, arg, res, row)?),
            "max_width" => outer = outer.max_width(num(file, arg, res, row)?),
            "min_height" => outer = outer.min_height(num(file, arg, res, row)?),
            "max_height" => outer = outer.max_height(num(file, arg, res, row)?),
            "margin" => outer = apply_container_margin(outer, file, arg, res, row)?,
            "margin_x" => outer = outer.margin_x(num(file, arg, res, row)?),
            "margin_y" => outer = outer.margin_y(num(file, arg, res, row)?),
            "margin_left" => outer = outer.margin_left(num(file, arg, res, row)?),
            "margin_right" => outer = outer.margin_right(num(file, arg, res, row)?),
            "margin_top" => outer = outer.margin_top(num(file, arg, res, row)?),
            "margin_bottom" => outer = outer.margin_bottom(num(file, arg, res, row)?),
            "overflow" => outer = outer.overflow(overflow_val(file, arg)?),
            "background" => outer = outer.background(color(file, theme, arg)?),
            "hover" => outer = outer.hover(color(file, theme, arg)?),
            _ => {}
        }
    }
    if let Some(arg) = find_arg(node, "on_click") {
        match res {
            Resolver::Static => {
                return Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    "on_click wymaga hosta Rust",
                ));
            }
            Resolver::Host(b) => match &arg.value {
                SeqValue::Callback { .. } | SeqValue::CallSeq { .. } => {
                    let fire = wire_action(file, arg.line, b, &arg.value, row)?;
                    outer = outer.on_click(move || fire());
                }
                _ => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "on_click: { akcja(args) }",
                    ));
                }
            },
        }
    }

    let mut inner_args: Vec<SeqArg> = node
        .args
        .iter()
        .filter(|a| {
            !matches!(
                a.name.as_str(),
                "min_width"
                    | "max_width"
                    | "min_height"
                    | "max_height"
                    | "margin"
                    | "margin_x"
                    | "margin_y"
                    | "margin_left"
                    | "margin_right"
                    | "margin_top"
                    | "margin_bottom"
                    | "overflow"
                    | "background"
                    | "style"
                    | "on_click"
                    | "hover"
            )
        })
        .cloned()
        .collect();

    if !inner_args.iter().any(|a| a.name == "hit") {
        inner_args.push(SeqArg {
            name: "hit".to_string(),
            line: node.line,
            value: SeqValue::Bool(false),
        });
    }
    let inner_node = SeqNode {
        kind: node.kind.clone(),
        line: node.line,
        label: node.label.clone(),
        args: inner_args,
        children: node.children.iter().map(click_through).collect(),
    };
    let inner = apply_flex_box(file, theme, &inner_node, make, res, row)?;
    let built = boxed_children(file, theme, styles, &inner_node, inner, res, row)?;
    outer = outer.child(built);
    Ok(AnyView::new(outer))
}

fn indents_of(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
) -> Result<Option<Indents>, SeqError> {
    let mut ind = Indents::NONE;
    let mut any = false;
    for arg in &node.args {
        match arg.name.as_str() {
            "indent" => {
                any = true;
                let n = san_indent(num(file, arg, res, row)?);
                ind.top = Some(n);
                ind.bottom = Some(n);
                ind.left = Some(n);
                ind.right = Some(n);
            }
            "indent_x" => {
                any = true;
                let n = san_indent(num(file, arg, res, row)?);
                ind.left = Some(n);
                ind.right = Some(n);
            }
            "indent_y" => {
                any = true;
                let n = san_indent(num(file, arg, res, row)?);
                ind.top = Some(n);
                ind.bottom = Some(n);
            }
            "indent_left" => {
                any = true;
                ind.left = Some(san_indent(num(file, arg, res, row)?));
            }
            "indent_right" => {
                any = true;
                ind.right = Some(san_indent(num(file, arg, res, row)?));
            }
            "indent_top" => {
                any = true;
                ind.top = Some(san_indent(num(file, arg, res, row)?));
            }
            "indent_bottom" => {
                any = true;
                ind.bottom = Some(san_indent(num(file, arg, res, row)?));
            }
            _ => {}
        }
    }
    Ok(any.then_some(ind))
}

fn san_indent(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

fn maybe_indent(
    view: AnyView,
    file: &str,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError> {
    match indents_of(file, res, row, node)? {
        None => Ok(view),
        Some(ind) => Ok(AnyView::new(Margined::with_indents(view, ind))),
    }
}

fn boxed_children<V>(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    mut v: V,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<AnyView, SeqError>
where
    V: FlexBoxOps + View + 'static,
{
    for c in &node.children {
        let kid = lower_node(file, theme, styles, c, res, row)?;

        v = match child_flex_weight(file, res, row, c)? {
            Some(w) => v.add_child_flex(kid, w),
            None => v.add_child(kid),
        };
    }
    Ok(AnyView::new(v))
}

fn child_flex_weight(
    file: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
    node: &SeqNode,
) -> Result<Option<f32>, SeqError> {
    match find_arg(node, "flex") {
        None => Ok(None),
        Some(arg) => match &arg.value {
            SeqValue::Num(n) => Ok(Some(*n)),
            SeqValue::Ident(_)
            | SeqValue::Dynamic(_)
            | SeqValue::ItemField(_)
            | SeqValue::Env(_) => Ok(Some(num(file, arg, res, row)?)),
            _ => Err(SeqError::new(file, arg.line, 1, "'flex' to liczba")),
        },
    }
}

fn single_child(
    file: &str,
    theme: &Theme,
    styles: &BuiltStyles,
    node: &SeqNode,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Option<AnyView>, SeqError> {
    match node.children.len() {
        0 => Ok(None),
        1 => Ok(Some(lower_node(
            file,
            theme,
            styles,
            &node.children[0],
            res,
            row,
        )?)),
        _ => {
            let mut col = Column::new();
            for c in &node.children {
                let kid = lower_node(file, theme, styles, c, res, row)?;
                col = match child_flex_weight(file, res, row, c)? {
                    Some(w) => col.child_flex(kid, w),
                    None => col.child(kid),
                };
            }
            Ok(Some(AnyView::new(col)))
        }
    }
}

fn unreachable_checked<T>(
    file: &str,
    node: &SeqNode,
    arg: &crate::ir::SeqArg,
) -> Result<T, SeqError> {
    Err(SeqError::new(
        file,
        arg.line,
        1,
        format!(
            "unknown prop '{}' in {} (call check() first)",
            arg.name, node.kind
        ),
    ))
}

fn find_arg<'n>(node: &'n SeqNode, name: &str) -> Option<&'n crate::ir::SeqArg> {
    node.args.iter().find(|a| a.name == name)
}

enum RowVal {
    Sig(FieldSignal),

    Id(i64),
}

fn row_value(
    file: &str,
    line: usize,
    row: Option<&SeqRowData>,
    field: &str,
) -> Result<RowVal, SeqError> {
    match row {
        None => Err(SeqError::new(
            file,
            line,
            1,
            format!("item.{field} only inside List row"),
        )),
        Some(r) if field == "id" => Ok(RowVal::Id(r.id)),
        Some(r) => r
            .fields
            .get(field)
            .cloned()
            .map(RowVal::Sig)
            .ok_or_else(|| {
                let mut names: Vec<&str> = r.fields.keys().map(String::as_str).collect();
                names.sort_unstable();
                SeqError::new(
                    file,
                    line,
                    1,
                    format!(
                        "nieznane pole wiersza '{field}' (pola: {})",
                        names.join(", ")
                    ),
                )
            }),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn row_num(
    file: &str,
    line: usize,
    row: Option<&SeqRowData>,
    field: &str,
) -> Result<f32, SeqError> {
    match row_value(file, line, row, field)? {
        RowVal::Sig(FieldSignal::Int(sig)) => Ok(sig.get_untracked() as f32),
        RowVal::Sig(FieldSignal::Float(sig)) => Ok(sig.get_untracked() as f32),
        RowVal::Id(id) => Ok(id as f32),
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("pole wiersza '{field}' to nie liczba (Int/Float/id)"),
        )),
    }
}

fn opt_str(
    file: &str,
    node: &SeqNode,
    name: &str,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Option<String>, SeqError> {
    match find_arg(node, name) {
        None => Ok(None),
        Some(arg) => match &arg.value {
            SeqValue::Str(s) => Ok(Some(s.clone())),
            SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
                Resolver::Static => Err(str_err(file, arg, name)),
                Resolver::Host(b) => Ok(Some(
                    b.get_str(prop)
                        .map(|sig| sig.get_untracked())
                        .or_else(|| b.get_memo_str(prop).map(|m| m.get_untracked()))
                        .or_else(|| b.get_env_str(prop).map(|sig| sig.get_untracked()))
                        .ok_or_else(|| prop_err(file, arg.line, prop, "String"))?,
                )),
            },
            SeqValue::Env(env_name) => match res {
                Resolver::Static => Err(str_err(file, arg, name)),
                Resolver::Host(b) => Ok(Some(
                    b.get_env_str(env_name)
                        .map(|sig| sig.get_untracked())
                        .ok_or_else(|| {
                            SeqError::new(
                                file,
                                arg.line,
                                1,
                                format!(
                                    "unknown env.{env_name} or wrong type (here you need: String)"
                                ),
                            )
                        })?,
                )),
            },
            SeqValue::ItemField(field) => match res {
                Resolver::Static => Err(str_err(file, arg, name)),
                Resolver::Host(_) => match row_value(file, arg.line, row, field)? {
                    RowVal::Sig(FieldSignal::Str(sig)) => Ok(Some(sig.get_untracked())),
                    RowVal::Id(id) => Ok(Some(id.to_string())),
                    _ => Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("pole wiersza '{field}' to nie tekst (String/id)"),
                    )),
                },
            },
            _ => Err(str_err(file, arg, name)),
        },
    }
}

fn str_err(file: &str, arg: &crate::ir::SeqArg, name: &str) -> SeqError {
    SeqError::new(
        file,
        arg.line,
        1,
        format!("'{name}' is a string in quotation marks"),
    )
}

fn prop_err(file: &str, line: usize, prop: &str, want: &str) -> SeqError {
    SeqError::new(
        file,
        line,
        1,
        format!("unknown prop '{prop}' or wrong type (here you need: {want})"),
    )
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn prop_num_env(file: &str, line: usize, b: &HostBindings, prop: &str) -> Result<f32, SeqError> {
    if let Some(sig) = b.get_float(prop) {
        Ok(sig.get_untracked() as f32)
    } else if let Some(sig) = b.get_int(prop) {
        Ok(sig.get_untracked() as f32)
    } else if let Some(m) = b.get_memo_float(prop) {
        Ok(m.get_untracked() as f32)
    } else if let Some(m) = b.get_memo_int(prop) {
        Ok(m.get_untracked() as f32)
    } else {
        Err(prop_err(file, line, prop, "Float/Int"))
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn env_num(file: &str, line: usize, b: &HostBindings, name: &str) -> Result<f32, SeqError> {
    if let Some(sig) = b.get_env_float(name) {
        Ok(sig.get_untracked() as f32)
    } else if let Some(sig) = b.get_env_int(name) {
        Ok(sig.get_untracked() as f32)
    } else {
        Err(SeqError::new(
            file,
            line,
            1,
            format!("unknown env.{name} or wrong type (here you need: Float/Int)"),
        ))
    }
}

fn prop_flag(file: &str, line: usize, b: &HostBindings, prop: &str) -> Result<bool, SeqError> {
    if let Some(sig) = b.get_bool(prop) {
        Ok(sig.get_untracked())
    } else if let Some(m) = b.get_memo_bool(prop) {
        Ok(m.get_untracked())
    } else {
        Err(prop_err(file, line, prop, "Bool"))
    }
}

fn env_flag(file: &str, line: usize, b: &HostBindings, name: &str) -> Result<bool, SeqError> {
    b.get_env_bool(name)
        .map(|sig| sig.get_untracked())
        .ok_or_else(|| {
            SeqError::new(
                file,
                line,
                1,
                format!("unknown env.{name} or wrong type (here you need: Bool)"),
            )
        })
}

fn num(
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<f32, SeqError> {
    match &arg.value {
        SeqValue::Num(n) => {
            if n.is_finite() {
                Ok(*n)
            } else {
                Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!("'{}' must be finished", arg.name),
                ))
            }
        }
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(num_err(file, arg)),
            Resolver::Host(b) => prop_num_env(file, arg.line, b, prop),
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(num_err(file, arg)),
            Resolver::Host(b) => env_num(file, arg.line, b, name),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(num_err(file, arg)),
            Resolver::Host(_) => row_num(file, arg.line, row, field),
        },
        _ => Err(num_err(file, arg)),
    }
}

fn num_err(file: &str, arg: &crate::ir::SeqArg) -> SeqError {
    SeqError::new(file, arg.line, 1, format!("'{}' to liczba", arg.name))
}

fn flag(
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<bool, SeqError> {
    match &arg.value {
        SeqValue::Bool(b) => Ok(*b),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(flag_err(file, arg)),
            Resolver::Host(b) => prop_flag(file, arg.line, b, prop),
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(flag_err(file, arg)),
            Resolver::Host(b) => env_flag(file, arg.line, b, name),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(flag_err(file, arg)),
            Resolver::Host(_) => match row_value(file, arg.line, row, field)? {
                RowVal::Sig(FieldSignal::Bool(sig)) => Ok(sig.get_untracked()),
                _ => Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!("pole wiersza '{field}' to nie Bool"),
                )),
            },
        },
        _ => Err(flag_err(file, arg)),
    }
}

fn flag_err(file: &str, arg: &crate::ir::SeqArg) -> SeqError {
    SeqError::new(file, arg.line, 1, format!("'{}' to true/false", arg.name))
}

fn ident(file: &str, arg: &crate::ir::SeqArg) -> Result<String, SeqError> {
    match &arg.value {
        SeqValue::Ident(s) => Ok(s.clone()),
        _ => Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("'{}' is the name (without quotes)", arg.name),
        )),
    }
}

fn len(
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Length, SeqError> {
    match &arg.value {
        SeqValue::Length(l) => match *l {
            LengthVal::Fill(w) => Ok(Length::Fill(w)),
            LengthVal::Px(v) => Ok(Length::Px(v)),
            LengthVal::Fit => Ok(Length::Fit),
            LengthVal::Percent(p) => Ok(Length::Percent(p)),
        },
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(len_err(file, arg)),
            Resolver::Host(b) => Ok(Length::Px(prop_num_env(file, arg.line, b, prop)?)),
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(len_err(file, arg)),
            Resolver::Host(b) => Ok(Length::Px(env_num(file, arg.line, b, name)?)),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(len_err(file, arg)),
            Resolver::Host(_) => Ok(Length::Px(row_num(file, arg.line, row, field)?)),
        },
        _ => Err(len_err(file, arg)),
    }
}

fn len_err(file: &str, arg: &crate::ir::SeqArg) -> SeqError {
    SeqError::new(
        file,
        arg.line,
        1,
        format!("'{}' to fill()/px()/fit/percent()", arg.name),
    )
}

enum PaddingVal {
    All(f32),
    Xy(f32, f32),
}

enum MarginVal {
    All(f32),
    Xy(f32, f32),
}

fn margin_val(
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<MarginVal, SeqError> {
    match &arg.value {
        SeqValue::Num(n) => Ok(MarginVal::All(*n)),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(margin_err(file, arg)),
            Resolver::Host(b) => Ok(MarginVal::All(prop_num_env(file, arg.line, b, prop)?)),
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(margin_err(file, arg)),
            Resolver::Host(b) => Ok(MarginVal::All(env_num(file, arg.line, b, name)?)),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(margin_err(file, arg)),
            Resolver::Host(_) => Ok(MarginVal::All(row_num(file, arg.line, row, field)?)),
        },
        SeqValue::Array(items) => {
            if items.len() == 2 {
                match (&items[0], &items[1]) {
                    (SeqValue::Num(x), SeqValue::Num(y)) => Ok(MarginVal::Xy(*x, *y)),
                    _ => Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "'margin' to liczba albo [x, y]",
                    )),
                }
            } else {
                Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    "'margin' to liczba albo [x, y]",
                ))
            }
        }
        _ => Err(SeqError::new(
            file,
            arg.line,
            1,
            "'margin' to liczba albo [x, y]",
        )),
    }
}

fn margin_err(file: &str, arg: &crate::ir::SeqArg) -> SeqError {
    SeqError::new(file, arg.line, 1, "'margin' to liczba albo [x, y]")
}

fn overflow_val(file: &str, arg: &crate::ir::SeqArg) -> Result<Overflow, SeqError> {
    match ident(file, arg)?.as_str() {
        "visible" => Ok(Overflow::Visible),
        "hidden" => Ok(Overflow::Hidden),
        "scroll" => Ok(Overflow::Scroll),
        "auto" => Ok(Overflow::Auto),
        other => Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("overflow to visible/hidden/scroll/auto, nie '{other}'"),
        )),
    }
}

fn padding_val(
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<PaddingVal, SeqError> {
    match &arg.value {
        SeqValue::Num(n) => Ok(PaddingVal::All(*n)),
        SeqValue::Ident(prop) | SeqValue::Dynamic(prop) => match res {
            Resolver::Static => Err(padding_err(file, arg)),
            Resolver::Host(b) => Ok(PaddingVal::All(prop_num_env(file, arg.line, b, prop)?)),
        },
        SeqValue::Env(name) => match res {
            Resolver::Static => Err(padding_err(file, arg)),
            Resolver::Host(b) => Ok(PaddingVal::All(env_num(file, arg.line, b, name)?)),
        },
        SeqValue::ItemField(field) => match res {
            Resolver::Static => Err(padding_err(file, arg)),
            Resolver::Host(_) => Ok(PaddingVal::All(row_num(file, arg.line, row, field)?)),
        },
        SeqValue::Array(items) => {
            if items.len() == 2 {
                match (&items[0], &items[1]) {
                    (SeqValue::Num(x), SeqValue::Num(y)) => Ok(PaddingVal::Xy(*x, *y)),
                    _ => Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "'padding' to liczba albo [x, y]",
                    )),
                }
            } else {
                Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    "'padding' to liczba albo [x, y]",
                ))
            }
        }
        _ => Err(SeqError::new(
            file,
            arg.line,
            1,
            "'padding' to liczba albo [x, y]",
        )),
    }
}

fn padding_err(file: &str, arg: &crate::ir::SeqArg) -> SeqError {
    SeqError::new(file, arg.line, 1, "'padding' to liczba albo [x, y]")
}

fn apply_padding<V>(
    mut v: V,
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<V, SeqError>
where
    V: Padded,
{
    match padding_val(file, arg, res, row)? {
        PaddingVal::All(n) => v = v.set_pad(n),
        PaddingVal::Xy(x, y) => v = v.set_pad_xy(x, y),
    }
    Ok(v)
}

trait Padded {
    fn set_pad(self, n: f32) -> Self;
    fn set_pad_xy(self, x: f32, y: f32) -> Self;
}

impl Padded for Container {
    fn set_pad(self, n: f32) -> Self {
        self.padding(n)
    }
    fn set_pad_xy(self, x: f32, y: f32) -> Self {
        self.padding_xy(x, y)
    }
}

impl Padded for Panel {
    fn set_pad(self, n: f32) -> Self {
        self.padding(n)
    }
    fn set_pad_xy(self, x: f32, y: f32) -> Self {
        self.padding_xy(x, y)
    }
}

fn apply_container_margin(
    v: Container,
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Container, SeqError> {
    match margin_val(file, arg, res, row)? {
        MarginVal::All(n) => Ok(v.margin(n)),
        MarginVal::Xy(x, y) => Ok(v.margin_xy(x, y)),
    }
}

fn apply_panel_margin(
    v: Panel,
    file: &str,
    arg: &crate::ir::SeqArg,
    res: &Resolver<'_>,
    row: Option<&SeqRowData>,
) -> Result<Panel, SeqError> {
    match margin_val(file, arg, res, row)? {
        MarginVal::All(n) => Ok(v.margin(n)),
        MarginVal::Xy(x, y) => Ok(v.margin_xy(x, y)),
    }
}

fn color(file: &str, theme: &Theme, arg: &crate::ir::SeqArg) -> Result<Color, SeqError> {
    match &arg.value {
        SeqValue::Color(hex) => hex_color(file, arg.line, hex),
        SeqValue::Theme(field) => theme_color(file, arg.line, theme, field),
        _ => Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("'{}' to #RRGGBB albo theme.pole", arg.name),
        )),
    }
}

fn hex_color(file: &str, line: usize, hex: &str) -> Result<Color, SeqError> {
    match u32::from_str_radix(hex, 16) {
        Ok(v) if hex.len() == 6 => Ok(Color::hex(v)),
        Ok(v) => Ok(Color::hexa(v)),
        Err(_) => Err(SeqError::new(file, line, 1, "wrong color hex")),
    }
}

fn theme_color(file: &str, line: usize, theme: &Theme, field: &str) -> Result<Color, SeqError> {
    match field {
        "bg" | "background" => Ok(theme.background),
        "surface" => Ok(theme.surface),
        "border" => Ok(theme.border),
        "ink" | "text" => Ok(theme.text),
        "muted" | "text_secondary" => Ok(theme.text_secondary),
        "accent" => Ok(theme.accent),
        "disabled" => Ok(theme.disabled),
        _ => Err(SeqError::new(
            file,
            line,
            1,
            format!("nieznane pole theme '{field}' (bg/surface/border/ink/muted/accent/disabled)"),
        )),
    }
}

fn border_pair(
    file: &str,
    theme: &Theme,
    arg: &crate::ir::SeqArg,
) -> Result<(f32, Color), SeqError> {
    match &arg.value {
        SeqValue::Array(items) if items.len() == 2 => {
            let w = match &items[0] {
                SeqValue::Num(n) => *n,
                _ => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "'border' is [width, color]",
                    ));
                }
            };
            let c = match &items[1] {
                SeqValue::Color(hex) => hex_color(file, arg.line, hex)?,
                SeqValue::Theme(f) => theme_color(file, arg.line, theme, f)?,
                _ => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "'border' is [width, color]",
                    ));
                }
            };
            Ok((w, c))
        }
        _ => Err(SeqError::new(
            file,
            arg.line,
            1,
            "'border' is [width, color]",
        )),
    }
}

fn align_val(file: &str, arg: &crate::ir::SeqArg) -> Result<Align, SeqError> {
    match ident(file, arg)?.as_str() {
        "center" => Ok(Align::Center),
        "start" => Ok(Align::Start),
        "end" => Ok(Align::End),
        "stretch" => Ok(Align::Stretch),
        "baseline" => Ok(Align::Baseline),
        other => Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("align to center/start/end/stretch/baseline, nie '{other}'"),
        )),
    }
}

fn text_valign(file: &str, arg: &crate::ir::SeqArg) -> Result<TextVAlign, SeqError> {
    match ident(file, arg)?.as_str() {
        "top" | "start" => Ok(TextVAlign::Top),
        "center" => Ok(TextVAlign::Center),
        "bottom" | "end" => Ok(TextVAlign::Bottom),
        "baseline" => Ok(TextVAlign::Baseline),
        other => Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("align_y to top/center/bottom/baseline, nie '{other}'"),
        )),
    }
}

fn justify_val(file: &str, arg: &crate::ir::SeqArg) -> Result<Justify, SeqError> {
    match ident(file, arg)?.as_str() {
        "start" => Ok(Justify::Start),
        "center" => Ok(Justify::Center),
        "end" => Ok(Justify::End),
        "between" => Ok(Justify::SpaceBetween),
        other => Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("justify to start/center/end/between, nie '{other}'"),
        )),
    }
}

fn button_style_of(
    file: &str,
    styles: &BuiltStyles,
    node: &SeqNode,
) -> Result<Option<ButtonStyle>, SeqError> {
    match find_arg(node, "style") {
        None => Ok(None),
        Some(arg) => match &arg.value {
            SeqValue::Ident(name) => {
                if let Some(preset) = button_preset(&Some(name.clone())) {
                    return Ok(Some(preset));
                }
                styles.button_style(name).map(Some).ok_or_else(|| {
                    SeqError::new(file, arg.line, 1, format!("nieznany styl '{name}'"))
                })
            }
            _ => Err(SeqError::new(file, arg.line, 1, "style: to nazwa stylu")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_style_block_builds_text_style() {
        let app = crate::parser::parse(
            "S.seq",
            "app A {\n  title: \"A\"\n  size: (400, 300)\n  style Title: Text {\n    size: 24\n    color: #FF0000\n    bold: true\n  }\n  window M {\n    Text(\"Hi\", style: Title)\n  }\n}\n",
        )
        .unwrap();
        let styles = build_styles("S.seq", &Theme::dark(), &app.styles).unwrap();
        let style = styles.text_style("Title").unwrap();
        assert_eq!(style.size, Some(24.0));
        assert_eq!(style.color, Some(Color::parse_hex("#FF0000").unwrap()));
        assert_eq!(style.weight, Some(700));
    }

    #[test]
    fn text_style_line_height_is_lowered() {
        let app = crate::parser::parse(
            "S.seq",
            "app A {\n  title: \"A\"\n  size: (400, 300)\n  style Title: Text {\n    line_height: 1.6\n  }\n  window M {\n    Text(\"Hi\", style: Title)\n  }\n}\n",
        )
        .unwrap();
        let styles = build_styles("S.seq", &Theme::dark(), &app.styles).unwrap();
        let style = styles.text_style("Title").unwrap();
        assert_eq!(style.line_height, Some(1.6));
    }

    #[test]
    fn text_arguments_accept_layout_options() {
        let app = crate::parser::parse(
            "S.seq",
            "app A {\n  title: \"A\"\n  size: (400, 300)\n  window M {\n    Text(\"one two three\", wrap: true, line_height: 1.5, align_y: baseline, max_lines: 2, font_scale: 1.2)\n  }\n}\n",
        )
        .unwrap();
        check_decls("S.seq", &app, &HostBindings::default()).unwrap();
        lower("S.seq", &app).unwrap();
    }
}
