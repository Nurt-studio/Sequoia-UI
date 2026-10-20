use crate::error::SeqError;
use crate::ir::{
    BinOp, ComponentDef, MatchPat, SeqApp, SeqExpr, SeqExprKind, SeqFn, SeqNode, SeqStmt, SeqValue,
    StyleDecl, UnOp, Visibility,
};

const COMPONENTS: &[&str] = &[
    "Column",
    "Row",
    "Container",
    "Panel",
    "ScrollView",
    "Text",
    "Button",
    "Spacer",
    "Icon",
    "Image",
    "List",
    "VirtualList",
    "Canvas",
    "Stack",
    "Grid",
    "Wrap",
    "SplitPanel",
    "Checkbox",
    "Switch",
    "TextField",
    "TextArea",
    "Field",
    "NumberField",
    "DateField",
    "MultiSelect",
    "Slider",
    "DragValue",
    "Dropdown",
    "Progress",
    "If",
    "Width",
    "Height",
    "Opacity",
    "Scale",
    "Rotate",
    "Translate",
    "Pulse",
    "Shake",
    "Async",
    "Loading",
    "Data",
    "Error",
    "Idle",
    "Cancelled",
    "Effect",
    "Rect",
    "Circle",
    "Ellipse",
    "Ring",
    "Line",
    "Arrow",
    "Capsule",
    "Polygon",
    "Polyline",
    "Spline",
    "Star",
    "Ngon",
    "Cross",
    "Text2d",
    "Svg",
    "Dots",
    "Axes",
    "Frame",
    "Marquee",
    "Handle",
    "Image2d",
    "Clip",
    "Group",
];

const SHAPES: &[&str] = &[
    "Grid", "Rect", "Circle", "Ellipse", "Ring", "Line", "Arrow", "Capsule", "Polygon", "Polyline",
    "Spline", "Star", "Ngon", "Cross", "Text2d", "Svg", "Dots", "Axes", "Frame", "Marquee",
    "Handle", "Image2d", "Clip", "Group",
];

const LAYOUT_PROPS: &[&str] = &[
    "width",
    "height",
    "padding",
    "padding_x",
    "padding_y",
    "background",
    "flex",
];

pub fn check(file: &str, app: &SeqApp) -> Result<(), SeqError> {
    check_styles(file, app)?;
    let comps = check_components(file, &app.components, &app.styles)?;
    let fns = check_fns(file, app)?;
    check_anims(file, app)?;
    check_machines(file, app)?;
    check_springs(file, app)?;
    check_seq_calls(file, app, &fns)?;
    let ctx = CheckCtx {
        styles: &app.styles,
        components: &app.components,
        comps: &comps,
        in_comp: false,
    };
    for w in &app.windows {
        for n in &w.nodes {
            check_node(file, n, None, false, &ctx)?;
        }
    }
    Ok(())
}

const COMP_PARAM_TYPES: &[&str] = &["String", "Bool", "Int", "Float", "Color"];

type CompTable = std::collections::HashMap<String, usize>;

struct CheckCtx<'a> {
    styles: &'a [StyleDecl],
    components: &'a [ComponentDef],
    comps: &'a CompTable,

    in_comp: bool,
}

fn check_components(
    file: &str,
    components: &[ComponentDef],
    styles: &[StyleDecl],
) -> Result<CompTable, SeqError> {
    let mut comps: CompTable = std::collections::HashMap::new();
    for (i, c) in components.iter().enumerate() {
        if let Some(&prev) = comps.get(&c.name) {
            let first = &components[prev];
            return Err(SeqError::new(
                &c.file,
                c.line,
                1,
                format!(
                    "komponent '{}' zdefiniowany dwa razy (pierwszy w {}:{})",
                    c.name, first.file, first.line
                ),
            ));
        }
        if COMPONENTS.contains(&c.name.as_str()) {
            return Err(SeqError::new(
                &c.file,
                c.line,
                1,
                format!("komponent '{}' koliduje z wbudowanym", c.name),
            ));
        }
        if !c.name.starts_with(|ch: char| ch.is_ascii_uppercase()) {
            return Err(SeqError::new(
                &c.file,
                c.line,
                1,
                format!(
                    "component name '{name}' capitalized (like Column)",
                    name = c.name
                ),
            ));
        }
        let mut seen: Vec<&str> = Vec::new();
        for p in &c.params {
            if seen.contains(&p.name.as_str()) {
                return Err(SeqError::new(
                    &c.file,
                    p.line,
                    1,
                    format!("parametr '{}' dwa razy w {}", p.name, c.name),
                ));
            }
            seen.push(p.name.as_str());
            if !COMP_PARAM_TYPES.contains(&p.ty.as_str()) {
                return Err(SeqError::new(
                    &c.file,
                    p.line,
                    1,
                    format!(
                        "nieznany typ parametru '{}' (String/Bool/Int/Float/Color)",
                        p.ty
                    ),
                ));
            }
        }
        comps.insert(c.name.clone(), i);
    }
    for c in components {
        if count_placeholders(&c.body) > 1 {
            return Err(SeqError::new(
                &c.file,
                c.line,
                1,
                format!("jedno `children` w ciele {}", c.name),
            ));
        }
    }

    let body_ctx = CheckCtx {
        styles,
        components,
        comps: &comps,
        in_comp: true,
    };
    for c in components {
        for n in &c.body {
            check_node(&c.file, n, None, false, &body_ctx)?;
        }
    }

    let mut edges: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for c in components {
        edges.insert(c.name.clone(), called_components(&c.body, &comps));
    }
    if let Some(cycle) = find_cycle(&edges) {
        let start = cycle.first().cloned().unwrap_or_default();
        let line = components
            .iter()
            .find(|c| c.name == start)
            .map(|c| c.line)
            .unwrap_or(1);
        let cycle_file = components
            .iter()
            .find(|c| c.name == start)
            .map_or(file, |c| c.file.as_str());
        let mut path = cycle.join(" -> ");
        path.push_str(&format!(" -> {start}"));
        return Err(SeqError::new(
            cycle_file,
            line,
            1,
            format!("component recursion: {path}"),
        ));
    }
    Ok(comps)
}

fn count_placeholders(nodes: &[SeqNode]) -> usize {
    nodes
        .iter()
        .map(|n| (usize::from(n.kind == "children")) + count_placeholders(&n.children))
        .sum()
}

fn called_components(nodes: &[SeqNode], comps: &CompTable) -> Vec<String> {
    let mut out = Vec::new();
    for n in nodes {
        if comps.contains_key(n.kind.as_str()) && !out.contains(&n.kind) {
            out.push(n.kind.clone());
        }
        for sub in called_components(&n.children, comps) {
            if !out.contains(&sub) {
                out.push(sub);
            }
        }
    }
    out
}

fn find_cycle(edges: &std::collections::HashMap<String, Vec<String>>) -> Option<Vec<String>> {
    fn visit(
        node: &str,
        edges: &std::collections::HashMap<String, Vec<String>>,
        state: &mut std::collections::HashMap<String, u8>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        state.insert(node.to_string(), 1);
        path.push(node.to_string());
        if let Some(next) = edges.get(node) {
            for n in next {
                match state.get(n.as_str()).copied().unwrap_or(0) {
                    1 => {
                        let pos = path.iter().position(|s| s == n).unwrap_or(0);
                        return Some(path[pos..].to_vec());
                    }
                    0 => {
                        if let Some(c) = visit(n, edges, state, path) {
                            return Some(c);
                        }
                    }
                    _ => {}
                }
            }
        }
        path.pop();
        state.insert(node.to_string(), 2);
        None
    }
    let mut names: Vec<&String> = edges.keys().collect();
    names.sort();
    let mut state: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
    for start in names {
        if state.get(start).copied().unwrap_or(0) == 0 {
            let mut path = Vec::new();
            if let Some(c) = visit(start, edges, &mut state, &mut path) {
                return Some(c);
            }
        }
    }
    None
}

fn has_placeholder(def: &ComponentDef) -> bool {
    count_placeholders(&def.body) > 0
}

const BOX_STYLE_FIELDS: &[&str] = &[
    "background",
    "radius",
    "border",
    "border_left",
    "border_right",
    "border_top",
    "border_bottom",
    "border_x",
    "border_y",
    "shadow",
    "inset_shadow",
];

const BUTTON_STYLE_FIELDS: &[&str] = &[
    "background",
    "hover",
    "pressed",
    "disabled",
    "text",
    "text_disabled",
    "font_size",
    "font_weight",
    "radius",
    "padding",
    "border",
    "border_width",
    "border_color",
];

const TEXT_STYLE_FIELDS: &[&str] = &[
    "size", "color", "bold", "semi", "medium", "normal", "light", "black",
];

const TEXT_WEIGHT_FLAGS: &[&str] = &["bold", "semi", "medium", "normal", "light", "black"];

const BUTTON_PRESETS: &[&str] = &[
    "primary",
    "secondary",
    "danger",
    "ghost",
    "dark_filled",
    "outline_rose",
];

fn check_styles(file: &str, app: &SeqApp) -> Result<(), SeqError> {
    for (i, s) in app.styles.iter().enumerate() {
        if app.styles[..i].iter().any(|o| o.name == s.name) {
            return Err(SeqError::new(
                file,
                s.line,
                1,
                format!("styl '{}' zdefiniowany dwa razy", s.name),
            ));
        }
        let fields: &[&str] = match s.target.as_str() {
            "Box" => BOX_STYLE_FIELDS,
            "Button" => BUTTON_STYLE_FIELDS,
            "Text" => TEXT_STYLE_FIELDS,
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
        };
        let mut weights = 0;
        for a in &s.args {
            if !fields.contains(&a.name.as_str()) {
                return Err(SeqError::new(
                    file,
                    a.line,
                    1,
                    format!(
                        "nieznane pole '{}' w style {}:{} (dozwolone: {})",
                        a.name,
                        s.name,
                        s.target,
                        fields.join(", ")
                    ),
                ));
            }
            if s.target == "Text" && TEXT_WEIGHT_FLAGS.contains(&a.name.as_str()) {
                weights += 1;
            }
        }
        if s.target == "Text" && weights > 1 {
            return Err(SeqError::new(
                file,
                s.line,
                1,
                format!(
                    "style '{}': jedna flaga wagi (bold/semi/medium/normal/light/black)",
                    s.name
                ),
            ));
        }
        if let Some(base) = &s.base {
            if s.target == "Button" && BUTTON_PRESETS.contains(&base.as_str()) {
                continue;
            }
            let mut cur = base.as_str();
            let mut seen = vec![s.name.as_str()];
            loop {
                if seen.contains(&cur) {
                    return Err(SeqError::new(
                        file,
                        s.line,
                        1,
                        format!("style '{}': cykliczne extends", s.name),
                    ));
                }
                seen.push(cur);
                let Some(parent) = app.styles.iter().find(|o| o.name == cur) else {
                    return Err(SeqError::new(
                        file,
                        s.line,
                        1,
                        format!(
                            "style '{}': nieznana baza '{cur}' (preset albo styl)",
                            s.name
                        ),
                    ));
                };
                if parent.target != s.target {
                    return Err(SeqError::new(
                        file,
                        s.line,
                        1,
                        format!(
                            "style '{}': baza '{}' to {}, a styl to {}",
                            s.name, cur, parent.target, s.target
                        ),
                    ));
                }
                if let Some(next) = &parent.base {
                    if s.target == "Button" && BUTTON_PRESETS.contains(&next.as_str()) {
                        break;
                    }
                    cur = next;
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn find_style<'s>(styles: &'s [StyleDecl], name: &str) -> Option<&'s StyleDecl> {
    styles.iter().find(|s| s.name == name)
}

fn check_node(
    file: &str,
    node: &SeqNode,
    parent: Option<&str>,
    in_canvas: bool,
    ctx: &CheckCtx<'_>,
) -> Result<(), SeqError> {
    let in_canvas = in_canvas || matches!(parent, Some("Canvas" | "Clip" | "Group"));
    if node.kind == "children" {
        if !ctx.in_comp {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                "znacznik `children` tylko w ciele component".to_string(),
            ));
        }
        if in_canvas {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                "inside Canvas is shapes or List, not 'children' (UI views belong outside Canvas)"
                    .to_string(),
            ));
        }
        if node.label.is_some() || !node.args.is_empty() || !node.children.is_empty() {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                "`children` is a bare tag (no props or children)".to_string(),
            ));
        }
        return Ok(());
    }
    if let Some(&idx) = ctx.comps.get(node.kind.as_str()) {
        if in_canvas {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                format!(
                    "inside Canvas is shapes or List, not '{}' (UI views belong outside Canvas)",
                    node.kind
                ),
            ));
        }
        check_call(file, node, ctx, idx)?;
        return Ok(());
    }
    if !COMPONENTS.contains(&node.kind.as_str()) {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!(
                "unknown component '{}' (Column/Row/Container/Panel/ScrollView/Text/Button/Spacer/Icon/Image/List/VirtualList/Canvas/Stack/Grid/Wrap/SplitPanel/Checkbox/Switch/TextFiel d/TextArea/Field/NumberField/DateField/MultiSelect/Slider/DragValue/Dropdown/Progress/If/Opacity/Scale/Rotate/Translate/Pulse/Shake/Async/Effect + shapes)",
                node.kind
            ),
        ));
    }

    if matches!(
        node.kind.as_str(),
        "Loading" | "Data" | "Error" | "Idle" | "Cancelled"
    ) && parent != Some("Async")
    {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!("block '{}' only inside Async", node.kind),
        ));
    }

    if in_canvas && node.kind != "List" && !SHAPES.contains(&node.kind.as_str()) {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!(
                "inside Canvas is shapes or List, not '{}' (UI views belong outside Canvas)",
                node.kind
            ),
        ));
    }
    if !in_canvas
        && node.kind != "Canvas"
        && node.kind != "Grid"
        && SHAPES.contains(&node.kind.as_str())
    {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!("shape '{}' must lie in Canvas", node.kind),
        ));
    }
    if node.label.is_some() && node.kind != "Text" && node.kind != "Button" && node.kind != "Image"
    {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!(
                "etykieta pozycyjna tylko w Text/Button/Image, nie w {}",
                node.kind
            ),
        ));
    }

    if node.kind == "Text" {
        let weights = node
            .args
            .iter()
            .filter(|a| {
                matches!(
                    a.name.as_str(),
                    "bold" | "semi" | "medium" | "normal" | "light" | "black"
                )
            })
            .count();
        if weights > 1 {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                "Text: jedna flaga wagi (bold/semi/medium/normal/light/black)".to_string(),
            ));
        }
    }
    for arg in &node.args {
        check_arg(file, node, arg, parent, in_canvas, ctx.styles)?;
    }
    for child in &node.children {
        check_node(file, child, Some(node.kind.as_str()), in_canvas, ctx)?;
    }
    Ok(())
}

fn check_call(file: &str, node: &SeqNode, ctx: &CheckCtx<'_>, idx: usize) -> Result<(), SeqError> {
    let def = &ctx.components[idx];
    if def.visibility == Visibility::Priv && def.file != file {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!(
                "komponent '{}' jest prywatny (zdefiniowany w {})",
                def.name, def.file
            ),
        ));
    }
    if node.label.is_some() {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!(
                "parametry {} tylko nazwane (bez etykiety pozycyjnej)",
                def.name
            ),
        ));
    }
    let mut seen: Vec<&str> = Vec::new();
    for arg in &node.args {
        if arg.name == "flex" {
            continue;
        }
        let Some(param) = def.params.iter().find(|p| p.name == arg.name) else {
            let want = if def.params.is_empty() {
                "(no parameters)".to_string()
            } else {
                def.params
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!(
                    "nieznany parametr '{}' w {} (oczekiwano: {want})",
                    arg.name, def.name
                ),
            ));
        };
        if seen.contains(&arg.name.as_str()) {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("parametr '{}' dwa razy w {}", arg.name, def.name),
            ));
        }
        seen.push(arg.name.as_str());
        check_comp_value(file, &def.name, param, arg)?;
    }
    for p in &def.params {
        if !seen.contains(&p.name.as_str()) {
            return Err(SeqError::new(
                file,
                node.line,
                1,
                format!("brakuje parametru '{}' w {}", p.name, def.name),
            ));
        }
    }
    if !node.children.is_empty() && !has_placeholder(def) {
        return Err(SeqError::new(
            file,
            node.line,
            1,
            format!(
                "{} nie przyjmuje dzieci (brak `children` w ciele)",
                def.name
            ),
        ));
    }
    for child in &node.children {
        check_node(file, child, Some(node.kind.as_str()), false, ctx)?;
    }
    Ok(())
}

fn check_comp_value(
    file: &str,
    comp: &str,
    param: &crate::ir::CompParam,
    arg: &crate::ir::SeqArg,
) -> Result<(), SeqError> {
    let ok = match &arg.value {
        SeqValue::Str(_) => param.ty == "String",
        SeqValue::Bool(_) => param.ty == "Bool",
        SeqValue::Num(n) => match param.ty.as_str() {
            "Float" => n.is_finite(),
            "Int" => n.is_finite() && n.fract() == 0.0,
            _ => false,
        },
        SeqValue::Color(_) | SeqValue::Theme(_) => param.ty == "Color",
        SeqValue::Ident(_) | SeqValue::Dynamic(_) | SeqValue::Env(_) | SeqValue::ItemField(_) => {
            true
        }
        SeqValue::Length(_) | SeqValue::Tuple(_) | SeqValue::Array(_) => false,
        SeqValue::Callback { .. } => false,

        SeqValue::CallSeq { .. } => true,
    };
    if !ok {
        return Err(SeqError::new(
            file,
            arg.line,
            1,
            format!(
                "bad type of parameter '{}' in {comp} (expected {})",
                param.name, param.ty
            ),
        ));
    }
    Ok(())
}

fn check_arg(
    file: &str,
    node: &SeqNode,
    arg: &crate::ir::SeqArg,
    parent: Option<&str>,
    in_canvas: bool,
    styles: &[StyleDecl],
) -> Result<(), SeqError> {
    let name = arg.name.as_str();

    if matches!(
        name,
        "indent"
            | "indent_x"
            | "indent_y"
            | "indent_left"
            | "indent_right"
            | "indent_top"
            | "indent_bottom"
    ) && matches!(
        node.kind.as_str(),
        "Column"
            | "Row"
            | "Container"
            | "Panel"
            | "Stack"
            | "Grid"
            | "Wrap"
            | "ScrollView"
            | "Text"
            | "Button"
            | "Spacer"
            | "Icon"
            | "Image"
            | "List"
            | "VirtualList"
            | "Canvas"
            | "SplitPanel"
            | "Checkbox"
            | "Switch"
            | "TextField"
            | "Slider"
            | "Dropdown"
            | "Progress"
            | "If"
            | "Async"
            | "Effect"
    ) && !in_canvas
    {
        return Ok(());
    }

    if name == "hit"
        && matches!(
            node.kind.as_str(),
            "Column"
                | "Row"
                | "Container"
                | "Panel"
                | "Stack"
                | "Wrap"
                | "SplitPanel"
                | "Text"
                | "Icon"
                | "Spacer"
        )
        && !in_canvas
    {
        return Ok(());
    }
    if parent == Some("Stack") && matches!(name, "at" | "offset" | "z") {
        return Ok(());
    }
    if parent == Some("Grid") && matches!(name, "col" | "row" | "col_span" | "row_span") {
        return Ok(());
    }

    if node.kind == "Grid" && in_canvas {
        let allowed: &[&str] = &["step", "color", "visible"];
        if !allowed.contains(&name) {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!(
                    "unknown prop '{name}' in Grid (Canvas shape; allowed: {})",
                    allowed.join(", ")
                ),
            ));
        }
        return Ok(());
    }
    let allowed: &[&str] = match node.kind.as_str() {
        "Column" | "Row" => &[
            "width",
            "height",
            "padding",
            "padding_x",
            "padding_y",
            "background",
            "flex",
            "gap",
            "main_gap",
            "cross_gap",
            "wrap",
            "align_content",
            "align",
            "justify",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "overflow",
            "on_click",
            "hover",
        ],
        "Container" => &[
            "width",
            "height",
            "padding",
            "padding_x",
            "padding_y",
            "background",
            "flex",
            "radius",
            "border",
            "border_left",
            "border_right",
            "border_top",
            "border_bottom",
            "border_x",
            "border_y",
            "style",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "overflow",
            "on_click",
            "hover",
        ],
        "Panel" => &[
            "width",
            "height",
            "padding",
            "padding_x",
            "padding_y",
            "background",
            "flex",
            "radius",
            "border",
            "border_left",
            "border_right",
            "border_top",
            "border_bottom",
            "border_x",
            "border_y",
            "style",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "overflow",
        ],
        "Stack" => &[
            "width",
            "height",
            "padding",
            "padding_x",
            "padding_y",
            "background",
            "flex",
            "radius",
            "border",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "overflow",
        ],
        "Grid" => &[
            "width",
            "height",
            "padding",
            "padding_x",
            "padding_y",
            "background",
            "flex",
            "radius",
            "border",
            "columns",
            "rows",
            "gap",
            "column_gap",
            "row_gap",
            "auto_flow",
            "align",
            "justify",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "overflow",
        ],
        "Wrap" => &[
            "width",
            "height",
            "padding",
            "padding_x",
            "padding_y",
            "background",
            "flex",
            "axis",
            "gap",
            "column_gap",
            "row_gap",
            "align",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "overflow",
        ],
        "ScrollView" => &["axis"],
        "Text" => &[
            "size",
            "color",
            "bold",
            "semi",
            "medium",
            "normal",
            "light",
            "black",
            "truncate",
            "wrap",
            "text",
            "dynamic",
            "t",
            "flex",
            "style",
            "width",
            "height",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Button" => &[
            "style",
            "enabled",
            "font_size",
            "on_click",
            "flex",
            "width",
            "height",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Spacer" => &[
            "flex",
            "width",
            "height",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
        ],
        "Icon" => &[
            "name",
            "size",
            "color",
            "flex",
            "width",
            "height",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Image" => &[
            "path",
            "width",
            "height",
            "flex",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "List" => &["items", "key"],
        "VirtualList" => &["items", "key", "item_height", "overscan"],
        "SplitPanel" => &[
            "value",
            "axis",
            "handle",
            "min_first",
            "min_second",
            "handle_color",
            "width",
            "height",
            "padding",
            "padding_x",
            "padding_y",
            "background",
            "flex",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "overflow",
        ],
        "If" => &["when", "enter", "exit"],
        "Width" => &[
            "value",
            "animate",
            "width",
            "height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Height" => &[
            "value",
            "animate",
            "width",
            "height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Opacity" => &[
            "value",
            "animate",
            "width",
            "height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Scale" => &[
            "value",
            "animate",
            "width",
            "height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Rotate" => &[
            "value",
            "animate",
            "width",
            "height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Translate" => &[
            "x",
            "y",
            "animate",
            "width",
            "height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Pulse" => &[
            "min",
            "period",
            "width",
            "height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Shake" => &[
            "amplitude",
            "period",
            "duration",
            "delay",
            "width",
            "height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
        ],
        "Async" => &["source"],
        "Loading" | "Data" | "Error" | "Idle" | "Cancelled" => &[],
        "Effect" => &["track", "on_run"],
        "Canvas" => &[
            "width",
            "height",
            "flex",
            "background",
            "transparent",
            "cursor",
            "focusable",
            "grid",
            "grid_color",
            "axes",
            "axes_color",
            "clip",
            "allow_overflow",
            "camera",
            "capture",
            "on_click_at",
            "on_drag",
            "on_wheel",
            "on_press",
            "on_pinch",
            "tick",
            "speed",
            "running",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
        ],
        "Checkbox" | "Switch" => &[
            "checked",
            "label",
            "font_size",
            "enabled",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "on_toggle",
        ],
        "TextField" => &[
            "value",
            "placeholder",
            "font_size",
            "width",
            "height",
            "min_width",
            "max_width",
            "max_height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "on_submit",
        ],
        "TextArea" => &[
            "value",
            "placeholder",
            "max_length",
            "min_lines",
            "max_lines",
            "font_size",
            "width",
            "height",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "on_edit_end",
        ],
        "Field" => &["value", "label", "placeholder"],
        "NumberField" => &["value", "label", "min", "max", "step", "precision"],
        "DateField" => &["value", "label"],
        "MultiSelect" => &["options", "label", "min_selected"],
        "Slider" => &[
            "value",
            "min",
            "max",
            "width",
            "height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "on_change",
        ],
        "DragValue" => &[
            "value",
            "min",
            "max",
            "step",
            "precision",
            "width",
            "height",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "on_commit",
        ],
        "Dropdown" => &[
            "selected",
            "items",
            "placeholder",
            "font_size",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "enabled",
            "flex",
            "margin",
            "margin_x",
            "margin_y",
            "margin_left",
            "margin_right",
            "margin_top",
            "margin_bottom",
            "on_select",
        ],
        "Progress" => &["value", "height", "track", "fill", "flex"],
        "Rect" => &[
            "x", "y", "w", "h", "fill", "color", "radius", "visible", "name",
        ],
        "Circle" => &["c", "r", "fill", "color", "visible", "name", "pick_r"],
        "Ellipse" => &["c", "rx", "ry", "fill", "color", "visible", "name"],
        "Ring" => &["c", "r", "width", "color", "visible", "name"],
        "Line" => &["a", "b", "width", "color", "visible", "name"],
        "Arrow" => &["a", "b", "width", "head", "color", "visible", "name"],
        "Capsule" => &["a", "b", "r", "fill", "color", "visible", "name"],
        "Polygon" => &["points", "fill", "color", "visible", "name"],
        "Polyline" => &["points", "width", "color", "visible", "name"],
        "Spline" => &["points", "width", "color", "visible", "name"],
        "Star" => &[
            "c", "outer", "inner", "n", "rotation", "color", "visible", "name",
        ],
        "Ngon" => &["c", "r", "sides", "rotation", "color", "visible", "name"],
        "Cross" => &["c", "half", "width", "color", "visible", "name"],
        "Text2d" => &["str", "t", "at", "size", "color", "visible"],
        "Svg" => &["d", "fill", "color", "visible"],
        "Dots" => &["step", "color", "radius", "visible"],
        "Axes" => &["color", "width", "at", "visible"],
        "Frame" => &["rect", "width", "color", "visible"],
        "Marquee" => &["rect", "color", "visible"],
        "Handle" => &["at", "color", "r", "visible"],
        "Image2d" => &["src", "rect", "x", "y", "w", "h", "uv", "visible", "name"],
        "Clip" => &["rect", "radius", "visible"],
        "Group" => &["translate", "rotate", "scale", "pivot"],
        _ => LAYOUT_PROPS,
    };
    if !allowed.contains(&name) {
        return Err(SeqError::new(
            file,
            arg.line,
            1,
            format!(
                "nieznana propka '{name}' w {} (dozwolone: {})",
                node.kind,
                allowed.join(", ")
            ),
        ));
    }

    if name == "style" {
        let style_name = match &arg.value {
            crate::ir::SeqValue::Ident(n) => n.as_str(),
            _ => {
                return Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    "style: is a style name (or preset), not an expression".to_string(),
                ));
            }
        };

        let want_target = match node.kind.as_str() {
            "Container" | "Panel" => "Box",
            "Text" => "Text",
            _ => "Button",
        };
        if want_target == "Button" && BUTTON_PRESETS.contains(&style_name) {
            return Ok(());
        }
        match find_style(styles, style_name) {
            Some(s) if s.target == want_target => {}
            Some(s) => {
                return Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!(
                        "style '{style_name}' to {}, a {} wymaga {}",
                        s.target, node.kind, want_target
                    ),
                ));
            }
            None => {
                return Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    format!("nieznany styl '{style_name}' (brak bloku style)"),
                ));
            }
        }
        return Ok(());
    }

    if name == "flex" && in_canvas {
        return Err(SeqError::new(
            file,
            arg.line,
            1,
            "flex not supported for Canvas shapes".to_string(),
        ));
    }
    Ok(())
}

const BUILTIN_FNS: &[&str] = &[
    "starts_with",
    "strip_prefix",
    "parse_tag",
    "len",
    "clamp",
    "powf",
    "max",
    "min",
];

#[derive(Clone, Debug, PartialEq)]
enum Ty {
    Int,
    Float,
    Str,
    Bool,
    Tuple(Vec<Ty>),
    Option(Box<Ty>),
    Row,
    Any,
    Null,
}

impl Ty {
    fn from_sig(ty: &str) -> Option<Self> {
        let t = ty.trim();
        if let Some(inner) = t.strip_prefix("Option<").and_then(|s| s.strip_suffix('>')) {
            return Some(Self::Option(Box::new(Self::parse_sig(inner.trim())?)));
        }
        match t {
            "Int" => Some(Self::Int),
            "Float" => Some(Self::Float),
            "String" => Some(Self::Str),
            "Bool" => Some(Self::Bool),
            _ => None,
        }
    }

    fn parse_sig(ty: &str) -> Option<Self> {
        let t = ty.trim();
        if let Some(inner) = t.strip_prefix("Option<").and_then(|s| s.strip_suffix('>')) {
            return Some(Self::Option(Box::new(Self::parse_sig(inner.trim())?)));
        }
        if let Some(inner) = t.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            if inner.trim().is_empty() {
                return None;
            }
            let mut items = Vec::new();
            for part in inner.split(',') {
                items.push(Self::from_sig(part.trim())?);
            }
            return Some(Self::Tuple(items));
        }
        Self::from_sig(t)
    }

    fn name(&self) -> String {
        match self {
            Self::Int => "Int".to_string(),
            Self::Float => "Float".to_string(),
            Self::Str => "String".to_string(),
            Self::Bool => "Bool".to_string(),
            Self::Tuple(items) => format!(
                "({})",
                items.iter().map(Self::name).collect::<Vec<_>>().join(", ")
            ),
            Self::Option(inner) => format!("Option<{}>", inner.name()),
            Self::Row => "wiersz".to_string(),
            Self::Any => "value".to_string(),
            Self::Null => "no value".to_string(),
        }
    }

    fn is_number(&self) -> bool {
        matches!(self, Self::Int | Self::Float | Self::Any)
    }

    fn fits(&self, target: &Self) -> bool {
        match (self, target) {
            (_, Self::Any) | (Self::Any, _) => true,
            (Self::Int, Self::Int) | (Self::Int, Self::Float) | (Self::Float, Self::Float) => true,
            (Self::Str, Self::Str) | (Self::Bool, Self::Bool) | (Self::Row, Self::Row) => true,
            (Self::Tuple(a), Self::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.fits(y))
            }
            (Self::Option(a), Self::Option(b)) => a.fits(b),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
struct FnSig {
    params: Vec<Ty>,
    ret: Option<Ty>,
}

type FnTable = std::collections::HashMap<String, FnSig>;

struct LocalTy {
    ty: Ty,
    mutable: bool,
}

struct FnCtx<'a> {
    file: &'a str,
    fns: &'a FnTable,
    props: &'a std::collections::HashMap<&'a str, Ty>,
    states: &'a std::collections::HashMap<String, Ty>,
}

fn check_fns(file: &str, app: &SeqApp) -> Result<FnTable, SeqError> {
    let mut prop_tys: std::collections::HashMap<&str, Ty> = std::collections::HashMap::new();
    for d in &app.deferred {
        if d.kind == crate::ir::DeferredKind::Prop {
            let ty = d
                .sig
                .first()
                .and_then(|s| Ty::from_sig(s))
                .unwrap_or(Ty::Any);
            prop_tys.entry(d.name.as_str()).or_insert(ty);
        }
    }
    let mut states: std::collections::HashMap<String, Ty> = std::collections::HashMap::new();
    for s in &app.states {
        if states.contains_key(&s.name) {
            return Err(SeqError::new(
                file,
                s.line,
                1,
                format!("state '{}' zdefiniowany dwa razy", s.name),
            ));
        }
        let Some(decl) = Ty::from_sig(&s.ty) else {
            return Err(SeqError::new(
                file,
                s.line,
                1,
                format!("state '{}': typ Int/Float/String/Bool", s.name),
            ));
        };
        if prop_tys.contains_key(s.name.as_str()) {
            return Err(SeqError::new(
                file,
                s.line,
                1,
                format!("state '{}' koliduje z `prop {}`", s.name, s.name),
            ));
        }
        let init = match &s.init.kind {
            SeqExprKind::Int(_) => Ty::Int,
            SeqExprKind::Float(_) => Ty::Float,
            SeqExprKind::Str(_) => Ty::Str,
            SeqExprKind::Bool(_) => Ty::Bool,
            SeqExprKind::Var(name) if name == "None" => Ty::Option(Box::new(Ty::Any)),
            _ => {
                return Err(SeqError::new(
                    file,
                    s.line,
                    1,
                    format!("state '{}': initial value is literal", s.name),
                ));
            }
        };
        if !init.fits(&decl) {
            return Err(SeqError::new(
                file,
                s.line,
                1,
                format!(
                    "state '{}': initial value is {} and type {}",
                    s.name,
                    init.name(),
                    decl.name()
                ),
            ));
        }
        states.insert(s.name.clone(), decl);
    }
    let mut table: FnTable = std::collections::HashMap::new();
    for f in &app.fns {
        if table.contains_key(&f.name) {
            return Err(SeqError::new(
                file,
                f.line,
                1,
                format!("funkcja '{}' zdefiniowana dwa razy", f.name),
            ));
        }
        if !f
            .name
            .starts_with(|ch: char| ch.is_ascii_lowercase() || ch == '_')
        {
            return Err(SeqError::new(
                file,
                f.line,
                1,
                format!("function name '{}' in lower case (like connect)", f.name),
            ));
        }
        if BUILTIN_FNS.contains(&f.name.as_str()) {
            return Err(SeqError::new(
                file,
                f.line,
                1,
                format!("fn '{}' overrides built-in function", f.name),
            ));
        }
        if states.contains_key(&f.name) {
            return Err(SeqError::new(
                file,
                f.line,
                1,
                format!("fn '{}' koliduje ze `state {}`", f.name, f.name),
            ));
        }
        if prop_tys.contains_key(f.name.as_str()) {
            return Err(SeqError::new(
                file,
                f.line,
                1,
                format!("fn '{}' koliduje z `prop {}`", f.name, f.name),
            ));
        }
        let mut seen: Vec<&str> = Vec::new();
        let mut params = Vec::new();
        for p in &f.params {
            if seen.contains(&p.name.as_str()) {
                return Err(SeqError::new(
                    file,
                    p.line,
                    1,
                    format!("parametr '{}' dwa razy w fn {}", p.name, f.name),
                ));
            }
            seen.push(p.name.as_str());
            let Some(ty) = Ty::from_sig(&p.ty) else {
                return Err(SeqError::new(
                    file,
                    p.line,
                    1,
                    format!(
                        "fn {}: nieznany typ parametru '{}' (Int/Float/String/Bool)",
                        f.name, p.ty
                    ),
                ));
            };
            params.push(ty);
        }
        let ret = match &f.ret {
            None => None,
            Some(r) => {
                let Some(ty) = Ty::parse_sig(r) else {
                    return Err(SeqError::new(
                        file,
                        f.line,
                        1,
                        format!(
                            "fn {}: nieznany typ zwracany '{r}' (Int/Float/String/Bool/(A, B))",
                            f.name
                        ),
                    ));
                };
                Some(ty)
            }
        };
        table.insert(f.name.clone(), FnSig { params, ret });
    }
    for f in &app.fns {
        check_fn_body(file, f, &table, &prop_tys, &states)?;
    }
    Ok(table)
}

fn check_fn_body(
    file: &str,
    f: &SeqFn,
    fns: &FnTable,
    props: &std::collections::HashMap<&str, Ty>,
    states: &std::collections::HashMap<String, Ty>,
) -> Result<(), SeqError> {
    let ctx = FnCtx {
        file,
        fns,
        props,
        states,
    };
    let mut frame = std::collections::HashMap::new();
    for p in &f.params {
        let ty = Ty::from_sig(&p.ty).expect("parameter types checked above");
        frame.insert(
            p.name.clone(),
            LocalTy {
                ty,
                mutable: p.mutable,
            },
        );
    }
    let mut locals = vec![frame];
    let ret = f
        .ret
        .as_ref()
        .map(|r| Ty::parse_sig(r).expect("return types checked above"));
    let returns = check_stmts(&ctx, &mut locals, &f.body, ret.as_ref())?;
    if ret.is_some() && !returns {
        return Err(SeqError::new(
            file,
            f.line,
            1,
            format!("fn {}: each path must end with `return`", f.name),
        ));
    }
    Ok(())
}

fn check_stmts(
    ctx: &FnCtx<'_>,
    locals: &mut Vec<std::collections::HashMap<String, LocalTy>>,
    stmts: &[SeqStmt],
    ret: Option<&Ty>,
) -> Result<bool, SeqError> {
    let mut always = false;
    for stmt in stmts {
        if check_stmt(ctx, locals, stmt, ret)? {
            always = true;
        }
    }
    Ok(always)
}

fn check_scoped(
    ctx: &FnCtx<'_>,
    locals: &mut Vec<std::collections::HashMap<String, LocalTy>>,
    stmts: &[SeqStmt],
    ret: Option<&Ty>,
) -> Result<bool, SeqError> {
    locals.push(std::collections::HashMap::new());
    let out = check_stmts(ctx, locals, stmts, ret);
    locals.pop();
    out
}

#[allow(clippy::too_many_lines)]
fn check_stmt(
    ctx: &FnCtx<'_>,
    locals: &mut Vec<std::collections::HashMap<String, LocalTy>>,
    stmt: &SeqStmt,
    ret: Option<&Ty>,
) -> Result<bool, SeqError> {
    match stmt {
        SeqStmt::Let {
            name,
            mutable,
            value,
            line,
        } => {
            let ty = expr_ty(ctx, locals, value)?;
            if ty == Ty::Null {
                return Err(SeqError::new(
                    ctx.file,
                    *line,
                    1,
                    format!("'{name}': expression returns nothing (function without `->Type`?)"),
                ));
            }
            let frame = locals.last_mut().expect("reception");
            if frame.contains_key(name) {
                return Err(SeqError::new(
                    ctx.file,
                    *line,
                    1,
                    format!("lokalna '{name}' zdefiniowana dwa razy"),
                ));
            }
            frame.insert(
                name.clone(),
                LocalTy {
                    ty,
                    mutable: *mutable,
                },
            );
            Ok(false)
        }
        SeqStmt::Assign { name, value, line } => {
            let ty = expr_ty(ctx, locals, value)?;
            if ty == Ty::Null {
                return Err(SeqError::new(
                    ctx.file,
                    *line,
                    1,
                    format!("writing to '{name}': expression returns nothing"),
                ));
            }
            for frame in locals.iter_mut().rev() {
                if let Some(local) = frame.get_mut(name) {
                    if !local.mutable {
                        return Err(SeqError::new(
                            ctx.file,
                            *line,
                            1,
                            format!("'{name}' niemutowalna — `mut {name} = ...`"),
                        ));
                    }
                    if !ty.fits(&local.ty) {
                        return Err(SeqError::new(
                            ctx.file,
                            *line,
                            1,
                            format!(
                                "wrong write type to '{name}' (is {}, given {})",
                                local.ty.name(),
                                ty.name()
                            ),
                        ));
                    }
                    return Ok(false);
                }
            }
            if let Some(st) = ctx.states.get(name) {
                if !ty.fits(st) {
                    return Err(SeqError::new(
                        ctx.file,
                        *line,
                        1,
                        format!(
                            "wrong type writing to state '{name}' (is {}, given {})",
                            st.name(),
                            ty.name()
                        ),
                    ));
                }
                return Ok(false);
            }
            if let Some(pt) = ctx.props.get(name.as_str()) {
                if !ty.fits(pt) {
                    return Err(SeqError::new(
                        ctx.file,
                        *line,
                        1,
                        format!(
                            "wrong write type to '{name}' (prop {}, given {})",
                            pt.name(),
                            ty.name()
                        ),
                    ));
                }
                return Ok(false);
            }
            Err(SeqError::new(
                ctx.file,
                *line,
                1,
                format!("nieznana zmienna '{name}' (lokalna/state/prop)"),
            ))
        }
        SeqStmt::If {
            cond,
            then,
            elifs,
            els,
            line,
        } => {
            check_cond(ctx, locals, cond, "if")?;
            let mut all = check_scoped(ctx, locals, then, ret)?;
            for (c, body) in elifs {
                check_cond(ctx, locals, c, "else if")?;
                all = check_scoped(ctx, locals, body, ret)? && all;
            }
            match els {
                Some(body) => {
                    all = check_scoped(ctx, locals, body, ret)? && all;
                }
                None => all = false,
            }
            let _ = line;
            Ok(all)
        }
        SeqStmt::For {
            var, source, body, ..
        } => {
            let _ = source;
            locals.push(std::collections::HashMap::new());
            locals.last_mut().expect("reception").insert(
                var.clone(),
                LocalTy {
                    ty: Ty::Row,
                    mutable: false,
                },
            );
            let out = check_stmts(ctx, locals, body, ret);
            locals.pop();
            out?;
            Ok(false)
        }
        SeqStmt::Match { value, arms, line } => {
            let vty = expr_ty(ctx, locals, value)?;
            if !arms.iter().any(|a| matches!(a.pattern, MatchPat::Wildcard)) {
                return Err(SeqError::new(
                    ctx.file,
                    *line,
                    1,
                    "match requires branch `_`",
                ));
            }
            let mut all = true;
            for arm in arms {
                let ok = matches!(
                    (&arm.pattern, &vty),
                    (MatchPat::Str(_), Ty::Str | Ty::Any)
                        | (MatchPat::Int(_), Ty::Int | Ty::Any)
                        | (MatchPat::Bool(_), Ty::Bool | Ty::Any)
                        | (MatchPat::Wildcard, _)
                );
                if !ok {
                    let what = match &arm.pattern {
                        MatchPat::Str(_) => "tekst",
                        MatchPat::Int(_) => "liczba",
                        MatchPat::Bool(_) => "true/false",
                        MatchPat::Wildcard => "_",
                    };
                    return Err(SeqError::new(
                        ctx.file,
                        arm.line,
                        1,
                        format!("wzorzec '{what}' nie pasuje do {}", vty.name()),
                    ));
                }
                all = check_scoped(ctx, locals, &arm.body, ret)? && all;
            }
            Ok(all)
        }
        SeqStmt::Return { value, line } => {
            match (ret, value) {
                (None, None) => {}
                (None, Some(_)) => {
                    return Err(SeqError::new(
                        ctx.file,
                        *line,
                        1,
                        "funkcja bez `-> Typ` — tylko `return;`",
                    ));
                }
                (Some(rt), None) => {
                    return Err(SeqError::new(
                        ctx.file,
                        *line,
                        1,
                        format!("funkcja zwraca {} — `return expr;`", rt.name()),
                    ));
                }
                (Some(rt), Some(e)) => {
                    let ty = expr_ty(ctx, locals, e)?;
                    if ty == Ty::Null {
                        return Err(SeqError::new(
                            ctx.file,
                            e.line,
                            1,
                            "return: expression returns nothing",
                        ));
                    }
                    if !ty.fits(rt) {
                        return Err(SeqError::new(
                            ctx.file,
                            *line,
                            1,
                            format!("return: oczekiwano {}, jest {}", rt.name(), ty.name()),
                        ));
                    }
                }
            }
            Ok(true)
        }
        SeqStmt::Expr(e) => {
            expr_ty(ctx, locals, e)?;
            Ok(false)
        }
    }
}

fn check_cond(
    ctx: &FnCtx<'_>,
    locals: &mut Vec<std::collections::HashMap<String, LocalTy>>,
    cond: &SeqExpr,
    what: &str,
) -> Result<(), SeqError> {
    let ty = expr_ty(ctx, locals, cond)?;
    if matches!(ty, Ty::Bool | Ty::Any) {
        Ok(())
    } else {
        Err(SeqError::new(
            ctx.file,
            cond.line,
            1,
            format!("condition `{what}` must be Bool (is {})", ty.name()),
        ))
    }
}

fn expr_ty(
    ctx: &FnCtx<'_>,
    locals: &mut Vec<std::collections::HashMap<String, LocalTy>>,
    e: &SeqExpr,
) -> Result<Ty, SeqError> {
    match &e.kind {
        SeqExprKind::Int(_) => Ok(Ty::Int),
        SeqExprKind::Float(_) => Ok(Ty::Float),
        SeqExprKind::Str(_) => Ok(Ty::Str),
        SeqExprKind::Bool(_) => Ok(Ty::Bool),
        SeqExprKind::Var(name) => {
            if name == "None" {
                return Ok(Ty::Option(Box::new(Ty::Any)));
            }
            for frame in locals.iter().rev() {
                if let Some(local) = frame.get(name) {
                    return Ok(local.ty.clone());
                }
            }
            if let Some(st) = ctx.states.get(name) {
                return Ok(st.clone());
            }
            if let Some(pt) = ctx.props.get(name.as_str()) {
                return Ok(pt.clone());
            }
            Err(SeqError::new(
                ctx.file,
                e.line,
                1,
                format!("nieznana zmienna '{name}' (parametr/lokalna/state/prop)"),
            ))
        }
        SeqExprKind::Field { base, field } => {
            for frame in locals.iter().rev() {
                if let Some(local) = frame.get(base) {
                    match &local.ty {
                        Ty::Row => {
                            let _ = field;
                            return Ok(Ty::Any);
                        }
                        Ty::Tuple(items) => {
                            let Ok(i) = field.parse::<usize>() else {
                                return Err(SeqError::new(
                                    ctx.file,
                                    e.line,
                                    1,
                                    format!("pole krotki '{base}' to `.0`/`.1` (jest '.{field}')"),
                                ));
                            };
                            return items.get(i).cloned().ok_or_else(|| {
                                SeqError::new(
                                    ctx.file,
                                    e.line,
                                    1,
                                    format!(
                                        "tuple '{base}' has {} elements (index {i})",
                                        items.len()
                                    ),
                                )
                            });
                        }
                        other => {
                            return Err(SeqError::new(
                                ctx.file,
                                e.line,
                                1,
                                format!(
                                    "'{base}' to {}, nie wiersz `for` ani krotka",
                                    other.name()
                                ),
                            ));
                        }
                    }
                }
            }
            Err(SeqError::new(
                ctx.file,
                e.line,
                1,
                format!("field '{base}.{field}' requires a `for` loop variable"),
            ))
        }
        SeqExprKind::Tuple(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let ty = expr_ty(ctx, locals, item)?;
                if ty == Ty::Null {
                    return Err(SeqError::new(
                        ctx.file,
                        item.line,
                        1,
                        "element krotki nic nie zwraca",
                    ));
                }
                out.push(ty);
            }
            Ok(Ty::Tuple(out))
        }
        SeqExprKind::Unary { op, expr } => {
            let ty = expr_ty(ctx, locals, expr)?;
            match op {
                UnOp::Neg => match ty {
                    Ty::Int => Ok(Ty::Int),
                    Ty::Float => Ok(Ty::Float),
                    Ty::Any => Ok(Ty::Any),
                    _ => Err(SeqError::new(
                        ctx.file,
                        e.line,
                        1,
                        format!("`-` wymaga liczby (jest {})", ty.name()),
                    )),
                },
                UnOp::Not => match ty {
                    Ty::Bool | Ty::Any => Ok(Ty::Bool),
                    _ => Err(SeqError::new(
                        ctx.file,
                        e.line,
                        1,
                        format!("`!` wymaga Bool (jest {})", ty.name()),
                    )),
                },
            }
        }
        SeqExprKind::Binary { op, lhs, rhs } => binary_ty(ctx, locals, *op, lhs, rhs, e.line),
        SeqExprKind::Call { name, args } => {
            let mut arg_tys = Vec::with_capacity(args.len());
            for a in args {
                let ty = expr_ty(ctx, locals, a)?;
                if ty == Ty::Null {
                    return Err(SeqError::new(
                        ctx.file,
                        a.line,
                        1,
                        "argument nic nie zwraca (funkcja bez `-> Typ`?)",
                    ));
                }
                arg_tys.push(ty);
            }
            if let Some(ty) = builtin_ty(ctx, name, &arg_tys, args, e.line)? {
                return Ok(ty);
            }
            let Some(sig) = ctx.fns.get(name) else {
                return Err(SeqError::new(
                    ctx.file,
                    e.line,
                    1,
                    format!("nieznana funkcja '{name}'"),
                ));
            };
            if arg_tys.len() != sig.params.len() {
                return Err(SeqError::new(
                    ctx.file,
                    e.line,
                    1,
                    format!(
                        "fn {name}: expected {} arguments, given {}",
                        sig.params.len(),
                        arg_tys.len()
                    ),
                ));
            }
            for (i, (ty, p)) in arg_tys.iter().zip(&sig.params).enumerate() {
                if !ty.fits(p) {
                    return Err(SeqError::new(
                        ctx.file,
                        args[i].line,
                        1,
                        format!(
                            "fn {name}: argument {} to {}, a parametr oczekuje {}",
                            i + 1,
                            ty.name(),
                            p.name()
                        ),
                    ));
                }
            }
            Ok(sig.ret.clone().unwrap_or(Ty::Null))
        }
    }
}

fn builtin_ty(
    ctx: &FnCtx<'_>,
    name: &str,
    tys: &[Ty],
    args: &[SeqExpr],
    line: usize,
) -> Result<Option<Ty>, SeqError> {
    if name == "Some" {
        if tys.len() != 1 {
            return Err(SeqError::new(
                ctx.file,
                line,
                1,
                format!("Some: oczekiwano 1 argumentu, dano {}", tys.len()),
            ));
        }
        return Ok(Some(Ty::Option(Box::new(tys[0].clone()))));
    }
    let arity = |n: usize| -> Result<(), SeqError> {
        if tys.len() == n {
            Ok(())
        } else {
            Err(SeqError::new(
                ctx.file,
                line,
                1,
                format!("{name}: expected {n} arguments, given {}", tys.len()),
            ))
        }
    };
    let want_str = |i: usize| -> Result<(), SeqError> {
        if matches!(tys[i], Ty::Str | Ty::Any) {
            Ok(())
        } else {
            Err(SeqError::new(
                ctx.file,
                args[i].line,
                1,
                format!(
                    "{name}: argument {} must be String (is {})",
                    i + 1,
                    tys[i].name()
                ),
            ))
        }
    };
    let want_num = |i: usize| -> Result<(), SeqError> {
        if tys[i].is_number() {
            Ok(())
        } else {
            Err(SeqError::new(
                ctx.file,
                args[i].line,
                1,
                format!(
                    "{name}: argument {} must be a number (is {})",
                    i + 1,
                    tys[i].name()
                ),
            ))
        }
    };
    match name {
        "starts_with" => {
            arity(2)?;
            want_str(0)?;
            want_str(1)?;
            Ok(Some(Ty::Bool))
        }
        "strip_prefix" => {
            arity(2)?;
            want_str(0)?;
            want_str(1)?;
            Ok(Some(Ty::Str))
        }
        "parse_tag" => {
            arity(1)?;
            want_str(0)?;
            Ok(Some(Ty::Int))
        }
        "len" => {
            arity(1)?;
            want_str(0)?;
            Ok(Some(Ty::Int))
        }
        "clamp" | "powf" => {
            let n = if name == "clamp" { 3 } else { 2 };
            arity(n)?;
            for i in 0..n {
                want_num(i)?;
            }
            Ok(Some(Ty::Float))
        }
        "max" | "min" => {
            arity(2)?;
            want_num(0)?;
            want_num(1)?;
            if tys.iter().all(|t| *t == Ty::Int) {
                Ok(Some(Ty::Int))
            } else {
                Ok(Some(Ty::Float))
            }
        }
        _ => Ok(None),
    }
}

fn binary_ty(
    ctx: &FnCtx<'_>,
    locals: &mut Vec<std::collections::HashMap<String, LocalTy>>,
    op: BinOp,
    lhs: &SeqExpr,
    rhs: &SeqExpr,
    line: usize,
) -> Result<Ty, SeqError> {
    let a = expr_ty(ctx, locals, lhs)?;
    let b = expr_ty(ctx, locals, rhs)?;
    let numeric = |a: &Ty, b: &Ty| -> Option<Ty> {
        if a.is_number() && b.is_number() {
            if *a == Ty::Any || *b == Ty::Any {
                Some(Ty::Any)
            } else if *a == Ty::Int && *b == Ty::Int {
                Some(Ty::Int)
            } else {
                Some(Ty::Float)
            }
        } else {
            None
        }
    };
    let bad = |what: &str| -> SeqError {
        SeqError::new(
            ctx.file,
            line,
            1,
            format!("{what} (jest {} i {})", a.name(), b.name()),
        )
    };
    match op {
        BinOp::Add => {
            if a == Ty::Str || b == Ty::Str {
                if matches!(a, Ty::Str | Ty::Any) && matches!(b, Ty::Str | Ty::Any) {
                    return Ok(Ty::Str);
                }
                return Err(bad("`+` concatenates String + String"));
            }
            numeric(&a, &b).ok_or_else(|| bad("`+` wymaga liczb"))
        }
        BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
            let op_name = match op {
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                _ => "%",
            };
            numeric(&a, &b).ok_or_else(|| bad(&format!("`{op_name}` wymaga liczb")))
        }
        BinOp::Eq | BinOp::Ne => {
            let ok = (a.is_number() && b.is_number())
                || (a == Ty::Str && b == Ty::Str)
                || (a == Ty::Bool && b == Ty::Bool)
                || (matches!(a, Ty::Option(_)) && matches!(b, Ty::Option(_)))
                || a == Ty::Any
                || b == Ty::Any;
            if ok {
                Ok(Ty::Bool)
            } else {
                Err(bad("`==`/`!=` requires compatible types"))
            }
        }
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            let ok = (a.is_number() && b.is_number()) || (a == Ty::Str && b == Ty::Str);
            if ok {
                Ok(Ty::Bool)
            } else {
                Err(bad("comparison requires numbers (or String)"))
            }
        }
        BinOp::And | BinOp::Or => {
            let ok = matches!(a, Ty::Bool | Ty::Any) && matches!(b, Ty::Bool | Ty::Any);
            if ok {
                Ok(Ty::Bool)
            } else {
                Err(bad("`&&`/`||` wymaga Bool"))
            }
        }
    }
}

fn live_callback_ty(attr: &str) -> Option<Ty> {
    match attr {
        "on_toggle" => Some(Ty::Bool),
        "on_submit" | "on_edit_end" => Some(Ty::Str),
        "on_commit" | "on_change" => Some(Ty::Float),
        "on_select" => Some(Ty::Int),
        _ => None,
    }
}

const GESTURE_CALLBACKS: &[&str] = &["on_click_at", "on_press", "on_drag"];

const PLAIN_CALLBACKS: &[&str] = &["on_click", "on_run"];

fn check_seq_calls(file: &str, app: &SeqApp, fns: &FnTable) -> Result<(), SeqError> {
    for w in &app.windows {
        for n in &w.nodes {
            check_calls_node(file, n, fns)?;
        }
    }
    for c in &app.components {
        for n in &c.body {
            check_calls_node(&c.file, n, fns)?;
        }
    }
    for s in &app.styles {
        for arg in &s.args {
            if contains_callseq(&arg.value) {
                return Err(SeqError::new(
                    file,
                    arg.line,
                    1,
                    "calling `.seq` function only in widget callbacks",
                ));
            }
        }
    }
    Ok(())
}

fn check_calls_node(file: &str, node: &SeqNode, fns: &FnTable) -> Result<(), SeqError> {
    for arg in &node.args {
        if let SeqValue::CallSeq { name, args } = &arg.value {
            check_callback_call(file, arg, name, args, fns)?;
        } else if contains_callseq(&arg.value) {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                "calling the `.seq` function only as a direct attribute value (e.g. `on_click: { fn() }`)",
            ));
        }
    }
    for c in &node.children {
        check_calls_node(file, c, fns)?;
    }
    Ok(())
}

fn contains_callseq(value: &SeqValue) -> bool {
    match value {
        SeqValue::CallSeq { .. } => true,
        SeqValue::Callback { args, .. } => args.iter().any(contains_callseq),
        SeqValue::Tuple(items) | SeqValue::Array(items) => items.iter().any(contains_callseq),
        _ => false,
    }
}

fn check_callback_call(
    file: &str,
    arg: &crate::ir::SeqArg,
    name: &str,
    args: &[SeqValue],
    fns: &FnTable,
) -> Result<(), SeqError> {
    let attr = arg.name.as_str();
    let Some(sig) = fns.get(name) else {
        return Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("nieznana funkcja '{name}'"),
        ));
    };
    let all_idents = || args.iter().all(|a| matches!(a, SeqValue::Ident(_)));
    let expected: Vec<Ty> = if let Some(lt) = live_callback_ty(attr) {
        if args.is_empty() {
            Vec::new()
        } else if args.len() == 1 && matches!(&args[0], SeqValue::Ident(n) if n == "value") {
            vec![lt]
        } else {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("{attr}: {{ {name}() }} albo {{ {name}(value) }}"),
            ));
        }
    } else if PLAIN_CALLBACKS.contains(&attr) {
        if args.is_empty() {
            Vec::new()
        } else if attr == "on_run" {
            return Err(SeqError::new(file, arg.line, 1, "on_run: no arguments"));
        } else if args.len() == 1 {
            vec![match &args[0] {
                SeqValue::Num(n) if n.fract() == 0.0 => Ty::Int,
                SeqValue::Num(_) => Ty::Float,
                SeqValue::Str(_) => Ty::Str,
                SeqValue::Bool(_) => Ty::Bool,
                SeqValue::ItemField(_) => Ty::Any,
                _ => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        "on_click: argument to liczba/tekst/Bool/item.pole (albo bez argumentu)"
                            .to_string(),
                    ));
                }
            }]
        } else {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("on_click: {{ {name}(arg) }} (0–1 argument)"),
            ));
        }
    } else if GESTURE_CALLBACKS.contains(&attr) {
        if args.is_empty() {
            Vec::new()
        } else if !all_idents() {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!("{attr}: {{ {name}(x, y[, name|d]) }}"),
            ));
        } else {
            match args.len() {
                1 => vec![Ty::Str],
                2 => vec![Ty::Float, Ty::Float],
                3 => {
                    let third_is_delta = matches!(&args[2], SeqValue::Ident(n) if n == "d");
                    if third_is_delta {
                        vec![Ty::Float, Ty::Float, Ty::Float]
                    } else {
                        vec![Ty::Float, Ty::Float, Ty::Str]
                    }
                }
                _ => {
                    return Err(SeqError::new(
                        file,
                        arg.line,
                        1,
                        format!("{attr}: {{ {name}(name) }} albo {{ {name}(x, y[, name|d]) }}"),
                    ));
                }
            }
        }
    } else if attr == "on_wheel" || attr == "on_pinch" {
        if args.is_empty() {
            Vec::new()
        } else if args.len() == 3 && all_idents() {
            vec![Ty::Float, Ty::Float, Ty::Float]
        } else {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!(
                    "{attr}: {{ {name}(x, y, {}) }}",
                    if attr == "on_wheel" { "d" } else { "s" }
                ),
            ));
        }
    } else {
        return Err(SeqError::new(
            file,
            arg.line,
            1,
            format!("calling '{name}' only as callback (on_click/on_toggle/on_*/gesty/on_run)"),
        ));
    };
    if expected.len() != sig.params.len() {
        return Err(SeqError::new(
            file,
            arg.line,
            1,
            format!(
                "fn {name}: expected {} parameters, and {attr} provides {} values",
                sig.params.len(),
                expected.len()
            ),
        ));
    }
    for (i, (e, p)) in expected.iter().zip(&sig.params).enumerate() {
        if !e.fits(p) {
            return Err(SeqError::new(
                file,
                arg.line,
                1,
                format!(
                    "fn {name}: parametr {} to {}, a {attr} podaje {}",
                    i + 1,
                    p.name(),
                    e.name()
                ),
            ));
        }
    }
    Ok(())
}

fn check_anims(file: &str, app: &crate::ir::SeqApp) -> Result<(), SeqError> {
    use std::collections::HashSet;
    let mut names: HashSet<String> = HashSet::new();
    for a in &app.anims {
        if !names.insert(a.name.clone()) {
            return Err(SeqError::new(
                file,
                a.line,
                1,
                format!("anim '{}' zdefiniowany dwa razy", a.name),
            ));
        }
        if let Some(m) = &a.loop_mode {
            if !matches!(m.as_str(), "once" | "loop" | "pingpong") {
                return Err(SeqError::new(
                    file,
                    a.line,
                    1,
                    format!("nieznany tryb loop '{}'", m),
                ));
            }
        }
        for t in &a.tracks {
            if let crate::ir::SeqValue::Array(items) = &t.value {
                let mut prev: Option<f32> = None;
                for kf in items {
                    if let crate::ir::SeqValue::Tuple(parts) = kf {
                        if let Some(crate::ir::SeqValue::Num(time)) = parts.first() {
                            if let Some(p) = prev {
                                if *time < p {
                                    return Err(SeqError::new(
                                        file,
                                        t.line,
                                        1,
                                        format!(
                                            "{}: track {}: czas {} po {}",
                                            a.name, t.name, time, p
                                        ),
                                    ));
                                }
                            }
                            prev = Some(*time);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_machines(file: &str, app: &crate::ir::SeqApp) -> Result<(), SeqError> {
    use std::collections::HashSet;
    let mut names: HashSet<String> = HashSet::new();
    for m in &app.machines {
        if !names.insert(m.name.clone()) {
            return Err(SeqError::new(
                file,
                m.line,
                1,
                format!("machine '{}' zdefiniowany dwa razy", m.name),
            ));
        }
        for tr in &m.transitions {
            for c in &tr.conditions {
                if c.starts_with("num(") {
                    let ops = ["<", "<=", "==", "!=", ">=", ">"];
                    if !ops.iter().any(|o| c.contains(o)) {
                        return Err(SeqError::new(
                            file,
                            tr.line,
                            1,
                            "nieznany operator".to_string(),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_springs(file: &str, app: &crate::ir::SeqApp) -> Result<(), SeqError> {
    use std::collections::HashSet;
    let mut names: HashSet<String> = HashSet::new();
    for s in &app.springs {
        if !names.insert(s.name.clone()) {
            return Err(SeqError::new(
                file,
                s.line,
                1,
                format!("spring '{}' zdefiniowany dwa razy", s.name),
            ));
        }
    }
    Ok(())
}
