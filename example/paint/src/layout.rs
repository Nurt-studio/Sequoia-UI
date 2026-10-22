use sequoia_ui::prelude::*;

use crate::model::*;
use crate::model::{Dialog, Icon};

pub const QAT_H: f32 = 32.0;
pub const TABS_H: f32 = 25.0;
pub const RIBBON_H: f32 = 97.0;
pub const STATUS_H: f32 = 26.0;
pub const PAD: f32 = 12.0;
pub const RULER: f32 = 18.0;
pub const SCROLLBAR: f32 = 17.0;
pub const THUMB_W: f32 = 168.0;
pub const THUMB_H: f32 = 144.0;

const BODY_H: f32 = 74.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Group {
    pub rect: Rect,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackItem {
    pub rect: Rect,
    pub action: Action,
    pub label: &'static str,
    pub icon: Icon,
}

#[derive(Clone, Debug)]
pub struct Layout {
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
    pub qat_save: Rect,
    pub qat_undo: Rect,
    pub qat_redo: Rect,
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

pub fn compute(state: &State, size: Size) -> Layout {
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

    let rulers = state.view.rulers && !full;
    let r = if rulers { RULER } else { 0.0 };

    let base = Rect::from_xywh(
        workspace.left() + r,
        workspace.top() + r,
        workspace.size.width.max(0.0),
        (workspace.size.height - r).max(0.0),
    );
    let zoom = state.zoom();
    let content_w = state.doc.width() as f32 * zoom + PAD * 2.0;
    let content_h = state.doc.height() as f32 * zoom + PAD * 2.0;
    let mut need_v = content_w > base.size.width;
    let mut need_h = content_h > base.size.height;
    if need_v && content_w <= base.size.width - SCROLLBAR {
        need_v = false;
    }
    if need_h && content_h <= base.size.height - SCROLLBAR {
        need_h = false;
    }
    let view_w = (base.size.width - if need_v { SCROLLBAR } else { 0.0 }).max(0.0);
    let view_h = (base.size.height - if need_h { SCROLLBAR } else { 0.0 }).max(0.0);
    if content_w <= view_w {
        need_v = false;
    }
    if content_h <= view_h {
        need_h = false;
    }
    if need_v && content_w <= base.size.width - SCROLLBAR {
        need_v = false;
    }
    if need_h && content_h <= base.size.height - SCROLLBAR {
        need_h = false;
    }
    let view_w = (base.size.width - if need_v { SCROLLBAR } else { 0.0 }).max(0.0);
    let view_h = (base.size.height - if need_h { SCROLLBAR } else { 0.0 }).max(0.0);
    let viewport = Rect::from_xywh(base.left(), base.top(), view_w, view_h);

    let max_scroll = Vec2::new((content_w - view_w).max(0.0), (content_h - view_h).max(0.0));
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
    let top_ruler = Rect::from_xywh(base.left(), workspace.top(), base.size.width, r);
    let left_ruler = Rect::from_xywh(workspace.left(), base.top(), r, base.size.height);
    let ruler_corner = Rect::from_xywh(workspace.left(), workspace.top(), r, r);
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

    let mut groups = Vec::new();
    let mut controls = Vec::new();
    if !full && !state.backstage {
        match state.tab {
            Tab::Home => build_home(ribbon, state, &mut groups, &mut controls),
            Tab::View => build_view(ribbon, state, &mut groups, &mut controls),
        }
        add_qat_controls(&mut controls, qat_save, qat_undo, qat_redo);
    }

    let backstage = if state.backstage && !full {
        Some(Rect::from_xywh(
            0.0,
            tabs.bottom(),
            size.width,
            (workspace.bottom() - tabs.bottom()).max(0.0),
        ))
    } else {
        None
    };
    let mut back_items = Vec::new();
    let back_close = Rect::from_xywh(10.0, 10.0, 32.0, 32.0);
    if let Some(bs) = backstage {
        let items = [
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
        let mut y = bs.top() + 58.0;
        for (action, label, icon) in items {
            back_items.push(BackItem {
                rect: Rect::from_xywh(bs.left() + 12.0, y, 250.0, 38.0),
                action,
                label,
                icon,
            });
            y += 38.0;
        }
    }

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
        qat_save,
        qat_undo,
        qat_redo,
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
    let mut save = Control::new(save, Icon::Save, Action::Save);
    save.style = CtlStyle::Qat;
    out.push(save);
    let mut undo = Control::new(undo, Icon::Undo, Action::Undo);
    undo.style = CtlStyle::Qat;
    out.push(undo);
    let mut redo = Control::new(redo, Icon::Redo, Action::Redo);
    redo.style = CtlStyle::Qat;
    out.push(redo);
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
    let mut c = Control::new(rect, icon, action);
    c.label = Some(label);
    c.style = CtlStyle::SmallText;
    c
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
    let mut paste = Control::new(
        Rect::from_xywh(clip.left(), clip.top() + 2.0, 54.0, 70.0),
        Icon::Paste,
        Action::Paste,
    );
    paste.label = Some("Paste");
    paste.style = CtlStyle::BigText;
    paste.arrow = true;
    out.push(paste);
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
    let mut select = Control::new(
        Rect::from_xywh(img.left(), img.top() + 11.0, 72.0, 52.0),
        Icon::Select,
        Action::SelectTool,
    );
    select.label = Some("Select");
    select.style = CtlStyle::BigText;
    select.arrow = true;
    select.selected = state.tool == Tool::Select;
    out.push(select);
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
    let mut rotate = small(
        Rect::from_xywh(icol_x, img.top() + 50.0, icol_w, 22.0),
        Icon::RotateLeft,
        Action::RotateLeft,
        "Rotate",
    );
    rotate.arrow = true;
    out.push(rotate);

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
        let mut c = Control::new(
            rect_grid(tools.left(), tools.top() + 10.0, 86.0, 53.0, 3, 2, 1.0, i),
            *icon,
            Action::SetTool(*tool),
        );
        c.selected = state.tool == *tool;
        c.style = CtlStyle::Tool;
        out.push(c);
    }

    let brushes = body(&groups[3], 5.0, 5.0);
    let mut brushbtn = Control::new(
        Rect::from_xywh(brushes.left(), brushes.top() + 2.0, 54.0, 70.0),
        Icon::Brush,
        Action::CycleBrush,
    );
    brushbtn.label = Some("Brushes");
    brushbtn.style = CtlStyle::BigText;
    brushbtn.arrow = true;
    out.push(brushbtn);

    let shapes = body(&groups[4], 5.0, 5.0);
    let mut outline = small(
        Rect::from_xywh(shapes.left(), shapes.top() + 13.0, 97.0, 22.0),
        Icon::Outline,
        Action::SetFill(FillMode::Outline),
        "Outline",
    );
    outline.arrow = true;
    out.push(outline);
    let mut filldd = small(
        Rect::from_xywh(shapes.left(), shapes.top() + 38.0, 97.0, 22.0),
        Icon::FillDropdown,
        Action::CycleFill,
        "Fill",
    );
    filldd.arrow = true;
    out.push(filldd);
    let grid = Rect::from_xywh(shapes.left() + 101.0, shapes.top() + 5.0, 190.0, 64.0);
    for (i, kind) in ShapeKind::ALL.iter().enumerate() {
        let mut c = Control::new(
            Rect::from_xywh(
                grid.left() + 1.0 + (i % 8) as f32 * 23.5,
                grid.top() + 1.0 + (i / 8) as f32 * 20.5,
                23.5,
                20.5,
            ),
            Icon::Shape(*kind),
            Action::SetShape(*kind),
        );
        c.selected = state.shape == *kind && state.tool == Tool::Shape;
        c.style = CtlStyle::Shape;
        out.push(c);
    }
    let mut spin_up = Control::new(
        Rect::from_xywh(grid.right(), grid.top(), 16.0, 32.0),
        Icon::Shape(ShapeKind::Triangle),
        Action::None,
    );
    spin_up.style = CtlStyle::SpinUp;
    out.push(spin_up);
    let mut spin_down = Control::new(
        Rect::from_xywh(grid.right(), grid.top() + 32.0, 16.0, 32.0),
        Icon::Shape(ShapeKind::Triangle),
        Action::None,
    );
    spin_down.style = CtlStyle::SpinDown;
    out.push(spin_down);

    let size = body(&groups[5], 5.0, 5.0);
    let mut sizebtn = Control::new(
        Rect::from_xywh(size.left(), size.top() + 4.0, 40.0, 66.0),
        Icon::Size,
        Action::CycleWidth,
    );
    sizebtn.style = CtlStyle::SizeButton;
    sizebtn.arrow = true;
    out.push(sizebtn);

    let colors = body(&groups[6], 10.0, 5.0);
    let slot_y = colors.top() + 3.0;
    let mut color1 = Control::new(
        Rect::from_xywh(colors.left(), slot_y, 44.0, 68.0),
        Icon::Check,
        Action::SetColor1,
    );
    color1.swatch = Some(state.color1);
    color1.selected = true;
    color1.style = CtlStyle::Color1;
    color1.label = Some("Color 1");
    out.push(color1);
    let mut color2 = Control::new(
        Rect::from_xywh(colors.left() + 47.0, slot_y, 44.0, 68.0),
        Icon::Check,
        Action::SetColor2,
    );
    color2.swatch = Some(state.color2);
    color2.style = CtlStyle::Color2;
    color2.label = Some("Color 2");
    out.push(color2);
    let palette = Rect::from_xywh(colors.left() + 94.0, colors.top() + 18.0, 174.0, 38.0);
    for (i, color) in state.palette.iter().take(20).enumerate() {
        let mut c = Control::new(
            Rect::from_xywh(
                palette.left() + 2.0 + (i % 10) as f32 * 17.0,
                palette.top() + 2.0 + (i / 10) as f32 * 17.0,
                17.0,
                17.0,
            ),
            Icon::Check,
            Action::Palette(i),
        );
        c.swatch = Some(*color);
        c.selected = *color == state.color1 || *color == state.color2;
        c.style = CtlStyle::PaletteCell;
        out.push(c);
    }
    let mut edit = Control::new(
        Rect::from_xywh(colors.left() + 271.0, slot_y, 66.0, 68.0),
        Icon::Wheel,
        Action::EditColors,
    );
    edit.label = Some("Edit colors");
    edit.style = CtlStyle::EditColors;
    out.push(edit);
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
            Rect::from_xywh(zoom.left(), zoom.top() + 3.0 + i as f32 * 23.0, 98.0, 22.0),
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
        let mut c = Control::new(
            Rect::from_xywh(show.left(), show.top() + 3.0 + i as f32 * 23.0, 120.0, 22.0),
            Icon::Check,
            *action,
        );
        c.label = Some(*label);
        c.check = Some(*on);
        c.style = CtlStyle::Checkbox;
        out.push(c);
    }

    let display = body(&groups[2], 5.0, 5.0);
    let view_acts = [
        (Icon::Fullscreen, Action::ToggleFullscreen, "Full screen"),
        (Icon::Thumbnail, Action::ToggleThumbnail, "Thumbnail"),
    ];
    for (i, (icon, action, label)) in view_acts.iter().enumerate() {
        out.push(small(
            Rect::from_xywh(
                display.left(),
                display.top() + 13.0 + i as f32 * 23.0,
                110.0,
                22.0,
            ),
            *icon,
            *action,
            label,
        ));
    }
}

pub fn tab_at(layout: &Layout, p: Point) -> Option<Tab> {
    if layout.tab_file.contains(p) {
        return None;
    }
    if layout.tab_home.contains(p) {
        return Some(Tab::Home);
    }
    if layout.tab_view.contains(p) {
        return Some(Tab::View);
    }
    None
}

#[derive(Clone, Debug)]
pub struct DialogLayout {
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

pub fn dialog_layout(state: &State, size: Size) -> Option<DialogLayout> {
    let dialog = state.dialog.as_ref()?;
    let (w, h) = match dialog {
        Dialog::Resize(_) => (360.0, 262.0),
        Dialog::Properties => (330.0, 180.0),
        Dialog::About => (380.0, 220.0),
        Dialog::EditColors(_) => (420.0, 220.0),
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

pub fn zoom_from_x(layout: &Layout, x: f32) -> usize {
    let t = ((x - layout.zoom_slider.left()) / layout.zoom_slider.size.width).clamp(0.0, 1.0);
    (t * (ZOOMS.len() - 1) as f32).round() as usize
}

pub fn zoom_handle_x(layout: &Layout, state: &State) -> f32 {
    let t = state.zoom_idx as f32 / (ZOOMS.len() - 1) as f32;
    layout.zoom_slider.left() + layout.zoom_slider.size.width * t
}

pub fn scroll_thumb(layout: &Layout, bar: Rect, content: f32, view: f32, offset: f32) -> Rect {
    let _ = layout;
    if content <= 0.0 {
        return bar;
    }
    let frac = (view / content).clamp(0.08, 1.0);
    let len = (bar.size.height * frac).max(18.0).min(bar.size.height);
    let max_off = (content - view).max(0.0);
    let t = if max_off > 0.0 {
        (offset / max_off).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Rect::from_xywh(
        bar.left() + 2.0,
        bar.top() + (bar.size.height - len) * t,
        bar.size.width - 4.0,
        len,
    )
}
