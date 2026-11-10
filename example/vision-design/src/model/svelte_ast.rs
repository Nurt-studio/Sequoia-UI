//! AST tree mirroring Svelte. The single source of truth.
//! All `Style` fields map 1:1 to CSS.

use sequoia_ui::prelude::{Color, Edges};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tag {
    Div,
    Button,
    Span,
    P,
    Section,
    Article,
    Header,
    Footer,
    Nav,
    Input,
    Textarea,
}

impl Tag {
    pub fn as_str(self) -> &'static str {
        match self {
            Tag::Div => "div",
            Tag::Button => "button",
            Tag::Span => "span",
            Tag::P => "p",
            Tag::Section => "section",
            Tag::Article => "article",
            Tag::Header => "header",
            Tag::Footer => "footer",
            Tag::Nav => "nav",
            Tag::Input => "input",
            Tag::Textarea => "textarea",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "div" => Tag::Div,
            "button" => Tag::Button,
            "span" => Tag::Span,
            "p" => Tag::P,
            "section" => Tag::Section,
            "article" => Tag::Article,
            "header" => Tag::Header,
            "footer" => Tag::Footer,
            "nav" => Tag::Nav,
            "input" => Tag::Input,
            "textarea" => Tag::Textarea,
            _ => return None,
        })
    }
    pub fn icon(self) -> &'static str {
        match self {
            Tag::Button => "tool/rect",
            Tag::Span | Tag::P => "tool/text",
            _ => "tool/frame",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Display {
    None,
    Flex,
    Grid,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlexDir {
    Row,
    Col,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wrap {
    NoWrap,
    Wrap,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
    SpaceBetween,
}

impl Align {
    pub fn css_items(self) -> &'static str {
        match self {
            Align::Start => "flex-start",
            Align::Center => "center",
            Align::End => "flex-end",
            Align::Stretch => "stretch",
            Align::SpaceBetween => "flex-start", // not used for align-items
        }
    }
    pub fn css_content(self) -> &'static str {
        match self {
            Align::Start => "flex-start",
            Align::Center => "center",
            Align::End => "flex-end",
            Align::Stretch => "stretch",
            Align::SpaceBetween => "space-between",
        }
    }
    pub fn from_items(s: &str) -> Self {
        match s {
            "center" => Align::Center,
            "flex-end" | "end" | "self-end" => Align::End,
            "stretch" => Align::Stretch,
            _ => Align::Start,
        }
    }
    pub fn from_self(s: &str) -> Option<Self> {
        match s {
            "auto" | "" => None,
            "center" => Some(Align::Center),
            "flex-end" | "end" | "self-end" => Some(Align::End),
            "stretch" => Some(Align::Stretch),
            "flex-start" | "start" | "self-start" => Some(Align::Start),
            _ => None,
        }
    }
    pub fn from_content(s: &str) -> Self {
        match s {
            "center" => Align::Center,
            "flex-end" | "end" => Align::End,
            "space-between" => Align::SpaceBetween,
            "stretch" => Align::Stretch,
            _ => Align::Start,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Position {
    Static,
    Relative,
    Absolute,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Dimension {
    Auto,
    Fill,
    Fixed(f32),
}

impl Dimension {
    pub fn value(self) -> Option<f32> {
        match self {
            Dimension::Fixed(v) => Some(v),
            _ => None,
        }
    }
    pub fn is_fill(self) -> bool {
        matches!(self, Dimension::Fill)
    }
}

// ── paints & effects ───────────────────────────────────────────

/// Gradient fill of an element (`background: linear-gradient(…)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientKind {
    Linear,
    Radial,
}

impl GradientKind {
    pub fn as_str(self) -> &'static str {
        match self {
            GradientKind::Linear => "linear",
            GradientKind::Radial => "radial",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "linear" => GradientKind::Linear,
            "radial" => GradientKind::Radial,
            _ => return None,
        })
    }
}

/// One color stop of a gradient. `at` is 0..1.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientStopDef {
    pub color: Color,
    pub hex: String,
    pub at: f32,
}

impl GradientStopDef {
    pub fn new(color: Color, hex: impl Into<String>, at: f32) -> Self {
        Self {
            color,
            hex: hex.into(),
            at: at.clamp(0.0, 1.0),
        }
    }
}

/// A two-to-four stop gradient. The GPU path supports four stops; the
/// editor caps the list so the canvas, code and export never disagree.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientDef {
    pub kind: GradientKind,
    /// Direction of a linear gradient in degrees (0 = left→right).
    pub angle: f32,
    pub stops: Vec<GradientStopDef>,
}

pub const MAX_GRADIENT_STOPS: usize = 4;

impl GradientDef {
    pub fn new(kind: GradientKind, angle: f32) -> Self {
        Self {
            kind,
            angle,
            stops: vec![
                GradientStopDef::new(Color::hex(0x0D99FF), "#0D99FF", 0.0),
                GradientStopDef::new(Color::hex(0x6E56CF), "#6E56CF", 1.0),
            ],
        }
    }

    /// Converts a solid fill into a gradient without changing its color.
    pub fn from_solid(color: Color, hex: &str) -> Self {
        Self {
            kind: GradientKind::Linear,
            angle: 90.0,
            stops: vec![
                GradientStopDef::new(color, hex, 0.0),
                GradientStopDef::new(Color::hex(0x6E56CF), "#6E56CF", 1.0),
            ],
        }
    }

    pub fn sorted_stops(&self) -> Vec<GradientStopDef> {
        let mut stops = self.stops.clone();
        stops.sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap_or(std::cmp::Ordering::Equal));
        stops
    }

    /// CSS `linear-gradient(...)` / `radial-gradient(...)` value.
    pub fn css(&self) -> String {
        let stops: Vec<String> = self
            .sorted_stops()
            .iter()
            .map(|s| format!("{} {:.0}%", css_color(s.color), s.at * 100.0))
            .collect();
        match self.kind {
            GradientKind::Linear => {
                format!(
                    "linear-gradient({:.0}deg, {})",
                    self.angle,
                    stops.join(", ")
                )
            }
            GradientKind::Radial => format!("radial-gradient(circle, {})", stops.join(", ")),
        }
    }
}

/// A drop-shadow effect (`box-shadow: x y blur spread color`).
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowEffect {
    pub color: Color,
    pub hex: String,
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub enabled: bool,
}

impl ShadowEffect {
    pub fn new() -> Self {
        Self {
            color: Color::hexa(0x00000099),
            hex: "#00000099".into(),
            x: 0.0,
            y: 8.0,
            blur: 24.0,
            spread: 0.0,
            enabled: true,
        }
    }
}

impl Default for ShadowEffect {
    fn default() -> Self {
        Self::new()
    }
}

/// CSS color literal that keeps alpha (`#RRGGBB` when opaque, otherwise
/// `rgba()`), so generated code and the importer agree on translucency.
pub fn css_color(c: Color) -> String {
    let (r, g, b, a) = c.to_rgba8();
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("rgba({r},{g},{b},{:.3})", a as f32 / 255.0)
    }
}

/// CSS `box-shadow` value, or an empty string when no effect is enabled.
pub fn shadows_css(effects: &[ShadowEffect]) -> String {
    effects
        .iter()
        .filter(|s| s.enabled)
        .map(|s| {
            format!(
                "{}px {}px {}px {}px {}",
                s.x,
                s.y,
                s.blur,
                s.spread,
                css_color(s.color)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// ── prototyping ───────────────────────────────────────────────

/// What starts a prototype connection while presenting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtoTrigger {
    Click,
    Hover,
}

impl ProtoTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            ProtoTrigger::Click => "click",
            ProtoTrigger::Hover => "hover",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "click" => ProtoTrigger::Click,
            "hover" => ProtoTrigger::Hover,
            _ => return None,
        })
    }
    pub fn label(self) -> &'static str {
        match self {
            ProtoTrigger::Click => "On Click",
            ProtoTrigger::Hover => "On Hover",
        }
    }
}

/// What a prototype connection does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtoAction {
    /// Focus the target frame (a screen-to-screen navigation).
    Navigate,
    /// Return to the previously focused frame.
    Back,
    /// Show the target frame above the current screen.
    OpenOverlay,
}

impl ProtoAction {
    pub fn as_str(self) -> &'static str {
        match self {
            ProtoAction::Navigate => "navigate",
            ProtoAction::Back => "back",
            ProtoAction::OpenOverlay => "overlay",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "navigate" => ProtoAction::Navigate,
            "back" => ProtoAction::Back,
            "overlay" => ProtoAction::OpenOverlay,
            _ => return None,
        })
    }
    pub fn label(self) -> &'static str {
        match self {
            ProtoAction::Navigate => "Navigate to",
            ProtoAction::Back => "Back",
            ProtoAction::OpenOverlay => "Open overlay",
        }
    }
}

/// Screen-transition animation used by the Present runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtoTransition {
    Instant,
    Dissolve,
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
}

impl ProtoTransition {
    pub fn as_str(self) -> &'static str {
        match self {
            ProtoTransition::Instant => "instant",
            ProtoTransition::Dissolve => "dissolve",
            ProtoTransition::SlideLeft => "slide-left",
            ProtoTransition::SlideRight => "slide-right",
            ProtoTransition::SlideUp => "slide-up",
            ProtoTransition::SlideDown => "slide-down",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "instant" => ProtoTransition::Instant,
            "dissolve" => ProtoTransition::Dissolve,
            "slide-left" => ProtoTransition::SlideLeft,
            "slide-right" => ProtoTransition::SlideRight,
            "slide-up" => ProtoTransition::SlideUp,
            "slide-down" => ProtoTransition::SlideDown,
            _ => return None,
        })
    }
    pub fn label(self) -> &'static str {
        match self {
            ProtoTransition::Instant => "Instant",
            ProtoTransition::Dissolve => "Dissolve",
            ProtoTransition::SlideLeft => "Slide left",
            ProtoTransition::SlideRight => "Slide right",
            ProtoTransition::SlideUp => "Slide up",
            ProtoTransition::SlideDown => "Slide down",
        }
    }
}

/// One prototype connection stored on the element that is clicked.
#[derive(Clone, Debug, PartialEq)]
pub struct PrototypeLink {
    pub trigger: ProtoTrigger,
    pub action: ProtoAction,
    /// Frame the link points at (ignored by `Back`).
    pub target: Option<u64>,
    pub transition: ProtoTransition,
    pub duration_ms: u32,
}

impl PrototypeLink {
    pub fn new(target: Option<u64>) -> Self {
        Self {
            trigger: ProtoTrigger::Click,
            action: ProtoAction::Navigate,
            target,
            transition: ProtoTransition::Dissolve,
            duration_ms: 240,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub display: Display,
    pub flex_dir: FlexDir,
    pub grid_cols: Option<u16>,
    pub grid_rows: Option<u16>,
    /// Spacing between items along the main axis (`gap` / item spacing).
    pub gap: f32,
    /// Spacing between wrapped lines (`gap` / row gap, cross axis).
    pub row_gap: f32,
    pub flex_wrap: Wrap,
    pub align_content: Align,
    pub padding: Edges,
    pub align_items: Align,
    pub align_self: Option<Align>,
    pub justify_content: Align,

    pub position: Position,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,

    pub width: Dimension,
    pub height: Dimension,

    pub fill: Option<Color>,
    pub fill_hex: String,
    pub stroke: Option<Color>,
    pub stroke_hex: String,
    pub stroke_width: f32,
    pub radius: f32,
    pub opacity: f32,
    /// Gradient painted instead of `fill` when set.
    pub gradient: Option<GradientDef>,
    /// Drop shadows, painted under the shape.
    pub effects: Vec<ShadowEffect>,
    /// Layer blur radius in px (`filter: blur(…)`).
    pub blur: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            display: Display::None,
            flex_dir: FlexDir::Row,
            grid_cols: None,
            grid_rows: None,
            gap: 0.0,
            row_gap: 0.0,
            flex_wrap: Wrap::NoWrap,
            align_content: Align::Start,
            padding: Edges::all(0.0),
            align_items: Align::Start,
            align_self: None,
            justify_content: Align::Start,
            position: Position::Relative,
            top: None,
            right: None,
            bottom: None,
            left: None,
            width: Dimension::Auto,
            height: Dimension::Auto,
            fill: None,
            fill_hex: "#00000000".into(),
            stroke: None,
            stroke_hex: "#00000000".into(),
            stroke_width: 0.0,
            radius: 0.0,
            opacity: 100.0,
            gradient: None,
            effects: Vec::new(),
            blur: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub font_weight: u16,
    pub color: Color,
    pub color_hex: String,
    pub align: Align,
    /// Percent (0-100). Multiplied into the painted layer alpha, so a text
    /// child of a translucent frame fades with it.
    pub opacity: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            font_weight: 400,
            color: Color::WHITE,
            color_hex: "#FFFFFF".into(),
            align: Align::Start,
            opacity: 100.0,
        }
    }
}

// ── component contract (Phase 1) ──────────────────────────────

/// What a prop drives on its target node. Phase 1 covers the two
/// families from the design doc: copy (text) and colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropKind {
    /// Content of a `TextNode`.
    Text,
    /// Background (`fill`) of an element.
    Fill,
    /// Text color of a `TextNode`.
    TextColor,
}

impl PropKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PropKind::Text => "text",
            PropKind::Fill => "fill",
            PropKind::TextColor => "text_color",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "text" => PropKind::Text,
            "fill" => PropKind::Fill,
            "text_color" => PropKind::TextColor,
            _ => return None,
        })
    }
}

/// The interface a master component publishes: what it accepts (props),
/// what it announces (events) and where parent content enters (slots).
/// Phase 1 implements props end to end; events and slots are part of the
/// model already, so the JSON format stays stable in later phases.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ComponentInterface {
    pub props: Vec<PropDef>,
    pub events: Vec<String>,
    pub slots: Vec<SlotDef>,
}

impl ComponentInterface {
    pub fn prop(&self, name: &str) -> Option<&PropDef> {
        self.props.iter().find(|p| p.name == name)
    }
    pub fn is_empty(&self) -> bool {
        self.props.is_empty() && self.events.is_empty() && self.slots.is_empty()
    }
}

/// A single knob of a component: a named value driving one node of the
/// master subtree.
#[derive(Clone, Debug, PartialEq)]
pub struct PropDef {
    pub name: String,
    /// Default value as text ("Okno", "#FA4D56", ...).
    pub default: String,
    /// Id of the node inside the master this prop drives.
    pub target: u64,
    pub kind: PropKind,
}

/// A hole for parent content. Rendered and filled in Phase 4.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotDef {
    pub name: String,
    /// Id of the element the parent's content falls into.
    pub container: u64,
}

/// One overridden prop on an instance. Keyed by prop **name** (not by node
/// id), so overrides survive re-id and reference the master's contract.
#[derive(Clone, Debug, PartialEq)]
pub struct Override {
    pub prop: String,
    pub value: String,
}

/// Content the parent puts into one slot of an instance (Phase 4). Stored on
/// the instance element, keyed by slot **name**; the nodes are laid out in
/// place of the slot container's fallback children (design doc §3.2).
#[derive(Clone, Debug, PartialEq)]
pub struct SlotContent {
    pub slot: String,
    pub nodes: Vec<SvelteNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub id: u64,
    pub name: String,
    pub tag: Tag,
    pub style: Style,
    pub children: Vec<SvelteNode>,
    pub visible: bool,
    pub locked: bool,
    pub component: bool,
    pub instance: bool,
    /// Master contract — only meaningful when `component` is set.
    pub interface: Option<ComponentInterface>,
    /// Overridden props — only meaningful when `instance` is set.
    pub overrides: Vec<Override>,
    /// Per-slot content provided by the parent (Phase 4, `instance` only).
    pub slot_content: Vec<SlotContent>,
    /// Local state + rules triggered by events on this element (Phase 2).
    pub behavior: Option<crate::model::behavior::Behavior>,
    /// Conditional visibility: `{#if var}` (Phase 2, design doc §5.2).
    pub binding: Option<crate::model::behavior::Binding>,
    /// Pure-CSS hover state (Phase 2, design doc §6.1).
    pub hover: Option<crate::model::behavior::HoverStyle>,
    /// Prototype connection fired while presenting.
    pub prototype: Option<PrototypeLink>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextNode {
    pub id: u64,
    pub name: String,
    pub content: String,
    pub style: TextStyle,
    pub width: Option<f32>, // fixed width (optional); None = auto
    pub height: Option<f32>,
    /// Position in world coordinates, used when `absolute` is set.
    pub x: f32,
    pub y: f32,
    /// Positioned freely like `position: absolute` on elements, instead of
    /// taking a slot in the parent's flow layout.
    pub absolute: bool,
    pub visible: bool,
    pub locked: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SvelteNode {
    Element(Element),
    Text(TextNode),
}

impl SvelteNode {
    pub fn id(&self) -> u64 {
        match self {
            SvelteNode::Element(e) => e.id,
            SvelteNode::Text(t) => t.id,
        }
    }
    /// A master component definition (◇).
    pub fn is_component(&self) -> bool {
        matches!(self, SvelteNode::Element(e) if e.component)
    }
    /// An instance of a component (◈).
    pub fn is_instance(&self) -> bool {
        matches!(self, SvelteNode::Element(e) if e.instance)
    }
    pub fn name(&self) -> &str {
        match self {
            SvelteNode::Element(e) => &e.name,
            SvelteNode::Text(t) => &t.name,
        }
    }
    pub fn name_mut(&mut self) -> &mut String {
        match self {
            SvelteNode::Element(e) => &mut e.name,
            SvelteNode::Text(t) => &mut t.name,
        }
    }
    pub fn visible(&self) -> bool {
        match self {
            SvelteNode::Element(e) => e.visible,
            SvelteNode::Text(t) => t.visible,
        }
    }
    pub fn locked(&self) -> bool {
        match self {
            SvelteNode::Element(e) => e.locked,
            SvelteNode::Text(t) => t.locked,
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            SvelteNode::Element(e) => e.tag.icon(),
            SvelteNode::Text(_) => "tool/text",
        }
    }
    pub fn children(&self) -> &[SvelteNode] {
        match self {
            SvelteNode::Element(e) => &e.children,
            SvelteNode::Text(_) => &[],
        }
    }
    pub fn children_mut(&mut self) -> Option<&mut Vec<SvelteNode>> {
        match self {
            SvelteNode::Element(e) => Some(&mut e.children),
            SvelteNode::Text(_) => None,
        }
    }
}

pub fn next_id(root: &Option<SvelteNode>) -> u64 {
    fn walk(n: &SvelteNode, max: &mut u64) {
        if n.id() > *max {
            *max = n.id();
        }
        for c in n.children() {
            walk(c, max);
        }
    }
    let mut m = 0;
    if let Some(r) = root {
        walk(r, &mut m);
    }
    m + 1
}

// ── helpers for creating new nodes ──

pub fn new_frame(id: u64, x: f32, y: f32, w: f32, h: f32) -> SvelteNode {
    SvelteNode::Element(Element {
        id,
        name: format!("Frame {}", id),
        tag: Tag::Div,
        style: Style {
            display: Display::Flex,
            position: Position::Relative,
            width: Dimension::Fixed(w),
            height: Dimension::Fixed(h),
            top: Some(y),
            left: Some(x),
            fill: Some(Color::hex(0x1B1B1B)),
            fill_hex: "#1B1B1B".into(),
            ..Default::default()
        },
        children: Vec::new(),
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

pub fn new_rect(id: u64, x: f32, y: f32, w: f32, h: f32, hex: u32) -> SvelteNode {
    SvelteNode::Element(Element {
        id,
        name: format!("Rectangle {}", id),
        tag: Tag::Div,
        style: Style {
            display: Display::None,
            position: Position::Absolute,
            width: Dimension::Fixed(w),
            height: Dimension::Fixed(h),
            top: Some(y),
            left: Some(x),
            fill: Some(Color::hex(hex)),
            fill_hex: format!("#{:06X}", hex),
            radius: 8.0,
            ..Default::default()
        },
        children: Vec::new(),
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

pub fn new_ellipse(id: u64, x: f32, y: f32, w: f32, h: f32, hex: u32) -> SvelteNode {
    let mut n = new_rect(id, x, y, w, h, hex);
    if let SvelteNode::Element(e) = &mut n {
        e.name = format!("Ellipse {}", id);
        e.style.radius = 9999.0;
    }
    n
}

pub fn new_text(id: u64, x: f32, y: f32, text: &str) -> SvelteNode {
    // New text starts in the parent's flow; the drawing tools flip `absolute`
    // so a click on the canvas places the text where the pointer was.
    SvelteNode::Text(TextNode {
        id,
        name: format!("Text {}", id),
        content: text.to_string(),
        style: TextStyle::default(),
        width: Some(200.0),
        height: Some(24.0),
        x,
        y,
        absolute: false,
        visible: true,
        locked: false,
    })
}

pub fn new_button(id: u64, x: f32, y: f32) -> SvelteNode {
    let mut text = new_text(id + 1, 0.0, 0.0, "Button");
    if let SvelteNode::Text(t) = &mut text {
        t.name = format!("Button label {}", id + 1);
        t.style.color = Color::WHITE;
        t.style.color_hex = "#FFFFFF".into();
        t.style.font_size = 14.0;
        t.width = None;
        t.height = None;
    }
    SvelteNode::Element(Element {
        id,
        name: format!("Button {}", id),
        tag: Tag::Button,
        style: Style {
            display: Display::None,
            position: Position::Absolute,
            width: Dimension::Auto,
            height: Dimension::Auto,
            top: Some(y),
            left: Some(x),
            padding: Edges::xy(16.0, 8.0),
            fill: Some(Color::hex(0x0D99FF)),
            fill_hex: "#0D99FF".into(),
            radius: 8.0,
            ..Default::default()
        },
        children: vec![text],
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
