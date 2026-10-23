use sequoia_ui::prelude::*;

use crate::model::*;
use crate::model::{Dialog, Icon};

pub(crate) const QAT_H: f32 = 32.0;
pub(crate) const TABS_H: f32 = 25.0;
pub(crate) const RIBBON_H: f32 = 97.0;
pub(crate) const STATUS_H: f32 = 26.0;
pub(crate) const PAD: f32 = 12.0;
pub(crate) const RULER: f32 = 18.0;
pub(crate) const SCROLLBAR: f32 = 17.0;
pub(crate) const THUMB_W: f32 = 168.0;
pub(crate) const THUMB_H: f32 = 144.0;

const BODY_H: f32 = 74.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScrollAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollMetrics {
    pub content: f32,
    pub view: f32,
    pub offset: f32,
}

impl ScrollMetrics {
    pub(crate) fn new(content: f32, view: f32, offset: f32) -> Self {
        Self {
            content: content.max(0.0),
            view: view.max(0.0),
            offset: offset.max(0.0),
        }
    }

    pub(crate) fn max_offset(self) -> f32 {
        (self.content - self.view).max(0.0)
    }

    pub(crate) fn thumb_len(self, bar_len: f32) -> f32 {
        if self.content <= 0.0 {
            return bar_len;
        }
        let fraction = (self.view / self.content).clamp(0.08, 1.0);
        (bar_len * fraction).max(18.0).min(bar_len)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CanvasTransform {
    pub origin: Point,
    pub zoom: f32,
}

impl CanvasTransform {
    pub(crate) fn new(layout: &Layout, state: &State) -> Self {
        Self {
            origin: layout.canvas.origin,
            zoom: state.zoom(),
        }
    }

    pub(crate) fn to_screen(self, p: Point) -> Point {
        Point::new(
            self.origin.x + p.x * self.zoom,
            self.origin.y + p.y * self.zoom,
        )
    }

    pub(crate) fn to_doc(self, p: Point) -> Point {
        Point::new(
            (p.x - self.origin.x) / self.zoom,
            (p.y - self.origin.y) / self.zoom,
        )
    }

    pub(crate) fn rect_to_screen(self, rect: Rect) -> Rect {
        Rect::from_xywh(
            self.origin.x + rect.origin.x * self.zoom,
            self.origin.y + rect.origin.y * self.zoom,
            rect.size.width * self.zoom,
            rect.size.height * self.zoom,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct Bands {
    qat: Rect,
    tabs: Rect,
    ribbon: Rect,
    workspace: Rect,
    status: Rect,
}

#[derive(Clone, Copy, Debug)]
struct CanvasParts {
    viewport: Rect,
    canvas: Rect,
    scroll: Vec2,
    max_scroll: Vec2,
    top_ruler: Rect,
    left_ruler: Rect,
    ruler_corner: Rect,
    vscroll: Option<Rect>,
    hscroll: Option<Rect>,
}

#[derive(Clone, Copy, Debug)]
struct StatusParts {
    zoom_slider: Rect,
    zoom_minus: Rect,
    zoom_plus: Rect,
}

#[derive(Clone, Debug)]
struct BackstageParts {
    rect: Option<Rect>,
    items: Vec<BackItem>,
    close: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Group {
    pub rect: Rect,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BackItem {
    pub rect: Rect,
    pub action: Action,
    pub label: &'static str,
    pub icon: Icon,
}

#[derive(Clone, Debug)]
pub(crate) struct Layout {
    pub size: Size,
    pub qat: Rect,
    pub tabs: Rect,
    pub ribbon: Rect,
    pub workspace: Rect,
    pub status: Rect,
    pub viewport: Rect,
    pub canvas: Rect,
    pub scroll: Vec2,
    pub max_scroll: Vec2,
    pub top_ruler: Rect,
    pub left_ruler: Rect,
    pub ruler_corner: Rect,
    pub thumb_panel: Option<Rect>,
    pub thumb_image: Option<Rect>,
    pub vscroll: Option<Rect>,
    pub hscroll: Option<Rect>,
    pub groups: Vec<Group>,
    pub controls: Vec<Control>,
    pub tab_file: Rect,
    pub tab_home: Rect,
    pub tab_view: Rect,
    pub qat_caret: Rect,
    pub app_icon: Rect,
    pub caption_min: Rect,
    pub caption_max: Rect,
    pub caption_close: Rect,
    pub zoom_slider: Rect,
    pub zoom_minus: Rect,
    pub zoom_plus: Rect,
    pub backstage: Option<Rect>,
    pub back_items: Vec<BackItem>,
    pub back_close: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    None,
    CaptionClose,
    CaptionMinimize,
    CaptionMaximize,
    Qat,
    BackstageClose,
    BackstageItem(Action),
    FileTab,
    Tab(Tab),
    Control(Action),
    ZoomOut,
    ZoomIn,
    ZoomSlider,
    Scroll(ScrollAxis),
    Canvas,
}

impl Layout {
    pub(crate) fn hit_at(&self, state: &State, p: Point) -> Hit {
        if self.caption_close.contains(p) {
            return Hit::CaptionClose;
        }
        if self.caption_min.contains(p) {
            return Hit::CaptionMinimize;
        }
        if self.caption_max.contains(p) {
            return Hit::CaptionMaximize;
        }
        if state.backstage {
            if self.back_close.contains(p) {
                return Hit::BackstageClose;
            }
            if let Some(item) = self.back_items.iter().find(|i| i.rect.contains(p)) {
                return Hit::BackstageItem(item.action);
            }
            return Hit::None;
        }
        let on_qat_control = self
            .controls
            .iter()
            .any(|c| c.style == CtlStyle::Qat && c.rect.contains(p));
        if self.qat.contains(p) && !on_qat_control {
            return Hit::Qat;
        }
        if self.tab_file.contains(p) {
            return Hit::FileTab;
        }
        if self.tab_home.contains(p) {
            return Hit::Tab(Tab::Home);
        }
        if self.tab_view.contains(p) {
            return Hit::Tab(Tab::View);
        }
        if let Some(control) = self.controls.iter().find(|c| c.rect.contains(p)) {
            return Hit::Control(control.action);
        }
        if !state.view.fullscreen && state.view.status {
            if self.zoom_minus.contains(p) {
                return Hit::ZoomOut;
            }
            if self.zoom_plus.contains(p) {
                return Hit::ZoomIn;
            }
            if self.zoom_slider.contains(p) {
                return Hit::ZoomSlider;
            }
        }
        if self.vscroll.is_some_and(|v| v.contains(p)) {
            return Hit::Scroll(ScrollAxis::Vertical);
        }
        if self.hscroll.is_some_and(|h| h.contains(p)) {
            return Hit::Scroll(ScrollAxis::Horizontal);
        }
        if self.canvas.contains(p) || self.viewport.contains(p) {
            return Hit::Canvas;
        }
        Hit::None
    }
}

pub(crate) fn content_size(state: &State) -> Size {
    let zoom = state.zoom();
    Size::new(
        state.doc.width() as f32 * zoom + PAD * 2.0,
        state.doc.height() as f32 * zoom + PAD * 2.0,
    )
}

fn scrollbars_needed(content: Size, base: Size) -> (bool, bool) {
    let mut vertical = content.height > base.height;
    let mut horizontal = content.width > base.width;
    for _ in 0..2 {
        let view_h = base.height - if horizontal { SCROLLBAR } else { 0.0 };
        if !vertical && content.height > view_h {
            vertical = true;
        }
        let view_w = base.width - if vertical { SCROLLBAR } else { 0.0 };
        if !horizontal && content.width > view_w {
            horizontal = true;
        }
    }
    (vertical, horizontal)
}

pub(crate) fn scroll_metrics(state: &State, layout: &Layout, axis: ScrollAxis) -> ScrollMetrics {
    let content = content_size(state);
    match axis {
        ScrollAxis::Horizontal => {
            ScrollMetrics::new(content.width, layout.viewport.size.width, layout.scroll.x)
        }
        ScrollAxis::Vertical => {
            ScrollMetrics::new(content.height, layout.viewport.size.height, layout.scroll.y)
        }
    }
}

pub(crate) fn scroll_thumb(bar: Rect, axis: ScrollAxis, metrics: ScrollMetrics) -> Rect {
    if metrics.content <= 0.0 {
        return bar;
    }
    let (bar_len, origin) = match axis {
        ScrollAxis::Horizontal => ((bar.size.width - 4.0).max(0.0), bar.left() + 2.0),
        ScrollAxis::Vertical => ((bar.size.height - 4.0).max(0.0), bar.top() + 2.0),
    };
    let thumb_len = metrics.thumb_len(bar_len);
    let max_offset = metrics.max_offset();
    let t = if max_offset > 0.0 {
        (metrics.offset / max_offset).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let offset = (bar_len - thumb_len) * t;
    match axis {
        ScrollAxis::Horizontal => Rect::from_xywh(
            origin + offset,
            bar.top() + 2.0,
            thumb_len,
            (bar.size.height - 4.0).max(0.0),
        ),
        ScrollAxis::Vertical => Rect::from_xywh(
            bar.left() + 2.0,
            origin + offset,
            (bar.size.width - 4.0).max(0.0),
            thumb_len,
        ),
    }
}

fn bands(state: &State, size: Size) -> Bands {
    let full = state.view.fullscreen;
    let qat_h = if full { 0.0 } else { QAT_H };
    let tabs_h = if full { 0.0 } else { TABS_H };
    let ribbon_h = if full { 0.0 } else { RIBBON_H };
    let status_h = if !full && state.view.status {
        STATUS_H
    } else {
        0.0
    };
    let qat = Rect::from_xywh(0.0, 0.0, size.width, qat_h);
    let tabs = Rect::from_xywh(0.0, qat_h, size.width, tabs_h);
    let ribbon = Rect::from_xywh(0.0, tabs.bottom(), size.width, ribbon_h);
    let workspace = Rect::from_xywh(
        0.0,
        ribbon.bottom(),
        size.width,
        (size.height - ribbon.bottom() - status_h).max(0.0),
    );
    let status = Rect::from_xywh(0.0, workspace.bottom(), size.width, status_h);
    Bands {
        qat,
        tabs,
        ribbon,
        workspace,
        status,
    }
}

fn canvas_parts(state: &State, workspace: Rect, rulers: bool) -> CanvasParts {
    let ruler = if rulers { RULER } else { 0.0 };
    let base = Rect::from_xywh(
        workspace.left() + ruler,
        workspace.top() + ruler,
        (workspace.size.width - ruler).max(0.0),
        (workspace.size.height - ruler).max(0.0),
    );
    let content = content_size(state);
    let (need_v, need_h) = scrollbars_needed(content, base.size);
    let view_w = (base.size.width - if need_v { SCROLLBAR } else { 0.0 }).max(0.0);
    let view_h = (base.size.height - if need_h { SCROLLBAR } else { 0.0 }).max(0.0);
    let viewport = Rect::from_xywh(base.left(), base.top(), view_w, view_h);

    let max_scroll = Vec2::new(
        (content.width - view_w).max(0.0),
        (content.height - view_h).max(0.0),
    );
    let scroll = Vec2::new(
        if max_scroll.x > 0.0 {
            state.scroll.x.clamp(0.0, max_scroll.x)
        } else {
            0.0
        },
        if max_scroll.y > 0.0 {
            state.scroll.y.clamp(0.0, max_scroll.y)
        } else {
            0.0
        },
    );
    let zoom = state.zoom();
    let canvas = Rect::from_xywh(
        viewport.left() + PAD - scroll.x,
        viewport.top() + PAD - scroll.y,
        state.doc.width() as f32 * zoom,
        state.doc.height() as f32 * zoom,
    );

    let vscroll = if need_v {
        Some(Rect::from_xywh(
            base.right() - SCROLLBAR,
            base.top(),
            SCROLLBAR,
            view_h,
        ))
    } else {
        None
    };
    let hscroll = if need_h {
        Some(Rect::from_xywh(
            base.left(),
            base.bottom() - SCROLLBAR,
            view_w,
            SCROLLBAR,
        ))
    } else {
        None
    };
    let top_ruler = Rect::from_xywh(base.left(), workspace.top(), base.size.width, ruler);
    let left_ruler = Rect::from_xywh(workspace.left(), base.top(), ruler, base.size.height);
    let ruler_corner = Rect::from_xywh(workspace.left(), workspace.top(), ruler, ruler);

    CanvasParts {
        viewport,
        canvas,
        scroll,
        max_scroll,
        top_ruler,
        left_ruler,
        ruler_corner,
        vscroll,
        hscroll,
    }
}

fn status_parts(size: Size, status: Rect) -> StatusParts {
    let zoom_val_w = 38.0;
    let zoom_plus = Rect::from_xywh(
        size.width - 10.0 - zoom_val_w - 5.0 - 20.0,
        status.top() + 3.0,
        20.0,
        20.0,
    );
    let zoom_slider = Rect::from_xywh(
        zoom_plus.left() - 5.0 - 110.0,
        status.top() + 4.0,
        110.0,
        18.0,
    );
    let zoom_minus = Rect::from_xywh(
        zoom_slider.left() - 5.0 - 20.0,
        status.top() + 3.0,
        20.0,
        20.0,
    );
    StatusParts {
        zoom_slider,
        zoom_minus,
        zoom_plus,
    }
}

fn backstage_parts(state: &State, tabs: Rect, workspace: Rect) -> BackstageParts {
    let rect = if state.backstage && !state.view.fullscreen {
        Some(Rect::from_xywh(
            0.0,
            tabs.bottom(),
            workspace.size.width,
            (workspace.bottom() - tabs.bottom()).max(0.0),
        ))
    } else {
        None
    };
    let close = Rect::from_xywh(10.0, 10.0, 32.0, 32.0);
    let mut items = Vec::new();
    if let Some(backstage) = rect {
        let entries = [
            (Action::New, "New", Icon::File),
            (Action::Open, "Open", Icon::Folder),
            (Action::Save, "Save", Icon::Save),
            (Action::SaveAs, "Save as", Icon::Save),
            (Action::Print, "Print", Icon::Print),
            (
                Action::SetWallpaper,
                "Set as desktop background",
                Icon::Wallpaper,
            ),
            (Action::Properties, "Properties", Icon::Properties),
            (Action::About, "About", Icon::Info),
            (Action::CloseApp, "Close", Icon::Close),
        ];
        let mut y = backstage.top() + 58.0;
        for (action, label, icon) in entries {
            items.push(BackItem {
                rect: Rect::from_xywh(backstage.left() + 12.0, y, 250.0, 38.0),
                action,
                label,
                icon,
            });
            y += 38.0;
        }
    }
    BackstageParts { rect, items, close }
}

pub(crate) fn compute(state: &State, size: Size) -> Layout {
    let Bands {
        qat,
        tabs,
        ribbon,
        workspace,
        status,
    } = bands(state, size);
    let full = state.view.fullscreen;
    let qat_h = qat.size.height;

    let tab_file = Rect::from_xywh(3.0, tabs.top() + 1.0, 47.0, 24.0);
    let tab_home = Rect::from_xywh(tab_file.right() + 1.0, tabs.top() + 1.0, 130.0, 24.0);
    let tab_view = Rect::from_xywh(tab_home.right() + 1.0, tabs.top() + 1.0, 59.0, 24.0);

    let app_icon = Rect::from_xywh(7.0, qat.top() + 8.0, 16.0, 16.0);
    let qat_save = Rect::from_xywh(28.0, qat.top() + 4.0, 24.0, 24.0);
    let qat_undo = Rect::from_xywh(53.0, qat.top() + 4.0, 24.0, 24.0);
    let qat_redo = Rect::from_xywh(78.0, qat.top() + 4.0, 24.0, 24.0);
    let qat_caret = Rect::from_xywh(103.0, qat.top() + 8.0, 15.0, 15.0);
    let caption_close = Rect::from_xywh(qat.right() - 44.0, qat.top(), 44.0, qat_h);
    let caption_max = Rect::from_xywh(caption_close.left() - 44.0, qat.top(), 44.0, qat_h);
    let caption_min = Rect::from_xywh(caption_max.left() - 44.0, qat.top(), 44.0, qat_h);

    let CanvasParts {
        viewport,
        canvas,
        scroll,
        max_scroll,
        top_ruler,
        left_ruler,
        ruler_corner,
        vscroll,
        hscroll,
    } = canvas_parts(state, workspace, state.view.rulers && !full);
    let thumb_panel = if state.view.thumbnail && !full {
        Some(Rect::from_xywh(
            (size.width - 14.0 - THUMB_W).max(0.0),
            (size.height - 38.0 - THUMB_H).max(0.0),
            THUMB_W,
            THUMB_H,
        ))
    } else {
        None
    };
    let thumb_image = thumb_panel.map(|panel| {
        Rect::from_xywh(
            panel.left() + 4.0,
            panel.top() + 20.0,
            THUMB_W - 8.0,
            THUMB_H - 24.0,
        )
    });

    let StatusParts {
        zoom_slider,
        zoom_minus,
        zoom_plus,
    } = status_parts(size, status);

    let mut groups = Vec::new();
    let mut controls = Vec::new();
    if !full && !state.backstage {
        match state.tab {
            Tab::Home => build_home(ribbon, state, &mut groups, &mut controls),
            Tab::View => build_view(ribbon, state, &mut groups, &mut controls),
        }
        add_qat_controls(&mut controls, qat_save, qat_undo, qat_redo);
    }

    let BackstageParts {
        rect: backstage,
        items: back_items,
        close: back_close,
    } = backstage_parts(state, tabs, workspace);

    Layout {
        size,
        qat,
        tabs,
        ribbon,
        workspace,
        status,
        viewport,
        canvas,
        scroll,
        max_scroll,
        top_ruler,
        left_ruler,
        ruler_corner,
        thumb_panel,
        thumb_image,
        vscroll,
        hscroll,
        groups,
        controls,
        tab_file,
        tab_home,
        tab_view,
        qat_caret,
        app_icon,
        caption_min,
        caption_max,
        caption_close,
        zoom_slider,
        zoom_minus,
        zoom_plus,
        backstage,
        back_items,
        back_close,
    }
}

fn add_qat_controls(out: &mut Vec<Control>, save: Rect, undo: Rect, redo: Rect) {
    for (rect, icon, action) in [
        (save, Icon::Save, Action::Save),
        (undo, Icon::Undo, Action::Undo),
        (redo, Icon::Redo, Action::Redo),
    ] {
        out.push(Control::new(rect, icon, action).with_style(CtlStyle::Qat));
    }
}

fn rect_grid(x: f32, y: f32, w: f32, h: f32, cols: usize, rows: usize, gap: f32, i: usize) -> Rect {
    let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
    let ch = (h - gap * (rows as f32 - 1.0)) / rows as f32;
    let cx = i % cols;
    let cy = i / cols;
    Rect::from_xywh(
        x + cx as f32 * (cw + gap),
        y + cy as f32 * (ch + gap),
        cw,
        ch,
    )
}

fn make_groups(ribbon: Rect, specs: &[(f32, &'static str)]) -> Vec<Group> {
    let mut out = Vec::with_capacity(specs.len());
    let mut x = ribbon.left() + 3.0;
    for (w, label) in specs {
        out.push(Group {
            rect: Rect::from_xywh(x, ribbon.top(), *w, (ribbon.size.height - 1.0).max(0.0)),
            label,
        });
        x += *w;
    }
    out
}

fn body(group: &Group, left: f32, right: f32) -> Rect {
    Rect::from_xywh(
        group.rect.left() + left,
        group.rect.top() + 4.0,
        (group.rect.size.width - left - right).max(0.0),
        BODY_H,
    )
}

fn small(rect: Rect, icon: Icon, action: Action, label: &'static str) -> Control {
    Control::new(rect, icon, action)
        .with_label(label)
        .with_style(CtlStyle::SmallText)
}

fn big_button(rect: Rect, icon: Icon, action: Action, label: &'static str) -> Control {
    Control::new(rect, icon, action)
        .with_label(label)
        .with_style(CtlStyle::BigText)
        .with_arrow()
}

fn column(x: f32, top: f32, width: f32, height: f32, gap: f32, index: usize) -> Rect {
    Rect::from_xywh(x, top + index as f32 * (height + gap), width, height)
}

fn build_home(ribbon: Rect, state: &State, groups: &mut Vec<Group>, out: &mut Vec<Control>) {
    let specs = [
        (127.0, "Clipboard"),
        (190.0, "Image"),
        (96.0, "Tools"),
        (64.0, "Brushes"),
        (321.0, "Shapes"),
        (50.0, "Size"),
        (352.0, "Colors"),
    ];
    *groups = make_groups(ribbon, &specs);

    let clip = body(&groups[0], 2.0, 5.0);
    out.push(big_button(
        Rect::from_xywh(clip.left(), clip.top() + 2.0, 54.0, 70.0),
        Icon::Paste,
        Action::Paste,
        "Paste",
    ));
    let col_x = clip.left() + 56.0;
    let col_w = (clip.right() - col_x).max(20.0);
    out.push(small(
        Rect::from_xywh(col_x, clip.top() + 14.0, col_w, 22.0),
        Icon::Cut,
        Action::Cut,
        "Cut",
    ));
    out.push(small(
        Rect::from_xywh(col_x, clip.top() + 38.0, col_w, 22.0),
        Icon::Copy,
        Action::Copy,
        "Copy",
    ));

    let img = body(&groups[1], 5.0, 5.0);
    out.push(
        big_button(
            Rect::from_xywh(img.left(), img.top() + 11.0, 72.0, 52.0),
            Icon::Select,
            Action::SelectTool,
            "Select",
        )
        .with_selected(state.tool == Tool::Select),
    );
    let icol_x = img.left() + 74.0;
    let icol_w = (img.right() - icol_x).max(20.0);
    out.push(small(
        Rect::from_xywh(icol_x, img.top() + 2.0, icol_w, 22.0),
        Icon::Crop,
        Action::Crop,
        "Crop",
    ));
    out.push(small(
        Rect::from_xywh(icol_x, img.top() + 26.0, icol_w, 22.0),
        Icon::Resize,
        Action::ResizeDialog,
        "Resize",
    ));
    out.push(
        small(
            Rect::from_xywh(icol_x, img.top() + 50.0, icol_w, 22.0),
            Icon::RotateLeft,
            Action::RotateLeft,
            "Rotate",
        )
        .with_arrow(),
    );

    let tools = body(&groups[2], 5.0, 5.0);
    let tool_acts = [
        (Tool::Pencil, Icon::Pencil),
        (Tool::Fill, Icon::Fill),
        (Tool::Text, Icon::Text),
        (Tool::Eraser, Icon::Eraser),
        (Tool::Picker, Icon::Picker),
        (Tool::Magnifier, Icon::Magnifier),
    ];
    for (i, (tool, icon)) in tool_acts.iter().enumerate() {
        out.push(
            Control::new(
                rect_grid(tools.left(), tools.top() + 10.0, 86.0, 53.0, 3, 2, 1.0, i),
                *icon,
                Action::SetTool(*tool),
            )
            .with_selected(state.tool == *tool)
            .with_style(CtlStyle::Tool),
        );
    }

    let brushes = body(&groups[3], 5.0, 5.0);
    out.push(big_button(
        Rect::from_xywh(brushes.left(), brushes.top() + 2.0, 54.0, 70.0),
        Icon::Brush,
        Action::CycleBrush,
        "Brushes",
    ));

    let shapes = body(&groups[4], 5.0, 5.0);
    out.push(
        small(
            Rect::from_xywh(shapes.left(), shapes.top() + 13.0, 97.0, 22.0),
            Icon::Outline,
            Action::SetFill(FillMode::Outline),
            "Outline",
        )
        .with_arrow(),
    );
    out.push(
        small(
            Rect::from_xywh(shapes.left(), shapes.top() + 38.0, 97.0, 22.0),
            Icon::FillDropdown,
            Action::CycleFill,
            "Fill",
        )
        .with_arrow(),
    );
    let grid = Rect::from_xywh(shapes.left() + 101.0, shapes.top() + 5.0, 190.0, 64.0);
    for (i, kind) in ShapeKind::ALL.iter().enumerate() {
        out.push(
            Control::new(
                Rect::from_xywh(
                    grid.left() + 1.0 + (i % 8) as f32 * 23.5,
                    grid.top() + 1.0 + (i / 8) as f32 * 20.5,
                    23.5,
                    20.5,
                ),
                Icon::Shape(*kind),
                Action::SetShape(*kind),
            )
            .with_selected(state.shape == *kind && state.tool == Tool::Shape)
            .with_style(CtlStyle::Shape),
        );
    }
    for (rect, style) in [
        (
            Rect::from_xywh(grid.right(), grid.top(), 16.0, 32.0),
            CtlStyle::SpinUp,
        ),
        (
            Rect::from_xywh(grid.right(), grid.top() + 32.0, 16.0, 32.0),
            CtlStyle::SpinDown,
        ),
    ] {
        out.push(
            Control::new(rect, Icon::Shape(ShapeKind::Triangle), Action::None).with_style(style),
        );
    }

    let size = body(&groups[5], 5.0, 5.0);
    out.push(
        Control::new(
            Rect::from_xywh(size.left(), size.top() + 4.0, 40.0, 66.0),
            Icon::Size,
            Action::CycleWidth,
        )
        .with_style(CtlStyle::SizeButton)
        .with_arrow(),
    );

    let colors = body(&groups[6], 10.0, 5.0);
    let slot_y = colors.top() + 3.0;
    out.push(
        Control::new(
            Rect::from_xywh(colors.left(), slot_y, 44.0, 68.0),
            Icon::Check,
            Action::SetColor1,
        )
        .with_swatch(state.color1)
        .with_selected(true)
        .with_style(CtlStyle::Color1)
        .with_label("Color 1"),
    );
    out.push(
        Control::new(
            Rect::from_xywh(colors.left() + 47.0, slot_y, 44.0, 68.0),
            Icon::Check,
            Action::SetColor2,
        )
        .with_swatch(state.color2)
        .with_style(CtlStyle::Color2)
        .with_label("Color 2"),
    );
    let palette = Rect::from_xywh(colors.left() + 94.0, colors.top() + 18.0, 174.0, 38.0);
    for (i, color) in state.palette.iter().take(20).enumerate() {
        out.push(
            Control::new(
                Rect::from_xywh(
                    palette.left() + 2.0 + (i % 10) as f32 * 17.0,
                    palette.top() + 2.0 + (i / 10) as f32 * 17.0,
                    17.0,
                    17.0,
                ),
                Icon::Check,
                Action::Palette(i),
            )
            .with_swatch(*color)
            .with_selected(*color == state.color1 || *color == state.color2)
            .with_style(CtlStyle::PaletteCell),
        );
    }
    out.push(
        Control::new(
            Rect::from_xywh(colors.left() + 271.0, slot_y, 66.0, 68.0),
            Icon::Wheel,
            Action::EditColors,
        )
        .with_label("Edit colors")
        .with_style(CtlStyle::EditColors),
    );
}

fn build_view(ribbon: Rect, state: &State, groups: &mut Vec<Group>, out: &mut Vec<Control>) {
    let specs = [(118.0, "Zoom"), (140.0, "Show or hide"), (130.0, "Display")];
    *groups = make_groups(ribbon, &specs);

    let zoom = body(&groups[0], 5.0, 5.0);
    let zoom_acts = [
        (Icon::ZoomIn, Action::ZoomIn, "Zoom in"),
        (Icon::ZoomOut, Action::ZoomOut, "Zoom out"),
        (Icon::ZoomReset, Action::Zoom100, "100%"),
    ];
    for (i, (icon, action, label)) in zoom_acts.iter().enumerate() {
        out.push(small(
            column(zoom.left(), zoom.top() + 3.0, 98.0, 22.0, 1.0, i),
            *icon,
            *action,
            label,
        ));
    }

    let show = body(&groups[1], 5.0, 5.0);
    let items = [
        ("Rulers", Action::ToggleRulers, state.view.rulers),
        ("Gridlines", Action::ToggleGrid, state.view.grid),
        ("Status bar", Action::ToggleStatus, state.view.status),
    ];
    for (i, (label, action, on)) in items.iter().enumerate() {
        out.push(
            Control::new(
                column(show.left(), show.top() + 3.0, 120.0, 22.0, 1.0, i),
                Icon::Check,
                *action,
            )
            .with_label(label)
            .with_check(*on)
            .with_style(CtlStyle::Checkbox),
        );
    }

    let display = body(&groups[2], 5.0, 5.0);
    let view_acts = [
        (Icon::Fullscreen, Action::ToggleFullscreen, "Full screen"),
        (Icon::Thumbnail, Action::ToggleThumbnail, "Thumbnail"),
    ];
    for (i, (icon, action, label)) in view_acts.iter().enumerate() {
        out.push(small(
            column(display.left(), display.top() + 13.0, 110.0, 22.0, 1.0, i),
            *icon,
            *action,
            label,
        ));
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DialogLayout {
    pub card: Rect,
    pub ok: Rect,
    pub cancel: Rect,
    pub close: Rect,
    pub fields: Vec<Rect>,
    pub sliders: Vec<(SliderChannel, Rect)>,
    pub palette: Vec<Rect>,
    pub presets: Vec<Rect>,
    pub save: Rect,
    pub dont_save: Rect,
}

impl DialogLayout {
    pub(crate) fn hit_at(&self, state: &State, p: Point) -> Option<DialogHit> {
        if self.close.contains(p) {
            return Some(DialogHit::Cancel);
        }
        match state.dialog.as_ref()? {
            Dialog::ConfirmNew => {
                if self.save.contains(p) {
                    Some(DialogHit::ConfirmSave)
                } else if self.dont_save.contains(p) {
                    Some(DialogHit::ConfirmDontSave)
                } else if self.cancel.contains(p) {
                    Some(DialogHit::Cancel)
                } else {
                    None
                }
            }
            Dialog::Resize(_) => {
                for (i, field) in self.fields.iter().enumerate() {
                    if field.contains(p) {
                        return Some(DialogHit::Field(i));
                    }
                }
                if self.ok.contains(p) {
                    Some(DialogHit::Ok)
                } else if self.cancel.contains(p) {
                    Some(DialogHit::Cancel)
                } else {
                    None
                }
            }
            Dialog::EditColors(_) => {
                for (channel, track) in &self.sliders {
                    if track.contains(p) {
                        return Some(DialogHit::Slider(*channel));
                    }
                }
                for (i, rect) in self.palette.iter().enumerate() {
                    if rect.contains(p) {
                        return Some(DialogHit::Palette(i));
                    }
                }
                for (i, rect) in self.presets.iter().enumerate() {
                    if rect.contains(p) {
                        return Some(DialogHit::Preset(i));
                    }
                }
                if self.ok.contains(p) {
                    Some(DialogHit::Ok)
                } else if self.cancel.contains(p) {
                    Some(DialogHit::Cancel)
                } else {
                    None
                }
            }
            _ => {
                if self.ok.contains(p) {
                    Some(DialogHit::Ok)
                } else if self.cancel.contains(p) {
                    Some(DialogHit::Cancel)
                } else {
                    None
                }
            }
        }
    }
}

pub(crate) fn dialog_layout(state: &State, size: Size) -> Option<DialogLayout> {
    let dialog = state.dialog.as_ref()?;
    let (w, h) = match dialog {
        Dialog::Resize(_) => (360.0, 262.0),
        Dialog::Properties => (330.0, 180.0),
        Dialog::About => (380.0, 220.0),
        Dialog::EditColors(_) => (420.0, 400.0),
        Dialog::ConfirmNew => (430.0, 180.0),
    };
    let card = Rect::from_xywh(
        ((size.width - w) * 0.5).max(0.0),
        ((size.height - h) * 0.5).max(0.0),
        w,
        h,
    );
    let close = Rect::from_xywh(card.right() - 30.0, card.top() + 6.0, 22.0, 22.0);
    let ok = Rect::from_xywh(card.right() - 100.0, card.bottom() - 42.0, 86.0, 28.0);
    let cancel = Rect::from_xywh(card.right() - 196.0, card.bottom() - 42.0, 86.0, 28.0);
    let save = Rect::from_xywh(card.right() - 214.0, card.bottom() - 42.0, 92.0, 28.0);
    let dont_save = Rect::from_xywh(card.right() - 322.0, card.bottom() - 42.0, 100.0, 28.0);
    let mut fields = Vec::new();
    let mut sliders = Vec::new();
    let mut palette = Vec::new();
    let mut presets = Vec::new();
    if let Dialog::Resize(r) = dialog {
        let _ = r;
        fields.push(Rect::from_xywh(
            card.left() + 130.0,
            card.top() + 66.0,
            120.0,
            28.0,
        ));
        fields.push(Rect::from_xywh(
            card.left() + 130.0,
            card.top() + 108.0,
            120.0,
            28.0,
        ));
    }
    if let Dialog::EditColors(_) = dialog {
        for i in 0..3 {
            sliders.push((
                match i {
                    0 => SliderChannel::R,
                    1 => SliderChannel::G,
                    _ => SliderChannel::B,
                },
                Rect::from_xywh(
                    card.left() + 116.0,
                    card.top() + 54.0 + i as f32 * 32.0,
                    300.0,
                    18.0,
                ),
            ));
        }
        for i in 0..20 {
            palette.push(rect_grid(
                card.left() + 24.0,
                card.top() + 178.0,
                card.size.width - 48.0,
                52.0,
                10,
                2,
                2.0,
                i,
            ));
        }
        for i in 0..24 {
            presets.push(rect_grid(
                card.left() + 24.0,
                card.top() + 248.0,
                card.size.width - 48.0,
                100.0,
                8,
                3,
                2.0,
                i,
            ));
        }
    }
    Some(DialogLayout {
        card,
        ok,
        cancel,
        close,
        fields,
        sliders,
        palette,
        presets,
        save,
        dont_save,
    })
}

pub(crate) fn zoom_from_x(layout: &Layout, x: f32) -> usize {
    let t = ((x - layout.zoom_slider.left()) / layout.zoom_slider.size.width).clamp(0.0, 1.0);
    (t * (ZOOMS.len() - 1) as f32).round() as usize
}

pub(crate) fn zoom_handle_x(layout: &Layout, state: &State) -> f32 {
    let t = state.zoom_idx as f32 / (ZOOMS.len() - 1) as f32;
    layout.zoom_slider.left() + layout.zoom_slider.size.width * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_ui::prelude::{Color, Pixels};

    fn inside(outer: Rect, inner: Rect) -> bool {
        inner.left() >= outer.left() - 0.01
            && inner.top() >= outer.top() - 0.01
            && inner.right() <= outer.right() + 0.01
            && inner.bottom() <= outer.bottom() + 0.01
    }

    fn edit_colors() -> Dialog {
        Dialog::EditColors(ColorEditor {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            target: None,
            slider: None,
        })
    }

    #[test]
    fn scrollbars_use_content_axis() {
        assert_eq!(
            scrollbars_needed(Size::new(100.0, 2000.0), Size::new(800.0, 420.0)),
            (true, false)
        );
        assert_eq!(
            scrollbars_needed(Size::new(2000.0, 100.0), Size::new(800.0, 420.0)),
            (false, true)
        );
        assert_eq!(
            scrollbars_needed(Size::new(300.0, 200.0), Size::new(800.0, 420.0)),
            (false, false)
        );
    }

    #[test]
    fn tall_document_only_needs_vertical_scrollbar() {
        let mut state = State::new();
        state.doc = Pixels::filled(100, 2000, Color::WHITE);
        let l = compute(&state, Size::new(800.0, 600.0));
        assert!(l.vscroll.is_some());
        assert!(l.hscroll.is_none());
    }

    #[test]
    fn wide_document_only_needs_horizontal_scrollbar() {
        let mut state = State::new();
        state.doc = Pixels::filled(2000, 100, Color::WHITE);
        let l = compute(&state, Size::new(800.0, 600.0));
        assert!(l.vscroll.is_none());
        assert!(l.hscroll.is_some());
    }

    #[test]
    fn scroll_is_clamped_to_content() {
        let mut state = State::new();
        state.doc = Pixels::filled(100, 2000, Color::WHITE);
        state.scroll.y = 99_999.0;
        let l = compute(&state, Size::new(800.0, 600.0));
        assert_eq!(l.scroll.y, l.max_scroll.y);
        assert!(l.max_scroll.y > 0.0);
    }

    #[test]
    fn ruler_band_stays_inside_workspace() {
        let mut state = State::new();
        state.view.rulers = true;
        state.doc = Pixels::filled(400, 300, Color::WHITE);
        let l = compute(&state, Size::new(800.0, 600.0));
        assert!(l.viewport.right() <= l.workspace.right());
        assert!(l.viewport.bottom() <= l.workspace.bottom());
        assert_eq!(l.top_ruler.right(), l.workspace.right());
    }

    #[test]
    fn scroll_thumb_round_trips() {
        let bar = Rect::from_xywh(0.0, 0.0, 17.0, 200.0);
        let metrics = ScrollMetrics::new(1000.0, 500.0, 0.0);
        let top = scroll_thumb(bar, ScrollAxis::Vertical, metrics);
        assert!((top.size.height - 98.0).abs() < 0.01);
        assert!((top.top() - bar.top() - 2.0).abs() < 0.01);

        let metrics = ScrollMetrics::new(1000.0, 500.0, 500.0);
        let bottom = scroll_thumb(bar, ScrollAxis::Vertical, metrics);
        assert!((bottom.bottom() - (bar.bottom() - 2.0)).abs() < 0.01);
    }

    #[test]
    fn zoom_slider_round_trips() {
        let state = State::new();
        let l = compute(&state, Size::new(1360.0, 860.0));
        let x = zoom_handle_x(&l, &state);
        assert_eq!(zoom_from_x(&l, x), state.zoom_idx);
    }

    #[test]
    fn dialog_controls_stay_inside_card() {
        let dialogs = [
            Dialog::Resize(ResizeDialog {
                width: "800".into(),
                height: "600".into(),
                focus: 0,
            }),
            Dialog::Properties,
            Dialog::About,
            edit_colors(),
            Dialog::ConfirmNew,
        ];
        for dialog in dialogs {
            let mut state = State::new();
            state.dialog = Some(dialog);
            let dl = dialog_layout(&state, Size::new(1360.0, 860.0)).unwrap();
            for rect in [dl.ok, dl.cancel, dl.close, dl.save, dl.dont_save] {
                assert!(inside(dl.card, rect));
            }
            for rect in &dl.fields {
                assert!(inside(dl.card, *rect));
            }
            for (_, rect) in &dl.sliders {
                assert!(inside(dl.card, *rect));
            }
            for rect in dl.palette.iter().chain(&dl.presets) {
                assert!(inside(dl.card, *rect));
            }
        }
    }

    #[test]
    fn ribbon_controls_stay_inside_chrome() {
        let state = State::new();
        let l = compute(&state, Size::new(1360.0, 860.0));
        assert!(!l.controls.is_empty());
        for control in &l.controls {
            assert!(inside(l.qat, control.rect) || inside(l.ribbon, control.rect));
        }
    }

    #[test]
    fn hit_test_follows_visual_order() {
        let state = State::new();
        let l = compute(&state, Size::new(1360.0, 860.0));
        assert_eq!(
            l.hit_at(&state, l.caption_close.center()),
            Hit::CaptionClose
        );
        assert_eq!(
            l.hit_at(&state, l.caption_min.center()),
            Hit::CaptionMinimize
        );
        assert_eq!(
            l.hit_at(&state, l.caption_max.center()),
            Hit::CaptionMaximize
        );
        assert_eq!(l.hit_at(&state, l.tab_file.center()), Hit::FileTab);
        assert_eq!(l.hit_at(&state, l.tab_home.center()), Hit::Tab(Tab::Home));
        assert_eq!(l.hit_at(&state, l.tab_view.center()), Hit::Tab(Tab::View));
        assert_eq!(
            l.hit_at(&state, l.controls[0].rect.center()),
            Hit::Control(Action::Paste)
        );
        assert_eq!(l.hit_at(&state, l.zoom_slider.center()), Hit::ZoomSlider);
        assert_eq!(l.hit_at(&state, l.canvas.center()), Hit::Canvas);
        assert_eq!(l.hit_at(&state, Point::new(0.0, 0.0)), Hit::Qat);
    }

    #[test]
    fn hit_test_handles_backstage_and_scrollbars() {
        let mut state = State::new();
        state.backstage = true;
        let l = compute(&state, Size::new(1360.0, 860.0));
        assert_eq!(l.hit_at(&state, l.back_close.center()), Hit::BackstageClose);
        assert_eq!(
            l.hit_at(&state, l.back_items[0].rect.center()),
            Hit::BackstageItem(Action::New)
        );

        state.backstage = false;
        state.doc = Pixels::filled(100, 2000, Color::WHITE);
        let l = compute(&state, Size::new(800.0, 600.0));
        assert_eq!(
            l.hit_at(&state, l.vscroll.unwrap().center()),
            Hit::Scroll(ScrollAxis::Vertical)
        );
    }

    #[test]
    fn dialog_hit_test_matches_controls() {
        let mut state = State::new();
        state.dialog = Some(edit_colors());
        let dl = dialog_layout(&state, Size::new(1360.0, 860.0)).unwrap();
        assert_eq!(
            dl.hit_at(&state, dl.palette[0].center()),
            Some(DialogHit::Palette(0))
        );
        assert_eq!(
            dl.hit_at(&state, dl.presets[0].center()),
            Some(DialogHit::Preset(0))
        );
        assert_eq!(
            dl.hit_at(&state, dl.sliders[0].1.center()),
            Some(DialogHit::Slider(SliderChannel::R))
        );
        assert_eq!(dl.hit_at(&state, dl.ok.center()), Some(DialogHit::Ok));
        assert_eq!(
            dl.hit_at(&state, dl.close.center()),
            Some(DialogHit::Cancel)
        );
        assert_eq!(
            dl.hit_at(
                &state,
                Point::new(dl.card.left() + 5.0, dl.card.top() + 5.0)
            ),
            None
        );
    }
}
