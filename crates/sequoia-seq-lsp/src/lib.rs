#![forbid(unsafe_code)]

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqDiag {
    pub line: u32,

    pub col: u32,

    pub end_col: u32,

    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqComplete {
    pub label: String,

    pub kind: u32,

    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqSymbol {
    pub name: String,

    pub kind: String,

    pub line: u32,
}

pub const COMPONENTS: &[&str] = &[
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
    "Clip",
    "Group",
];

pub const THEME_FIELDS: &[(&str, &str)] = &[
    ("bg", "application background (theme.background)"),
    ("background", "alias bg"),
    ("surface", "card/container background"),
    ("border", "kolor ramki"),
    ("ink", "main text (theme.text)"),
    ("text", "alias ink"),
    ("muted", "secondary text"),
    ("text_secondary", "alias muted"),
    ("accent", "akcent (cyjan/niebieski)"),
    ("disabled", "disabled state"),
];

pub const APP_KEYWORDS: &[(&str, &str)] = &[
    ("title", "window title: title: \"...\""),
    ("size", "rozmiar: size: (W, H)"),
    ("theme", "motyw: dark | light | \"tokens.json\""),
    ("prop", "deklaracja stanu z hosta: prop nazwa: Typ"),
    (
        "on",
        "callback declaration: on name(args) - one signature per line",
    ),
    (
        "style",
        "blok stylu: style Nazwa: Box|Button|Text [extends Baza] { ... }",
    ),
    ("window", "blok okna: window Nazwa [(size: (W, H))] { ... }"),
    (
        "component",
        "komponent: component Nazwa(p: Typ, ...) { ... } (obok app, typy: String/Bool/Int/Float/Color)",
    ),
    (
        "extends",
        "baza stylu: style Nazwa: Button extends preset { ... }",
    ),
    ("children", "slot in component body (bare tag, max 1)"),
];

pub const TYPES: &[&str] = &["String", "Bool", "Int", "Float", "Color"];

pub fn props_for(component: &str) -> &'static [&'static str] {
    match component {
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
            "hit",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "hit",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "hit",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "hit",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
            "step",
            "color",
            "visible",
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
            "hit",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
        "ScrollView" => &[
            "axis",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
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
            "hit",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
        "Spacer" => &[
            "flex",
            "width",
            "height",
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "hit",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "hit",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
        "List" => &[
            "items",
            "key",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
        "VirtualList" => &[
            "items",
            "key",
            "item_height",
            "overscan",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
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
            "hit",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
        "If" => &[
            "when",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
        "Async" => &[
            "source",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
        "Loading" | "Data" | "Error" | "Idle" | "Cancelled" => &[],
        "Effect" => &[
            "track",
            "on_run",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
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
            "min_width",
            "max_width",
            "min_height",
            "max_height",
            "margin",
            "margin_x",
            "margin_y",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
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
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
        "Progress" => &[
            "value",
            "height",
            "track",
            "fill",
            "flex",
            "indent",
            "indent_x",
            "indent_y",
            "indent_left",
            "indent_right",
            "indent_top",
            "indent_bottom",
        ],
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
        "Clip" => &["rect", "radius", "visible"],
        "Group" => &["translate", "rotate", "scale"],
        _ => &[],
    }
}

pub fn values_for(prop: &str) -> &'static [&'static str] {
    match prop {
        "align" | "align_content" | "align_self" => &["center", "start", "end", "stretch"],
        "justify" => &["start", "center", "end", "between"],
        "axis" => &["vertical", "horizontal", "both"],
        "overflow" => &["visible", "hidden", "scroll", "auto"],
        "auto_flow" => &["row", "column"],
        "cursor" => &[
            "default",
            "pointer",
            "crosshair",
            "grab",
            "grabbing",
            "text",
        ],
        "at" => &[
            "top",
            "bottom",
            "left",
            "right",
            "top_left",
            "top_right",
            "bottom_left",
            "bottom_right",
            "fill",
            "center",
        ],
        "style" => &[
            "primary",
            "secondary",
            "danger",
            "ghost",
            "dark_filled",
            "outline_rose",
        ],
        "theme" => &["dark", "light", "\"tokens.json\""],
        "width" | "height" => &["fill(1)", "px(200)", "fit", "percent(50)"],
        "background" | "color" | "fill" | "border_color" | "handle_color" | "grid_color"
        | "axes_color" => &[
            "theme.bg",
            "theme.surface",
            "theme.border",
            "theme.ink",
            "theme.muted",
            "theme.accent",
            "#RRGGBB",
        ],

        "track" => &[
            "dynamic(nazwa)",
            "env.nazwa",
            "item.pole",
            "theme.bg",
            "theme.accent",
        ],
        "dynamic" => &["dynamic(nazwa)"],
        "t" => &["dynamic(nazwa)", "\"klucz-i18n\""],
        "on_click" | "on_toggle" | "on_submit" | "on_edit_end" | "on_change" | "on_commit"
        | "on_select" | "on_run" => &["{ akcja() }", "{ akcja(x) }"],
        "on_click_at" | "on_drag" | "on_press" => {
            &["{ akcja() }", "{ akcja(x, y) }", "{ akcja(x, y, name) }"]
        }
        "on_wheel" | "on_pinch" => &["{ akcja(x, y, d) }"],
        "checked" | "enabled" | "wrap" | "transparent" | "focusable" | "visible" | "bold"
        | "clip" | "capture" => &["true", "false"],
        _ => &[],
    }
}

fn token_len(line_text: &str, col: usize) -> usize {
    let chars: Vec<char> = line_text.chars().collect();
    if col >= chars.len() {
        return 1;
    }
    let mut len = 0;
    for c in chars.iter().skip(col) {
        if c.is_alphanumeric() || *c == '_' || *c == '.' || *c == '"' || *c == '#' {
            len += 1;
        } else {
            break;
        }
    }
    len.max(1)
}

pub fn diagnostics_for(file: &str, src: &str) -> Vec<SeqDiag> {
    let app = match sequoia_seq::parse(file, src) {
        Ok(a) => a,
        Err(e) => return vec![seq_err_to_diag(src, &e.line, &e.col, &e.message)],
    };
    if let Err(e) = sequoia_seq::check(file, &app) {
        return vec![seq_err_to_diag(src, &e.line, &e.col, &e.message)];
    }
    Vec::new()
}

fn seq_err_to_diag(src: &str, line1: &usize, col1: &usize, message: &str) -> SeqDiag {
    let line0 = line1.saturating_sub(1) as u32;
    let col0 = col1.saturating_sub(1) as u32;
    let line_text = src.lines().nth(line1.saturating_sub(1)).unwrap_or("");
    let len = token_len(line_text, col0 as usize) as u32;
    SeqDiag {
        line: line0,
        col: col0,
        end_col: col0 + len,
        message: message.to_string(),
    }
}

fn line_prefix(src: &str, line: u32, character: u32) -> String {
    let text = src.lines().nth(line as usize).unwrap_or("");
    let chars: Vec<char> = text.chars().collect();
    let end = (character as usize).min(chars.len());
    chars[..end].iter().collect()
}

fn enclosing_component(src: &str, line: u32) -> Option<String> {
    let lines: Vec<&str> = src.lines().collect();
    let upto = (line as usize).min(lines.len().saturating_sub(1));
    for l in (0..=upto).rev() {
        let text = lines[l];
        let mut last: Option<String> = None;
        let bytes: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i].is_ascii_uppercase() {
                let mut j = i;
                while j < bytes.len() && (bytes[j].is_alphanumeric() || bytes[j] == '_') {
                    j += 1;
                }
                let mut k = j;
                while k < bytes.len() && bytes[k].is_whitespace() {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == '(' {
                    last = Some(bytes[i..j].iter().collect());
                }
                i = j;
            } else {
                i += 1;
            }
        }
        if let Some(c) = last.filter(|c| COMPONENTS.contains(&c.as_str())) {
            return Some(c);
        }
        for c in COMPONENTS {
            if text.contains(&format!("{c} ")) || text.contains(&format!("{c} {{")) {
                return Some((*c).to_string());
            }
        }
    }
    None
}

fn enclosing_parent(src: &str, line: u32) -> Option<String> {
    let prefix = line_prefix(src, line, u32::MAX);
    let mut parent: Option<String> = None;
    for name in ["Stack", "Grid"] {
        if let Some(pos) = prefix.rfind(&format!("{name}("))
            && prefix[pos..].matches(')').count() < prefix[pos..].matches('(').count()
        {
            parent = Some(name.to_string());
        }
    }
    parent
}

pub fn completions_for(src: &str, line: u32, character: u32) -> Vec<SeqComplete> {
    let prefix = line_prefix(src, line, character);
    let trimmed = prefix.trim_start();

    if let Some(dot) = trimmed.rfind("theme.") {
        let frag: String = trimmed[dot + 6..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        return THEME_FIELDS
            .iter()
            .filter(|(n, _)| n.starts_with(frag.as_str()))
            .map(|(n, d)| SeqComplete {
                label: (*n).to_string(),
                kind: 5,
                detail: (*d).to_string(),
            })
            .collect();
    }

    if trimmed.rfind("env.").is_some() {
        return vec![SeqComplete {
            label: "nazwa".to_string(),
            kind: 5,
            detail: "host value: HostBindings::env_str/bool/int/float(\"name\")".to_string(),
        }];
    }

    if trimmed.rfind("item.").is_some() {
        return vec![SeqComplete {
            label: "pole".to_string(),
            kind: 5,
            detail: "List row field (check SeqRow::fields() on host; item.id always works)"
                .to_string(),
        }];
    }

    if let Some(colon) = trimmed.rfind(':') {
        let before = trimmed[..colon].trim_end();
        if let Some(prop) = before.split([',', '(', ' ', '\t']).next_back() {
            let vals = values_for(prop);
            if !vals.is_empty() {
                let frag: String = trimmed[colon + 1..]
                    .trim_start()
                    .chars()
                    .take_while(|c| {
                        c.is_alphanumeric() || *c == '_' || *c == '(' || *c == ')' || *c == '"'
                    })
                    .collect();
                let frag_trim = frag.trim_matches('"');
                return vals
                    .iter()
                    .filter(|v| {
                        v.starts_with(frag.as_str()) || v.trim_matches('"').starts_with(frag_trim)
                    })
                    .map(|v| SeqComplete {
                        label: (*v).to_string(),
                        kind: 22,
                        detail: format!("value for {prop}:"),
                    })
                    .collect();
            }
        }
    }

    let mut out = Vec::new();

    if (prefix.contains(':') || prefix.contains('('))
        && let Some(comp) = enclosing_component(src, line)
    {
        for p in props_for(&comp) {
            out.push(SeqComplete {
                label: (*p).to_string(),
                kind: 10,
                detail: format!("propka {comp}:"),
            });
        }
        if let Some(parent) = enclosing_parent(src, line) {
            let extra: &[&str] = match parent.as_str() {
                "Stack" => &["at", "offset", "z"],
                "Grid" => &["col", "row", "col_span", "row_span"],
                _ => &[],
            };
            for p in extra {
                out.push(SeqComplete {
                    label: (*p).to_string(),
                    kind: 10,
                    detail: format!("propka dziecka {parent}:"),
                });
            }
        }
    }

    let frag: String = trimmed
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    for c in COMPONENTS {
        if frag.is_empty() || c.starts_with(frag.as_str()) {
            out.push(SeqComplete {
                label: (*c).to_string(),
                kind: 13,
                detail: format!("komponent .seq: {}", props_for(c).join(", ")),
            });
        }
    }

    for (kw, doc) in APP_KEYWORDS {
        if frag.is_empty() || kw.starts_with(&frag) {
            out.push(SeqComplete {
                label: (*kw).to_string(),
                kind: 14,
                detail: (*doc).to_string(),
            });
        }
    }
    for t in TYPES {
        if frag.is_empty() || t.starts_with(&frag) {
            out.push(SeqComplete {
                label: (*t).to_string(),
                kind: 14,
                detail: "typ prop/on/component".to_string(),
            });
        }
    }

    out.sort_by(|a, b| a.label.cmp(&b.label));
    out.dedup_by(|a, b| a.label == b.label);
    out.truncate(80);
    out
}

fn word_at(src: &str, line: u32, character: u32) -> String {
    let text = src.lines().nth(line as usize).unwrap_or("");
    let chars: Vec<char> = text.chars().collect();
    let mut i = (character as usize).min(chars.len());
    if i > 0 && i == chars.len() {
        i -= 1;
    }
    if i < chars.len() && !(chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
        return String::new();
    }
    let mut s = i;
    while s > 0 && (chars[s - 1].is_alphanumeric() || chars[s - 1] == '_' || chars[s - 1] == '.') {
        s -= 1;
    }
    let mut e = i;
    while e < chars.len() && (chars[e].is_alphanumeric() || chars[e] == '_' || chars[e] == '.') {
        e += 1;
    }
    chars[s..e].iter().collect()
}

pub fn hover_for(src: &str, line: u32, character: u32) -> Option<String> {
    let word = word_at(src, line, character);
    if word.is_empty() {
        return None;
    }
    if let Some(field) = word.strip_prefix("theme.") {
        for (n, d) in THEME_FIELDS {
            if *n == field {
                return Some(format!(
                    "**theme.{n}** — {d}\n\n`theme.bg | surface | border | ink | muted | accent | disabled`"
                ));
            }
        }
        return Some(
            "**theme.*** — nieznane pole (bg/surface/border/ink/muted/accent/disabled)".to_string(),
        );
    }
    if let Some(field) = word.strip_prefix("env.") {
        return Some(format!(
            "**env.{field}** - host value (`HostBindings::env_str/bool/int/float`); in numbers/bools snapshot, in `Text(dynamic:)` subscription"
        ));
    }
    if word == "env" {
        return Some(
            "**env.name** - value from the host environment (type depends on propka)".to_string(),
        );
    }
    if COMPONENTS.contains(&word.as_str()) {
        let props = props_for(&word).join(", ");
        return Some(format!("**{word}** — komponent .seq\n\nPropki: `{props}`"));
    }
    for (kw, doc) in APP_KEYWORDS {
        if *kw == word {
            return Some(format!("**{kw}** — {doc}"));
        }
    }
    if TYPES.contains(&word.as_str()) {
        return Some(format!(
            "**{word}** — typ `prop` / parametru `on` / parametru `component` (Color tylko w component)"
        ));
    }
    match word.as_str() {
        "dynamic" => Some(
            "**dynamic(name)** - read subscribed host signal (`Text(dynamic:)`, `text: dynamic(x)`, numbers/points/visible in Canvas)"
                .to_string(),
        ),
        "item" | "item.id" | "item.title" => {
            Some("**item.*** — pole wiersza `List` (tylko w szablonie `List { ... }`)".to_string())
        }
        "fill" => Some("**fill(weight)** - fill length (`width: fill(1)`)".to_string()),
        "px" => Some("**px(n)** — piksele (`width: px(200)`)".to_string()),
        "percent" => Some("**percent(n)** — procent rodzica".to_string()),
        "fit" => Some("**fit** - fit to content (default)".to_string()),
        "app" => Some(
            "**app Nazwa { ... }** — jeden na plik; wymaga `title:`, `size:` i min. 1× `window`"
                .to_string(),
        ),
        "component" => Some(
            "**component Name(p: Type, ...) { ... }** - next to `app`; call `Name(p: v)` with all parameters + `flex:`, children only in `children` slot"
                .to_string(),
        ),
        "children" => Some(
            "**children** - bare slot tag in `component` body (max 1)".to_string(),
        ),
        "extends" => Some(
            "**extends** — baza stylu (`style X: Button extends preset`) albo stylu tego samego typu"
                .to_string(),
        ),
        "window" => {
            Some("**window Name [(size: (W, H))] { ... }** - first is main".to_string())
        }
        "dark" | "light" => Some(
            "**dark | light** — wbudowany motyw (`theme: dark`); albo `theme: \"tokens.json\"`"
                .to_string(),
        ),
        _ => {
            if word.starts_with("item.") {
                return Some(format!(
                    "**{word}** - row field `List` (check `SeqRow::fields()` on host)"
                ));
            }
            None
        }
    }
}

pub fn symbols_for(src: &str) -> Vec<SeqSymbol> {
    let mut out = Vec::new();
    for (idx, line_text) in src.lines().enumerate() {
        let t = line_text.trim_start();
        for (prefix, kind) in [
            ("app ", "app"),
            ("component ", "component"),
            ("window ", "window"),
            ("style ", "style"),
        ] {
            if let Some(rest) = t.strip_prefix(prefix) {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    out.push(SeqSymbol {
                        name,
                        kind: kind.to_string(),
                        line: idx as u32,
                    });
                    continue;
                }
            }
        }
        if out.last().is_some_and(|s| s.line == idx as u32) {
            continue;
        }

        let ident: String = t
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if COMPONENTS.contains(&ident.as_str()) {
            out.push(SeqSymbol {
                name: ident,
                kind: "node".to_string(),
                line: idx as u32,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = "app Demo {\n  title: \"Demo\"\n  size: (400, 300)\n  window Main {\n    Column(padding: 12) {\n      Text(\"Hi\", size: 14)\n    }\n  }\n}\n";

    #[test]
    fn clean_file_has_no_diags() {
        assert!(diagnostics_for("Main.seq", DEMO).is_empty());
    }

    #[test]
    fn unknown_component_is_diag_with_pos() {
        let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Col {}\n  }\n}\n";
        let d = diagnostics_for("Main.seq", src);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].line, 4);
        assert!(d[0].message.contains("Col"));
    }

    #[test]
    fn host_constructs_are_not_diags() {
        let src = "app A {\n  title: \"A\"\n  size: (10, 10)\n  prop label: String\n  on add()\n  window M {\n    Text(dynamic: label)\n    Button(\"Go\", on_click: { add() })\n  }\n}\n";
        assert!(diagnostics_for("Main.seq", src).is_empty());
    }

    #[test]
    fn theme_completion_after_dot() {
        let items = completions_for("Column(background: theme.)", 0, 25);
        assert!(items.iter().any(|i| i.label == "bg"));
        assert!(items.iter().any(|i| i.label == "accent"));
    }

    #[test]
    fn component_completion() {
        let items = completions_for("    Col", 0, 7);
        assert!(items.iter().any(|i| i.label == "Column"));
    }

    #[test]
    fn hover_component() {
        let h = hover_for("    Column(padding: 12)", 0, 6).unwrap();
        assert!(h.contains("Column"));
        assert!(h.contains("padding"));
    }

    #[test]
    fn symbols_outline() {
        let s = symbols_for(DEMO);
        assert!(s.iter().any(|x| x.kind == "app" && x.name == "Demo"));
        assert!(s.iter().any(|x| x.kind == "window" && x.name == "Main"));
        assert!(s.iter().any(|x| x.kind == "node" && x.name == "Column"));
    }

    #[test]
    fn component_and_style_are_clean() {
        let src = "component Card(title: String, c: Color) {\n  Column {\n    Text(text: title)\n    children\n  }\n}\napp A {\n  title: \"A\"\n  size: (400, 300)\n  style Danger2: Button extends danger {\n    radius: 12\n  }\n  window M {\n    Card(title: \"Hi\", c: #FF0000)\n  }\n}\n";
        assert!(diagnostics_for("Main.seq", src).is_empty());
    }

    #[test]
    fn new_components_have_props() {
        assert!(props_for("Image").contains(&"path"));
        assert!(props_for("Checkbox").contains(&"on_toggle"));
        assert!(props_for("TextField").contains(&"on_submit"));
        assert!(props_for("Slider").contains(&"on_change"));
        assert!(props_for("Dropdown").contains(&"on_select"));
        assert!(props_for("Text").contains(&"wrap"));
        assert!(props_for("Text").contains(&"style"));
        assert!(props_for("Container").contains(&"style"));
        assert!(props_for("Canvas").contains(&"on_pinch"));
        assert!(props_for("Circle").contains(&"pick_r"));
    }

    #[test]
    fn env_hover_and_completion() {
        let h = hover_for("    Text(t: env.lang)", 0, 15).unwrap();
        assert!(h.contains("env.lang"));
        let items = completions_for("    Text(t: env.", 0, 17);
        assert!(items.iter().any(|i| i.detail.contains("HostBindings")));
    }

    #[test]
    fn style_extends_completion() {
        let items = completions_for("    Button(style: )", 0, 18);
        assert!(items.iter().any(|i| i.label == "primary"));
    }

    #[test]
    fn symbols_include_component_and_style() {
        let src = "component Card(title: String) {\n  Text(text: title)\n}\napp A {\n  title: \"A\"\n  size: (10, 10)\n  style Card: Box {\n    radius: 8\n  }\n  window M {\n    Text(\"x\")\n  }\n}\n";
        let s = symbols_for(src);
        assert!(s.iter().any(|x| x.kind == "component" && x.name == "Card"));
        assert!(s.iter().any(|x| x.kind == "style" && x.name == "Card"));
    }
}
