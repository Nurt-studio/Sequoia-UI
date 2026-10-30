use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_ui::prelude::*;
use sequoia_ui::view::ElementId;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickerTarget {
    Fill,
    Stroke,
}

#[derive(Clone)]
pub(crate) struct ColorPickerState {
    pub(crate) open: Signal<bool>,
    pub(crate) target: Signal<PickerTarget>,
    pub(crate) hue: Signal<f32>,
    pub(crate) pos: Signal<Vec2>,
    pub(crate) dragging: Rc<Cell<bool>>,
    pub(crate) drag_offset: Rc<RefCell<Vec2>>,
    pub(crate) has_positioned: Rc<Cell<bool>>,
}

impl ColorPickerState {
    pub(crate) fn new() -> Self {
        Self {
            open: Signal::new(false),
            target: Signal::new(PickerTarget::Fill),
            hue: Signal::new(0.0),
            pos: Signal::new(Vec2::new(0.0, 0.0)),
            dragging: Rc::new(Cell::new(false)),
            drag_offset: Rc::new(RefCell::new(Vec2::new(0.0, 0.0))),
            has_positioned: Rc::new(Cell::new(false)),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Menu {
    pub(crate) open: Signal<bool>,
    pub(crate) anchor: Signal<Option<ElementId>>,
}

impl Menu {
    pub(crate) fn new() -> Self {
        Self {
            open: Signal::new(false),
            anchor: Signal::new(None),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Menus {
    pub(crate) file: Menu,
    pub(crate) edit: Menu,
    pub(crate) view: Menu,
    pub(crate) object: Menu,
    pub(crate) window: Menu,
    pub(crate) help: Menu,
}

impl Menus {
    pub(crate) fn new() -> Self {
        Self {
            file: Menu::new(),
            edit: Menu::new(),
            view: Menu::new(),
            object: Menu::new(),
            window: Menu::new(),
            help: Menu::new(),
        }
    }
    pub(crate) fn by_id(&self, id: u8) -> &Menu {
        match id {
            1 => &self.file,
            2 => &self.edit,
            3 => &self.view,
            4 => &self.object,
            5 => &self.window,
            _ => &self.help,
        }
    }
    pub(crate) fn others(&self, id: u8) -> Vec<Signal<bool>> {
        (1..=6u8)
            .filter(|i| *i != id)
            .map(|i| self.by_id(i).open.clone())
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Design,
    Inspect,
}

#[derive(Clone)]
pub(crate) struct PanelSections {
    pub(crate) alignment: Signal<bool>,
    pub(crate) position: Signal<bool>,
    pub(crate) size: Signal<bool>,
    pub(crate) appearance: Signal<bool>,
    pub(crate) auto_layout: Signal<bool>,
    pub(crate) fill: Signal<bool>,
    pub(crate) stroke: Signal<bool>,
    pub(crate) export: Signal<bool>,
}

impl PanelSections {
    pub(crate) fn new() -> Self {
        Self {
            alignment: Signal::new(true),
            position: Signal::new(true),
            size: Signal::new(true),
            appearance: Signal::new(true),
            auto_layout: Signal::new(true),
            fill: Signal::new(true),
            stroke: Signal::new(false),
            export: Signal::new(false),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToastKind {
    Info,
    Warn,
    Error,
}

#[derive(Clone, PartialEq)]
pub(crate) struct ToastEntry {
    pub(crate) id: u64,
    pub(crate) msg: String,
    pub(crate) kind: ToastKind,
}

#[derive(Clone)]
pub(crate) struct EditorState {
    pub(crate) doc_name: Signal<String>,
    pub(crate) zoom: Signal<f32>,
    pub(crate) status: Signal<String>,
    pub(crate) menus: Menus,
    pub(crate) tool: Signal<&'static str>,
    pub(crate) editing_doc: Signal<bool>,
    pub(crate) grid_on: Signal<bool>,
    pub(crate) pan_x: Signal<f32>,
    pub(crate) pan_y: Signal<f32>,
    pub(crate) viewport_w: Signal<f32>,
    pub(crate) viewport_h: Signal<f32>,
    pub(crate) ctx_menu_open: Signal<bool>,
    pub(crate) ctx_menu_x: Signal<f32>,
    pub(crate) ctx_menu_y: Signal<f32>,
    pub(crate) can_undo: Signal<bool>,
    pub(crate) can_redo: Signal<bool>,
    pub(crate) layers: Signal<Vec<Layer>>,
    pub(crate) selected_layer: Signal<Option<u64>>,
    pub(crate) layer_query: Signal<String>,
    pub(crate) sec_page_open: Signal<bool>,
    pub(crate) sec_comp_open: Signal<bool>,

    pub(crate) cmd_k_open: Signal<bool>,
    pub(crate) help_open: Signal<bool>,
    pub(crate) cmdk_query: Signal<String>,

    pub(crate) picker: ColorPickerState,

    pub(crate) editing_text: Signal<bool>,
    pub(crate) text_edit_anchor: Signal<(f32, f32, f32, f32)>,
    pub(crate) text_edit_orig: Rc<RefCell<String>>,
    pub(crate) editing_layer: Signal<Option<u64>>,
    pub(crate) rename_buffer: Signal<String>,
    pub(crate) last_canvas_click: Rc<RefCell<(Option<u64>, Instant)>>,

    pub(crate) toasts: Signal<Vec<ToastEntry>>,
    toast_seq: Rc<RefCell<u64>>,

    pub(crate) history: Rc<RefCell<Vec<Vec<Layer>>>>,
    pub(crate) future: Rc<RefCell<Vec<Vec<Layer>>>>,
    pub(crate) clipboard: Rc<RefCell<Vec<Layer>>>,
    bindings: Rc<RefCell<EffectScope>>,

    pub(crate) active_tab: Signal<Tab>,
    pub(crate) sections: PanelSections,

    pub(crate) sel_name: Signal<String>,
    pub(crate) sel_kind: Signal<String>,
    pub(crate) sel_x: Signal<f32>,
    pub(crate) sel_y: Signal<f32>,
    pub(crate) sel_w: Signal<f32>,
    pub(crate) sel_h: Signal<f32>,
    pub(crate) sel_opacity: Signal<f32>,
    pub(crate) sel_radius: Signal<f32>,
    pub(crate) sel_fill: Signal<Color>,
    pub(crate) sel_fill_hex: Signal<String>,
    pub(crate) sel_stroke: Signal<Color>,
    pub(crate) sel_stroke_hex: Signal<String>,
    pub(crate) sel_stroke_w: Signal<f32>,
    pub(crate) sel_text: Signal<String>,
    pub(crate) sel_font_size: Signal<f32>,

    pub(crate) sel_layout: Signal<String>,
    pub(crate) sel_gap: Signal<f32>,
    pub(crate) sel_pad_h: Signal<f32>,
    pub(crate) sel_pad_v: Signal<f32>,
    pub(crate) sel_align_h: Signal<String>,
    pub(crate) sel_align_v: Signal<String>,
}

impl EditorState {
    pub(crate) fn new() -> Self {
        Self::with_seed(Vec::new(), None)
    }
    pub(crate) fn demo() -> Self {
        Self::with_seed(demo_layers(), Some(2))
    }

    fn with_seed(layers_data: Vec<Layer>, selected: Option<u64>) -> Self {
        let state = Self {
            doc_name: Signal::new(String::from("Vision Design — Untitled")),
            zoom: Signal::new(1.0),
            status: Signal::new(String::new()),
            menus: Menus::new(),
            tool: Signal::new("V"),
            editing_doc: Signal::new(false),
            grid_on: Signal::new(true),
            pan_x: Signal::new(0.0),
            pan_y: Signal::new(0.0),
            viewport_w: Signal::new(0.0),
            viewport_h: Signal::new(0.0),
            ctx_menu_x: Signal::new(0.0),
            ctx_menu_y: Signal::new(0.0),
            ctx_menu_open: Signal::new(false),
            picker: ColorPickerState::new(),
            can_undo: Signal::new(false),
            can_redo: Signal::new(false),
            layers: Signal::new(layers_data),
            selected_layer: Signal::new(selected),
            layer_query: Signal::new(String::new()),
            sec_page_open: Signal::new(true),
            sec_comp_open: Signal::new(true),

            cmd_k_open: Signal::new(false),
            help_open: Signal::new(false),
            cmdk_query: Signal::new(String::new()),

            editing_text: Signal::new(false),
            text_edit_anchor: Signal::new((0.0, 0.0, 0.0, 0.0)),
            text_edit_orig: Rc::new(RefCell::new(String::new())),
            editing_layer: Signal::new(None),
            rename_buffer: Signal::new(String::new()),
            last_canvas_click: Rc::new(RefCell::new((None, Instant::now()))),

            toasts: Signal::new(Vec::new()),
            toast_seq: Rc::new(RefCell::new(0)),

            history: Rc::new(RefCell::new(Vec::new())),
            future: Rc::new(RefCell::new(Vec::new())),
            clipboard: Rc::new(RefCell::new(Vec::new())),
            bindings: Rc::new(RefCell::new(EffectScope::new())),

            active_tab: Signal::new(Tab::Design),
            sections: PanelSections::new(),

            sel_name: Signal::new(String::new()),
            sel_kind: Signal::new(String::new()),
            sel_x: Signal::new(0.0),
            sel_y: Signal::new(0.0),
            sel_w: Signal::new(0.0),
            sel_h: Signal::new(0.0),
            sel_opacity: Signal::new(100.0),
            sel_radius: Signal::new(0.0),
            sel_fill: Signal::new(Color::hex(0x0D99FF)),
            sel_fill_hex: Signal::new(String::from("#0D99FF")),
            sel_stroke: Signal::new(Color::TRANSPARENT),
            sel_stroke_hex: Signal::new(String::from("#00000000")),
            sel_stroke_w: Signal::new(0.0),
            sel_text: Signal::new(String::new()),
            sel_font_size: Signal::new(14.0),
            sel_layout: Signal::new("NONE".to_string()),
            sel_gap: Signal::new(0.0),
            sel_pad_h: Signal::new(0.0),
            sel_pad_v: Signal::new(0.0),
            sel_align_h: Signal::new("start".to_string()),
            sel_align_v: Signal::new("start".to_string()),
        };

        // effect 1: selection -> props
        {
            let selected = state.selected_layer.clone();
            let layers = state.layers.clone();
            let name = state.sel_name.clone();
            let kind = state.sel_kind.clone();
            let x = state.sel_x.clone();
            let y = state.sel_y.clone();
            let w = state.sel_w.clone();
            let h = state.sel_h.clone();
            let op = state.sel_opacity.clone();
            let r = state.sel_radius.clone();
            let f = state.sel_fill.clone();
            let fh = state.sel_fill_hex.clone();
            let s = state.sel_stroke.clone();
            let sh = state.sel_stroke_hex.clone();
            let sw = state.sel_stroke_w.clone();
            let text = state.sel_text.clone();
            let fsize = state.sel_font_size.clone();
            let layout = state.sel_layout.clone();
            let gap = state.sel_gap.clone();
            let pad_h = state.sel_pad_h.clone();
            let pad_v = state.sel_pad_v.clone();
            let align_h = state.sel_align_h.clone();
            let align_v = state.sel_align_v.clone();

            state.bindings.borrow_mut().push(effect(move || {
                if let Some(id) = selected.get() {
                    let ls = layers.get();
                    if let Some(l) = ls.iter().find(|l| l.id == id) {
                        name.set(l.name.clone());
                        kind.set(l.kind.to_string());
                        x.set(l.x);
                        y.set(l.y);
                        w.set(l.w);
                        h.set(l.h);
                        op.set(l.opacity);
                        r.set(l.radius);
                        f.set(l.fill);
                        fh.set(l.fill_hex.clone());
                        s.set(l.stroke);
                        sh.set(l.stroke_hex.clone());
                        sw.set(l.stroke_width);
                        text.set(l.text.clone());
                        fsize.set(l.font_size);
                        layout.set(l.layout.to_string());
                        gap.set(l.gap);
                        pad_h.set(l.pad_l);
                        pad_v.set(l.pad_t);
                        align_h.set(l.align_h.to_string());
                        align_v.set(l.align_v.to_string());
                        return;
                    }
                }
                name.set(String::new());
                kind.set(String::new());
                x.set(0.0);
                y.set(0.0);
                w.set(0.0);
                h.set(0.0);
                op.set(100.0);
                r.set(0.0);
                f.set(Color::hex(0x0D99FF));
                fh.set(String::from("#0D99FF"));
                s.set(Color::TRANSPARENT);
                sh.set(String::from("#00000000"));
                sw.set(0.0);
                text.set(String::new());
                fsize.set(14.0);
                layout.set("NONE".to_string());
                gap.set(0.0);
                pad_h.set(0.0);
                pad_v.set(0.0);
                align_h.set("start".to_string());
                align_v.set("start".to_string());
            }));
        }

        // effect 2: props -> layers
        {
            let selected = state.selected_layer.clone();
            let layers = state.layers.clone();
            let x = state.sel_x.clone();
            let y = state.sel_y.clone();
            let w = state.sel_w.clone();
            let h = state.sel_h.clone();
            let op = state.sel_opacity.clone();
            let r = state.sel_radius.clone();
            let f = state.sel_fill.clone();
            let fh = state.sel_fill_hex.clone();
            let s = state.sel_stroke.clone();
            let sh = state.sel_stroke_hex.clone();
            let sw = state.sel_stroke_w.clone();
            let name = state.sel_name.clone();
            let text = state.sel_text.clone();
            let fsize = state.sel_font_size.clone();
            let layout = state.sel_layout.clone();
            let gap = state.sel_gap.clone();
            let pad_h = state.sel_pad_h.clone();
            let pad_v = state.sel_pad_v.clone();
            let align_h = state.sel_align_h.clone();
            let align_v = state.sel_align_v.clone();
            let state_c = state.clone();
            let last_push: Rc<RefCell<Instant>> =
                Rc::new(RefCell::new(Instant::now() - Duration::from_secs(60)));

            state.bindings.borrow_mut().push(effect(move || {
                let xv = x.get();
                let yv = y.get();
                let wv = w.get();
                let hv = h.get();
                let opv = op.get();
                let rv = r.get();
                let fv = f.get();
                let fhv = fh.get();
                let sv = s.get();
                let shv = sh.get();
                let swv = sw.get();
                let nmv = name.get();
                let textv = text.get();
                let fsv = fsize.get();
                let layoutv = layout.get();
                let gapv = gap.get();
                let padv = pad_h.get();
                let padvv = pad_v.get();
                let alignhv = align_h.get();
                let alignvv = align_v.get();

                let layout_static: &'static str = match layoutv.as_str() {
                    "H" => "H",
                    "V" => "V",
                    _ => "NONE",
                };
                let align_h_static: &'static str = match alignhv.as_str() {
                    "center" => "center",
                    "end" => "end",
                    _ => "start",
                };
                let align_v_static: &'static str = match alignvv.as_str() {
                    "center" => "center",
                    "end" => "end",
                    _ => "start",
                };

                let Some(id) = selected.get_untracked() else {
                    return;
                };
                let layers_now = layers.get_untracked();
                let Some(cur_l) = layers_now.iter().find(|l| l.id == id) else {
                    return;
                };

                let same = (cur_l.x - xv).abs() < 0.01
                    && (cur_l.y - yv).abs() < 0.01
                    && (cur_l.w - wv).abs() < 0.01
                    && (cur_l.h - hv).abs() < 0.01
                    && (cur_l.opacity - opv).abs() < 0.01
                    && (cur_l.radius - rv).abs() < 0.01
                    && cur_l.fill == fv
                    && cur_l.fill_hex == fhv
                    && cur_l.stroke == sv
                    && cur_l.stroke_hex == shv
                    && (cur_l.stroke_width - swv).abs() < 0.01
                    && cur_l.name == nmv
                    && cur_l.text == textv
                    && (cur_l.font_size - fsv).abs() < 0.01
                    && cur_l.layout == layout_static
                    && (cur_l.gap - gapv).abs() < 0.01
                    && (cur_l.pad_l - padv).abs() < 0.01
                    && (cur_l.pad_t - padvv).abs() < 0.01
                    && cur_l.align_h == align_h_static
                    && cur_l.align_v == align_v_static;
                if same {
                    return;
                }

                let now = Instant::now();
                let need_push = {
                    let mut lp = last_push.borrow_mut();
                    if now.duration_since(*lp) > Duration::from_millis(400) {
                        *lp = now;
                        true
                    } else {
                        false
                    }
                };
                if need_push {
                    state_c.push_history();
                }

                layers.update(|ls| {
                    if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                        l.x = xv;
                        l.y = yv;
                        l.w = wv;
                        l.h = hv;
                        l.opacity = opv;
                        l.radius = rv;
                        l.fill = fv;
                        l.fill_hex = fhv;
                        l.stroke = sv;
                        l.stroke_hex = shv;
                        l.stroke_width = swv;
                        l.name = nmv;
                        l.text = textv;
                        l.font_size = fsv;
                        l.layout = layout_static;
                        l.gap = gapv.max(0.0);
                        l.pad_l = padv.max(0.0);
                        l.pad_r = padv.max(0.0);
                        l.pad_t = padvv.max(0.0);
                        l.pad_b = padvv.max(0.0);
                        l.align_h = align_h_static;
                        l.align_v = align_v_static;
                    }
                });

                state_c.propagate_layout();
            }));
        }

        state
    }

    pub(crate) fn propagate_layout(&self) {
        let snapshot = self.layers.get_untracked();
        let frame_ids: Vec<u64> = snapshot
            .iter()
            .filter(|l| l.kind == "FRAME" && l.layout != "NONE")
            .map(|l| l.id)
            .collect();
        if frame_ids.is_empty() {
            return;
        }
        self.layers.update(|ls| {
            for fid in frame_ids {
                apply_auto_layout_frame(fid, ls);
            }
        });
    }

    pub(crate) fn wrap_in_auto_layout(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let Some(frame) = self
            .layers
            .get_untracked()
            .iter()
            .find(|l| l.id == id)
            .cloned()
        else {
            return;
        };
        if frame.kind != "FRAME" {
            self.toast_warn("Select a frame first");
            return;
        }

        self.push_history();
        self.layers.update(|ls| {
            if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                l.layout = "H";
                if l.gap == 0.0 {
                    l.gap = 12.0;
                }
                if l.pad_l == 0.0 {
                    l.pad_l = 16.0;
                    l.pad_r = 16.0;
                }
                if l.pad_t == 0.0 {
                    l.pad_t = 16.0;
                    l.pad_b = 16.0;
                }
            }
        });
        self.propagate_layout();
        self.toast("Wrapped in auto layout");
    }

    pub(crate) fn toast(&self, msg: impl Into<String>) {
        self.toast_kind(msg, ToastKind::Info);
    }
    pub(crate) fn toast_warn(&self, msg: impl Into<String>) {
        self.toast_kind(msg, ToastKind::Warn);
    }
    pub(crate) fn toast_error(&self, msg: impl Into<String>) {
        self.toast_kind(msg, ToastKind::Error);
    }

    fn toast_kind(&self, msg: impl Into<String>, kind: ToastKind) {
        let mut seq = self.toast_seq.borrow_mut();
        *seq += 1;
        let id = *seq;
        let msg = msg.into();
        self.toasts.update(|ts| {
            ts.push(ToastEntry { id, msg, kind });
            while ts.len() > 3 {
                ts.remove(0);
            }
        });
    }

    pub(crate) fn dismiss_toast(&self, id: u64) {
        self.toasts.update(|ts| ts.retain(|t| t.id != id));
    }

    pub(crate) fn start_text_edit(&self, snapshot: bool) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        let Some(l) = self
            .layers
            .get_untracked()
            .iter()
            .find(|l| l.id == id)
            .cloned()
        else {
            return;
        };
        if l.kind != "TEXT" {
            return;
        }
        let z = self.zoom.get_untracked();
        let px = self.pan_x.get_untracked();
        let py = self.pan_y.get_untracked();
        let sx = 44.0 + (l.x - px) * z;
        let sy = 64.0 + (l.y - py) * z;
        self.text_edit_anchor
            .set((sx, sy, (l.w * z).max(120.0), (l.h * z).max(28.0)));
        *self.text_edit_orig.borrow_mut() = l.text.clone();
        if snapshot {
            self.push_history();
        }
        self.editing_text.set(true);
    }

    pub(crate) fn end_text_edit(&self, commit: bool) {
        if !commit {
            let orig = self.text_edit_orig.borrow().clone();
            if let Some(id) = self.selected_layer.get_untracked() {
                self.layers.update(|ls| {
                    if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                        l.text = orig;
                    }
                });
            }
        }
        self.editing_text.set(false);
    }

    pub(crate) fn open_layer_rename(&self, id: u64) {
        let Some(l) = self
            .layers
            .get_untracked()
            .iter()
            .find(|l| l.id == id)
            .cloned()
        else {
            return;
        };
        self.rename_buffer.set(l.name);
        self.editing_layer.set(Some(id));
    }

    pub(crate) fn end_layer_rename(&self, commit: bool) {
        let editing = self.editing_layer.get_untracked();
        self.editing_layer.set(None);
        if !commit {
            return;
        }
        let Some(id) = editing else {
            return;
        };
        let name = self.rename_buffer.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        self.push_history();
        self.layers.update(|ls| {
            if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                l.name = name;
            }
        });
    }

    pub(crate) fn push_history(&self) {
        let current = self.layers.get_untracked();
        self.push_snapshot_unchecked(current);
    }

    pub(crate) fn push_snapshot(&self, prev: Vec<Layer>) {
        if prev == self.layers.get_untracked() {
            return;
        }
        self.push_snapshot_unchecked(prev);
    }

    fn push_snapshot_unchecked(&self, prev: Vec<Layer>) {
        self.history.borrow_mut().push(prev);
        self.future.borrow_mut().clear();
        if self.history.borrow().len() > 80 {
            self.history.borrow_mut().remove(0);
        }
        self.can_undo.set(true);
        self.can_redo.set(false);
    }

    pub(crate) fn undo(&self) {
        if let Some(prev) = self.history.borrow_mut().pop() {
            self.future.borrow_mut().push(self.layers.get_untracked());
            self.layers.set(prev);
            self.can_undo.set(!self.history.borrow().is_empty());
            self.can_redo.set(true);
            self.fix_selection_after_history();
            self.propagate_layout();
            self.toast("Undo");
        } else {
            self.toast_warn("Nothing to undo");
        }
    }

    pub(crate) fn redo(&self) {
        if let Some(next) = self.future.borrow_mut().pop() {
            self.history.borrow_mut().push(self.layers.get_untracked());
            self.layers.set(next);
            self.can_undo.set(true);
            self.can_redo.set(!self.future.borrow().is_empty());
            self.fix_selection_after_history();
            self.propagate_layout();
            self.toast("Redo");
        } else {
            self.toast_warn("Nothing to redo");
        }
    }

    fn fix_selection_after_history(&self) {
        if let Some(id) = self.selected_layer.get_untracked()
            && !self.layers.get_untracked().iter().any(|l| l.id == id)
        {
            self.selected_layer.set(None);
        }
    }

    pub(crate) fn new_document(&self) {
        self.push_history();
        self.layers.set(Vec::new());
        self.selected_layer.set(None);
        self.zoom.set(1.0);
        self.pan_x.set(0.0);
        self.pan_y.set(0.0);
        self.toast("New document");
    }

    pub(crate) fn duplicate_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let ls = self.layers.get_untracked();
        let Some(orig) = ls.iter().find(|l| l.id == id).cloned() else {
            return;
        };
        self.push_history();

        let is_frame = orig.kind == "FRAME";
        let children: Vec<Layer> = if is_frame {
            ls.iter()
                .filter(|l| l.parent == Some(id))
                .cloned()
                .collect()
        } else {
            vec![]
        };

        let mut next_id = ls.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let new_root = next_id;
        next_id += 1;

        let mut new_children = Vec::new();
        for mut c in children {
            c.id = next_id;
            c.parent = Some(new_root);
            c.x += 16.0;
            c.y += 16.0;
            new_children.push(c);
            next_id += 1;
        }

        let copy = Layer {
            id: new_root,
            name: format!("{} copy", orig.name),
            x: orig.x + 16.0,
            y: orig.y + 16.0,
            ..orig
        };
        self.layers.update(|ls| {
            ls.push(copy);
            for c in new_children {
                ls.push(c);
            }
        });
        self.selected_layer.set(Some(new_root));
        self.propagate_layout();
        self.toast("Duplicated");
    }

    pub(crate) fn copy_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let ls = self.layers.get_untracked();
        let mut copy = Vec::new();
        if let Some(root) = ls.iter().find(|l| l.id == id).cloned() {
            if root.kind == "FRAME" {
                copy = ls
                    .iter()
                    .filter(|l| l.parent == Some(id))
                    .cloned()
                    .collect();
            }
            copy.insert(0, root);
        }
        if copy.is_empty() {
            return;
        }
        *self.clipboard.borrow_mut() = copy;
        self.toast("Copied");
    }

    pub(crate) fn paste(&self) {
        let clip = self.clipboard.borrow().clone();
        if clip.is_empty() {
            self.toast_warn("Clipboard empty");
            return;
        }
        self.push_history();
        let mut ls = self.layers.get_untracked();
        let mut next = ls.iter().map(|l| l.id).max().unwrap_or(0) + 1;

        let mut remap: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
        for l in &clip {
            remap.insert(l.id, next);
            next += 1;
        }

        let mut pasted = Vec::new();
        for mut l in clip {
            l.id = remap[&l.id];
            if let Some(p) = l.parent {
                l.parent = remap.get(&p).copied();
            }
            l.x += 16.0;
            l.y += 16.0;
            pasted.push(l);
        }

        let first_id = pasted.first().map(|l| l.id);
        ls.extend(pasted);
        self.layers.set(ls);
        if let Some(fid) = first_id {
            self.selected_layer.set(Some(fid));
        }
        self.propagate_layout();
        self.toast("Pasted");
    }

    pub(crate) fn delete_selected(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        self.push_history();
        let is_frame = self
            .layers
            .get_untracked()
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.kind == "FRAME")
            .unwrap_or(false);
        self.layers.update(|ls| {
            if is_frame {
                ls.retain(|l| l.id != id && l.parent != Some(id));
            } else {
                ls.retain(|l| l.id != id);
            }
        });
        self.selected_layer.set(None);
        self.toast("Deleted");
    }

    pub(crate) fn bring_forward(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        self.layers.update(|ls| {
            if let Some(i) = ls.iter().position(|l| l.id == id)
                && i + 1 < ls.len()
            {
                let l = ls.remove(i);
                ls.insert(i + 1, l);
            }
        });
    }

    pub(crate) fn send_backward(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        self.layers.update(|ls| {
            if let Some(i) = ls.iter().position(|l| l.id == id)
                && i > 0
            {
                let l = ls.remove(i);
                ls.insert(i - 1, l);
            }
        });
    }

    pub(crate) fn bring_to_front(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        self.layers.update(|ls| {
            if let Some(i) = ls.iter().position(|l| l.id == id) {
                let l = ls.remove(i);
                ls.push(l);
            }
        });
    }

    pub(crate) fn send_to_back(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.push_history();
        self.layers.update(|ls| {
            if let Some(i) = ls.iter().position(|l| l.id == id) {
                let l = ls.remove(i);
                ls.insert(0, l);
            }
        });
    }

    pub(crate) fn toggle_visibility(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.layers.update(|ls| {
            if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                l.visible = !l.visible;
            }
        });
    }

    pub(crate) fn toggle_lock(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        self.layers.update(|ls| {
            if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                l.locked = !l.locked;
            }
        });
        if let Some(l) = self.layers.get_untracked().iter().find(|l| l.id == id) {
            self.toast(if l.locked { "Locked" } else { "Unlocked" });
        }
    }

    pub(crate) fn zoom_to_fit(&self) {
        let layers = self.layers.get_untracked();
        let visible: Vec<&Layer> = layers.iter().filter(|l| l.visible).collect();
        if visible.is_empty() {
            self.toast_warn("Nothing visible");
            return;
        }
        let mut x1 = f32::INFINITY;
        let mut y1 = f32::INFINITY;
        let mut x2 = f32::NEG_INFINITY;
        let mut y2 = f32::NEG_INFINITY;
        for l in &visible {
            x1 = x1.min(l.x);
            y1 = y1.min(l.y);
            x2 = x2.max(l.x + l.w);
            y2 = y2.max(l.y + l.h);
        }
        let w = (x2 - x1).max(1.0);
        let h = (y2 - y1).max(1.0);
        let vw = self.viewport_w.get_untracked();
        let vh = self.viewport_h.get_untracked();
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let pad = 80.0;
        let z = ((vw - pad) / w).min((vh - pad) / h).clamp(0.1, 4.0);
        let px = x1 - (vw / z - w) * 0.5;
        let py = y1 - (vh / z - h) * 0.5;
        self.zoom.set(z);
        self.pan_x.set(px);
        self.pan_y.set(py);
    }

    pub(crate) fn zoom_to_selection(&self) {
        let Some(id) = self.selected_layer.get_untracked() else {
            self.toast_warn("Nothing selected");
            return;
        };
        let Some(l) = self.layers.get_untracked().into_iter().find(|l| l.id == id) else {
            return;
        };
        let vw = self.viewport_w.get_untracked();
        let vh = self.viewport_h.get_untracked();
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let pad = 120.0;
        let z = ((vw - pad) / l.w).min((vh - pad) / l.h).clamp(0.1, 4.0);
        let cx = l.x + l.w * 0.5;
        let cy = l.y + l.h * 0.5;
        self.zoom.set(z);
        self.pan_x.set(cx - (vw / z) * 0.5);
        self.pan_y.set(cy - (vh / z) * 0.5);
    }

    pub(crate) fn reset_view(&self) {
        self.zoom.set(1.0);
        self.pan_x.set(0.0);
        self.pan_y.set(0.0);
    }

    pub(crate) fn toggle_grid(&self) {
        self.grid_on.update(|v| *v = !*v);
    }

    pub(crate) fn export_svg(&self) {
        let ls = self.layers.get_untracked();
        let mut svg =
            String::from("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 800 600\">\n");
        for l in ls.iter().filter(|l| l.visible) {
            let op = l.opacity / 100.0;
            match l.kind {
                "RECTANGLE" | "FRAME" => {
                    svg.push_str(&format!("  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" fill-opacity=\"{}\" />\n", l.x, l.y, l.w, l.h, l.radius, l.fill_hex, op));
                }
                "ELLIPSE" => {
                    svg.push_str(&format!("  <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" fill-opacity=\"{}\" />\n", l.x + l.w/2.0, l.y + l.h/2.0, l.w/2.0, l.h/2.0, l.fill_hex, op));
                }
                "TEXT" => {
                    svg.push_str(&format!("  <text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\" font-family=\"sans-serif\" fill-opacity=\"{}\">{}</text>\n", l.x, l.y + l.font_size, l.font_size, l.fill_hex, op, l.text));
                }
                _ => {}
            }
        }
        svg.push_str("</svg>\n");
        match std::fs::write("export.svg", &svg) {
            Ok(_) => self.toast("Exported to export.svg"),
            Err(_) => self.toast_error("Failed to export SVG"),
        }
    }
}

fn apply_auto_layout_frame(frame_id: u64, ls: &mut [Layer]) {
    let Some(fi) = ls.iter().position(|l| l.id == frame_id) else {
        return;
    };
    let frame = ls[fi].clone();
    if frame.kind != "FRAME" || frame.layout == "NONE" {
        return;
    }

    let pad_l = frame.pad_l.max(0.0);
    let pad_t = frame.pad_t.max(0.0);
    let pad_r = frame.pad_r.max(0.0);
    let pad_b = frame.pad_b.max(0.0);
    let gap = frame.gap.max(0.0);
    let inner_w = (frame.w - pad_l - pad_r).max(0.0);
    let inner_h = (frame.h - pad_t - pad_b).max(0.0);

    let child_ids: Vec<u64> = ls
        .iter()
        .filter(|l| l.parent == Some(frame_id) && l.visible)
        .map(|l| l.id)
        .collect();
    if child_ids.is_empty() {
        return;
    }

    if frame.layout == "H" {
        let mut cx = frame.x + pad_l;
        for cid in child_ids {
            let Some(ci) = ls.iter().position(|l| l.id == cid) else {
                continue;
            };
            let c = &mut ls[ci];
            let cy = match frame.align_v {
                "center" => frame.y + pad_t + (inner_h - c.h) * 0.5,
                "end" => frame.y + pad_t + inner_h - c.h,
                _ => frame.y + pad_t,
            };
            c.x = cx;
            c.y = cy;
            cx += c.w + gap;
        }
    } else if frame.layout == "V" {
        let mut cy = frame.y + pad_t;
        for cid in child_ids {
            let Some(ci) = ls.iter().position(|l| l.id == cid) else {
                continue;
            };
            let c = &mut ls[ci];
            let cx = match frame.align_h {
                "center" => frame.x + pad_l + (inner_w - c.w) * 0.5,
                "end" => frame.x + pad_l + inner_w - c.w,
                _ => frame.x + pad_l,
            };
            c.x = cx;
            c.y = cy;
            cy += c.h + gap;
        }
    }
}

fn demo_layers() -> Vec<Layer> {
    let base = |id, name: &str, icon, kind, level, parent, x, y, w, h, radius, hex: &str| Layer {
        id,
        name: name.into(),
        icon,
        kind,
        level,
        parent,
        visible: true,
        locked: false,
        component: false,
        instance: false,
        x,
        y,
        w,
        h,
        opacity: 100.0,
        radius,
        fill: Color::hex(u32::from_str_radix(&hex[1..], 16).unwrap()),
        fill_hex: hex.into(),
        stroke: Color::TRANSPARENT,
        stroke_hex: "#00000000".into(),
        stroke_width: 0.0,
        text: String::new(),
        font_size: 14.0,
        layout: "NONE",
        gap: 0.0,
        pad_l: 0.0,
        pad_t: 0.0,
        pad_r: 0.0,
        pad_b: 0.0,
        align_h: "start",
        align_v: "start",
    };

    vec![
        base(
            1,
            "Frame 1",
            "tool/frame",
            "FRAME",
            0,
            None,
            0.0,
            0.0,
            800.0,
            600.0,
            0.0,
            "#1B1B1B",
        ),
        base(
            2,
            "Rectangle",
            "tool/rect",
            "RECTANGLE",
            1,
            Some(1),
            100.0,
            100.0,
            220.0,
            140.0,
            8.0,
            "#0D99FF",
        ),
        base(
            3,
            "Ellipse",
            "tool/ellipse",
            "ELLIPSE",
            1,
            Some(1),
            360.0,
            100.0,
            120.0,
            120.0,
            60.0,
            "#42BE65",
        ),
        Layer {
            id: 4,
            name: "Title".into(),
            icon: "tool/text",
            kind: "TEXT",
            level: 1,
            parent: Some(1),
            visible: true,
            locked: false,
            component: false,
            instance: false,
            x: 100.0,
            y: 260.0,
            w: 380.0,
            h: 40.0,
            opacity: 100.0,
            radius: 0.0,
            fill: Color::hex(0xFFFFFF),
            fill_hex: "#FFFFFF".into(),
            stroke: Color::TRANSPARENT,
            stroke_hex: "#00000000".into(),
            stroke_width: 0.0,
            text: "Zaprojektuj w Sequoia".into(),
            font_size: 28.0,
            layout: "NONE",
            gap: 0.0,
            pad_l: 0.0,
            pad_t: 0.0,
            pad_r: 0.0,
            pad_b: 0.0,
            align_h: "start",
            align_v: "start",
        },
    ]
}

#[derive(Clone, PartialEq)]
pub(crate) struct Layer {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) icon: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) level: u8,
    pub(crate) parent: Option<u64>,
    pub(crate) visible: bool,
    pub(crate) locked: bool,
    pub(crate) component: bool,
    pub(crate) instance: bool,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) opacity: f32,
    pub(crate) radius: f32,
    pub(crate) fill: Color,
    pub(crate) fill_hex: String,
    pub(crate) stroke: Color,
    pub(crate) stroke_hex: String,
    pub(crate) stroke_width: f32,
    pub(crate) text: String,
    pub(crate) font_size: f32,
    pub(crate) layout: &'static str,
    pub(crate) gap: f32,
    pub(crate) pad_l: f32,
    pub(crate) pad_t: f32,
    pub(crate) pad_r: f32,
    pub(crate) pad_b: f32,
    pub(crate) align_h: &'static str,
    pub(crate) align_v: &'static str,
}
