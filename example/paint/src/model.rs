use std::path::PathBuf;
use std::time::Instant;

use sequoia_ui::prelude::*;

pub const PALETTE_COLORS: [Color; 20] = [
    Color::hex(0x000000),
    Color::hex(0x7F7F7F),
    Color::hex(0x880015),
    Color::hex(0xED1C24),
    Color::hex(0xFF7F27),
    Color::hex(0xFFF200),
    Color::hex(0x22B14C),
    Color::hex(0x00A2E8),
    Color::hex(0x3F48CC),
    Color::hex(0xA349A4),
    Color::hex(0xFFFFFF),
    Color::hex(0xC3C3C3),
    Color::hex(0xB97A57),
    Color::hex(0xFFAEC9),
    Color::hex(0xFFC90E),
    Color::hex(0xEFE4B0),
    Color::hex(0xB5E61D),
    Color::hex(0x99D9EA),
    Color::hex(0x7092BE),
    Color::hex(0xC8BFE7),
];

pub const CUSTOM_COLORS: [Color; 20] = [
    Color::hex(0x4B1E78),
    Color::hex(0x2D5DA8),
    Color::hex(0x1D7D3F),
    Color::hex(0x7D5A1D),
    Color::hex(0x7D1D1D),
    Color::hex(0x1D7D78),
    Color::hex(0xA8A8A8),
    Color::hex(0x565656),
    Color::hex(0xF0F0F0),
    Color::hex(0x13233A),
    Color::hex(0xB06AD4),
    Color::hex(0x6AB4D4),
    Color::hex(0x8AD46A),
    Color::hex(0xD4B46A),
    Color::hex(0xD46A6A),
    Color::hex(0x6AD4B4),
    Color::hex(0xFFD800),
    Color::hex(0xFF6D00),
    Color::hex(0x00D4FF),
    Color::hex(0x7CFF00),
];

pub const ZOOMS: [f32; 11] = [
    12.5, 25.0, 50.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0,
];

pub const LINE_WIDTHS: [f32; 4] = [1.0, 3.0, 5.0, 8.0];

pub const DEFAULT_DOC_W: u32 = 1152;
pub const DEFAULT_DOC_H: u32 = 600;
pub const MAX_UNDO: usize = 30;
pub const COL_BG: Color = Color::hex(0xF3F3F3);
pub const COL_RIBBON: Color = Color::hex(0xFCFCFC);
pub const COL_BORDER: Color = Color::hex(0xD6D6D6);
pub const COL_GROUP_TEXT: Color = Color::hex(0x6B6B6B);
pub const COL_TEXT: Color = Color::hex(0x202020);
pub const COL_MUTED: Color = Color::hex(0x6D6D6D);
pub const COL_HOVER: Color = Color::hex(0xE5F1FB);
pub const COL_ACTIVE: Color = Color::hex(0xCCE8FF);
pub const COL_ACCENT: Color = Color::hex(0x2B579A);
pub const COL_VIEWPORT: Color = Color::hex(0xEFEFEF);
pub const COL_RULER: Color = Color::hex(0xFAFAFA);
pub const COL_STATUS: Color = Color::hex(0xF0F0F0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Pencil,
    Fill,
    Text,
    Eraser,
    Picker,
    Magnifier,
    Brush,
    Shape,
    Select,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushKind {
    Brush,
    Airbrush,
    Calligraphy1,
    Calligraphy2,
    Crayon,
    Marker,
    NaturalPencil,
    Watercolor,
    Highlighter,
}

impl BrushKind {
    pub const ALL: [BrushKind; 9] = [
        BrushKind::Brush,
        BrushKind::Airbrush,
        BrushKind::Calligraphy1,
        BrushKind::Calligraphy2,
        BrushKind::Crayon,
        BrushKind::Marker,
        BrushKind::NaturalPencil,
        BrushKind::Watercolor,
        BrushKind::Highlighter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BrushKind::Brush => "Brush",
            BrushKind::Airbrush => "Airbrush",
            BrushKind::Calligraphy1 => "Calligraphy 1",
            BrushKind::Calligraphy2 => "Calligraphy 2",
            BrushKind::Crayon => "Crayon",
            BrushKind::Marker => "Marker",
            BrushKind::NaturalPencil => "Natural pencil",
            BrushKind::Watercolor => "Watercolor",
            BrushKind::Highlighter => "Highlighter",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeKind {
    Line,
    Curve,
    Oval,
    Rectangle,
    RoundedRectangle,
    Polygon,
    Triangle,
    RightTriangle,
    Diamond,
    Pentagon,
    Hexagon,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    Star4,
    Star5,
    Star6,
    Callout,
    OvalCallout,
    Heart,
    Lightning,
}

impl ShapeKind {
    pub const ALL: [ShapeKind; 22] = [
        ShapeKind::Line,
        ShapeKind::Curve,
        ShapeKind::Oval,
        ShapeKind::Rectangle,
        ShapeKind::RoundedRectangle,
        ShapeKind::Polygon,
        ShapeKind::Triangle,
        ShapeKind::RightTriangle,
        ShapeKind::Diamond,
        ShapeKind::Pentagon,
        ShapeKind::Hexagon,
        ShapeKind::ArrowRight,
        ShapeKind::ArrowLeft,
        ShapeKind::ArrowUp,
        ShapeKind::ArrowDown,
        ShapeKind::Star4,
        ShapeKind::Star5,
        ShapeKind::Star6,
        ShapeKind::Callout,
        ShapeKind::OvalCallout,
        ShapeKind::Heart,
        ShapeKind::Lightning,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ShapeKind::Line => "Line",
            ShapeKind::Curve => "Curve",
            ShapeKind::Oval => "Oval",
            ShapeKind::Rectangle => "Rectangle",
            ShapeKind::RoundedRectangle => "Rounded rectangle",
            ShapeKind::Polygon => "Polygon",
            ShapeKind::Triangle => "Triangle",
            ShapeKind::RightTriangle => "Right triangle",
            ShapeKind::Diamond => "Diamond",
            ShapeKind::Pentagon => "Pentagon",
            ShapeKind::Hexagon => "Hexagon",
            ShapeKind::ArrowRight => "Arrow right",
            ShapeKind::ArrowLeft => "Arrow left",
            ShapeKind::ArrowUp => "Arrow up",
            ShapeKind::ArrowDown => "Arrow down",
            ShapeKind::Star4 => "Four-point star",
            ShapeKind::Star5 => "Five-point star",
            ShapeKind::Star6 => "Six-point star",
            ShapeKind::Callout => "Callout",
            ShapeKind::OvalCallout => "Oval callout",
            ShapeKind::Heart => "Heart",
            ShapeKind::Lightning => "Lightning",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillMode {
    Outline,
    Filled,
    Both,
}

impl FillMode {
    pub fn name(self) -> &'static str {
        match self {
            FillMode::Outline => "Outline",
            FillMode::Filled => "Fill",
            FillMode::Both => "Outline and fill",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    View,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Save,
    Undo,
    Redo,
    Paste,
    Cut,
    Copy,
    Select,
    Crop,
    Resize,
    RotateLeft,
    RotateRight,
    FlipH,
    FlipV,
    Pencil,
    Fill,
    Text,
    Eraser,
    Picker,
    Magnifier,
    Brush,
    Size,
    Outline,
    FillDropdown,
    Wheel,
    Shape(ShapeKind),
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Fit,
    Ruler,
    Grid,
    StatusBar,
    Thumbnail,
    Fullscreen,
    Check,
    Close,
    File,
    Folder,
    Print,
    Info,
    Properties,
    Wallpaper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Save,
    Undo,
    Redo,
    Paste,
    Cut,
    Copy,
    SelectTool,
    SelectAll,
    Crop,
    ResizeDialog,
    RotateLeft,
    RotateRight,
    FlipH,
    FlipV,
    SetTool(Tool),
    SetBrush(BrushKind),
    SetShape(ShapeKind),
    SetFill(FillMode),
    SetWidth(usize),
    SetColor1,
    SetColor2,
    Palette(usize),
    EditColors,
    CycleBrush,
    CycleWidth,
    CycleFill,
    ZoomIn,
    ZoomOut,
    Zoom100,
    FitWindow,
    ToggleRulers,
    ToggleGrid,
    ToggleStatus,
    ToggleThumbnail,
    ToggleFullscreen,
    New,
    Open,
    SaveAs,
    Print,
    SetWallpaper,
    Properties,
    About,
    CloseApp,
    ConfirmSave,
    ConfirmDontSave,
    ConfirmCancel,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogHit {
    Ok,
    Cancel,
    Field(usize),
    Palette(usize),
    Preset(usize),
    Slider(SliderChannel),
    ConfirmSave,
    ConfirmDontSave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CtlStyle {
    Plain,
    Qat,
    BigText,
    SmallText,
    Tool,
    Shape,
    SizeButton,
    Color1,
    Color2,
    PaletteCell,
    EditColors,
    Checkbox,
    SpinUp,
    SpinDown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Control {
    pub rect: Rect,
    pub icon: Icon,
    pub label: Option<&'static str>,
    pub action: Action,
    pub selected: bool,
    pub check: Option<bool>,
    pub swatch: Option<Color>,
    pub style: CtlStyle,
    pub arrow: bool,
}

impl Control {
    pub fn new(rect: Rect, icon: Icon, action: Action) -> Self {
        Self {
            rect,
            icon,
            label: None,
            action,
            selected: false,
            check: None,
            swatch: None,
            style: CtlStyle::Plain,
            arrow: false,
        }
    }

    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_style(mut self, style: CtlStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_arrow(mut self) -> Self {
        self.arrow = true;
        self
    }

    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn with_check(mut self, checked: bool) -> Self {
        self.check = Some(checked);
        self
    }

    pub fn with_swatch(mut self, color: Color) -> Self {
        self.swatch = Some(color);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    pub fn from_points(a: Point, b: Point) -> Self {
        let x0 = a.x.min(b.x).floor() as i32;
        let y0 = a.y.min(b.y).floor() as i32;
        let x1 = a.x.max(b.x).ceil() as i32;
        let y1 = a.y.max(b.y).ceil() as i32;
        Self {
            x: x0,
            y: y0,
            w: (x1 - x0).max(1),
            h: (y1 - y0).max(1),
        }
    }

    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Self {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = (x + w).ceil() as i32;
        let y1 = (y + h).ceil() as i32;
        Self {
            x: x0,
            y: y0,
            w: (x1 - x0).max(1),
            h: (y1 - y0).max(1),
        }
    }

    pub fn clamp(self, width: u32, height: u32) -> Self {
        let x0 = self.x.clamp(0, width as i32);
        let y0 = self.y.clamp(0, height as i32);
        let x1 = (self.x + self.w).clamp(0, width as i32);
        let y1 = (self.y + self.h).clamp(0, height as i32);
        Self {
            x: x0,
            y: y0,
            w: (x1 - x0).max(1),
            h: (y1 - y0).max(1),
        }
    }

    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x as f32
            && p.y >= self.y as f32
            && p.x < (self.x + self.w) as f32
            && p.y < (self.y + self.h) as f32
    }

    pub fn as_rect(self) -> Rect {
        Rect::from_xywh(self.x as f32, self.y as f32, self.w as f32, self.h as f32)
    }
}

#[derive(Clone)]
pub struct Selection {
    pub rect: IRect,
    pub floating: Option<Pixels>,
}

#[derive(Clone, Debug)]
pub struct TextEditor {
    pub text: String,
    pub pos: Point,
    pub size: f32,
    pub angle_deg: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Dialog {
    Resize(ResizeDialog),
    Properties,
    About,
    EditColors(ColorEditor),
    ConfirmNew,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResizeDialog {
    pub width: String,
    pub height: String,
    pub focus: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorEditor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub target: Option<usize>,
    pub slider: Option<SliderChannel>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderChannel {
    R,
    G,
    B,
}

#[derive(Clone, Debug)]
pub struct ViewOptions {
    pub rulers: bool,
    pub grid: bool,
    pub status: bool,
    pub thumbnail: bool,
    pub fullscreen: bool,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            rulers: false,
            grid: false,
            status: true,
            thumbnail: false,
            fullscreen: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Drag {
    None,
    Draw {
        last: Point,
        button: PointerButton,
    },
    Shape {
        start: Point,
        current: Point,
        button: PointerButton,
    },
    Select {
        start: Point,
    },
    MoveSelection {
        start: Point,
        lifted: bool,
        content: Pixels,
        origin: IRect,
        undo_pushed: bool,
    },
    TextRotate {
        center: Point,
        start: f32,
        base: f32,
    },
    Pan {
        start: Point,
        scroll: Vec2,
    },
    ScrollH {
        grab: f32,
    },
    ScrollV {
        grab: f32,
    },
    ZoomSlider,
    DialogSlider(SliderChannel),
}

pub struct State {
    pub doc: Pixels,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub undo: Vec<Pixels>,
    pub redo: Vec<Pixels>,
    pub tool: Tool,
    pub brush: BrushKind,
    pub shape: ShapeKind,
    pub fill: FillMode,
    pub width_idx: usize,
    pub color1: Color,
    pub color2: Color,
    pub palette: Vec<Color>,
    pub zoom_idx: usize,
    pub scroll: Vec2,
    pub view: ViewOptions,
    pub tab: Tab,
    pub backstage: bool,
    pub dialog: Option<Dialog>,
    pub pending: Option<Action>,
    pub selection: Option<Selection>,
    pub clipboard: Option<Pixels>,
    pub text: Option<TextEditor>,
    pub recent: Vec<PathBuf>,
    pub status: Option<(String, Instant)>,
    pub mouse: Point,
    pub mouse_down: bool,
    pub drag: Drag,
    pub pointer: Point,
    pub rev: Signal<u64>,
}

impl State {
    pub fn new() -> Self {
        let palette = PALETTE_COLORS.to_vec();
        Self {
            doc: Pixels::filled(DEFAULT_DOC_W, DEFAULT_DOC_H, Color::WHITE),
            path: None,
            dirty: false,
            undo: Vec::new(),
            redo: Vec::new(),
            tool: Tool::Pencil,
            brush: BrushKind::Brush,
            shape: ShapeKind::Line,
            fill: FillMode::Outline,
            width_idx: 0,
            color1: Color::BLACK,
            color2: Color::WHITE,
            palette,
            zoom_idx: 3,
            scroll: Vec2::ZERO,
            view: ViewOptions::default(),
            tab: Tab::Home,
            backstage: false,
            dialog: None,
            pending: None,
            selection: None,
            clipboard: None,
            text: None,
            recent: Vec::new(),
            status: None,
            mouse: Point::new(-1.0, -1.0),
            mouse_down: false,
            drag: Drag::None,
            pointer: Point::ZERO,
            rev: Signal::new(0),
        }
    }

    pub fn bump(&self) {
        self.rev.set(self.rev.get_untracked().wrapping_add(1));
    }

    pub fn touch_doc(&mut self) {
        self.dirty = true;
        self.bump();
    }

    pub fn push_undo(&mut self) {
        self.undo.push(self.doc.clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            let current = std::mem::replace(&mut self.doc, prev);
            self.redo.push(current);
            if self.redo.len() > MAX_UNDO {
                self.redo.remove(0);
            }
            self.dirty = true;
            self.selection = None;
            self.text = None;
            self.bump();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let current = std::mem::replace(&mut self.doc, next);
            self.undo.push(current);
            if self.undo.len() > MAX_UNDO {
                self.undo.remove(0);
            }
            self.dirty = true;
            self.selection = None;
            self.text = None;
            self.bump();
        }
    }

    pub fn zoom(&self) -> f32 {
        ZOOMS[self.zoom_idx.min(ZOOMS.len() - 1)] / 100.0
    }

    pub fn zoom_percent(&self) -> f32 {
        ZOOMS[self.zoom_idx.min(ZOOMS.len() - 1)]
    }

    pub fn width(&self) -> f32 {
        LINE_WIDTHS[self.width_idx.min(LINE_WIDTHS.len() - 1)]
    }

    pub fn stroke_color(&self, button: PointerButton) -> Color {
        if button == PointerButton::Right {
            self.color2
        } else {
            self.color1
        }
    }

    pub fn fill_color(&self) -> Color {
        self.color2
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = Some((text.into(), Instant::now()));
        self.bump();
    }

    pub fn font_file_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    pub fn clear_selection_committing(&mut self) {
        if let Some(sel) = self.selection.take() {
            if let Some(pix) = sel.floating {
                self.push_undo();
                self.doc.blend_from(&pix, sel.rect.x, sel.rect.y);
                self.touch_doc();
            }
        }
    }

    pub fn commit_floating(&mut self) {
        let Some(sel) = self.selection.as_mut() else {
            return;
        };
        let Some(pix) = sel.floating.take() else {
            return;
        };
        self.doc.blend_from(&pix, sel.rect.x, sel.rect.y);
        self.touch_doc();
    }

    pub fn selection_rect(&self) -> Option<IRect> {
        self.selection.as_ref().map(|s| s.rect)
    }

    pub fn sync_zoom_scroll(&mut self) {
        self.scroll = Vec2::ZERO;
        self.bump();
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
