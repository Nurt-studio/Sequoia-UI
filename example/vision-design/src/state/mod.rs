//! Editor state: signals, reactive effects and the view facade.
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_ui::prelude::*;

use crate::core::layer::Layer;
use crate::document::demo::demo_layers;

mod document_ops;
mod history;
mod text_edit;
mod toasts;
mod ui;
mod view_ops;

pub(crate) use ui::{
    ColorPickerState, Menu, Menus, PanelSections, PickerTarget, Tab, ToastEntry, ToastKind,
};

#[derive(Clone)]
pub(crate) struct EditorState {
    pub(crate) is_night: Signal<bool>,
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
    pub(crate) interaction: Rc<crate::canvas::Interaction>,

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

    // NEW: per-child positioning
    pub(crate) sel_position_mode: Signal<String>, // "RELATIVE" | "ABSOLUTE"
    pub(crate) sel_pin_top: Signal<bool>,
    pub(crate) sel_pin_right: Signal<bool>,
    pub(crate) sel_pin_bottom: Signal<bool>,
    pub(crate) sel_pin_left: Signal<bool>,
    pub(crate) sel_self_pad_top: Signal<f32>,
    pub(crate) sel_self_pad_right: Signal<f32>,
    pub(crate) sel_self_pad_bottom: Signal<f32>,
    pub(crate) sel_self_pad_left: Signal<f32>,
    pub(crate) sel_parent_layout: Signal<String>, // parent frame's layout ("NONE" if none)
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
            is_night: Signal::new(true), // start w night
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
            interaction: Rc::new(crate::canvas::Interaction::new()),

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

            sel_position_mode: Signal::new("RELATIVE".to_string()),
            sel_pin_top: Signal::new(false),
            sel_pin_right: Signal::new(false),
            sel_pin_bottom: Signal::new(false),
            sel_pin_left: Signal::new(false),
            sel_self_pad_top: Signal::new(0.0),
            sel_self_pad_right: Signal::new(0.0),
            sel_self_pad_bottom: Signal::new(0.0),
            sel_self_pad_left: Signal::new(0.0),
            sel_parent_layout: Signal::new("NONE".to_string()),
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
            let pos_mode = state.sel_position_mode.clone();
            let pt = state.sel_pin_top.clone();
            let pr = state.sel_pin_right.clone();
            let pb = state.sel_pin_bottom.clone();
            let pl = state.sel_pin_left.clone();
            let spt = state.sel_self_pad_top.clone();
            let spr = state.sel_self_pad_right.clone();
            let spb = state.sel_self_pad_bottom.clone();
            let spl = state.sel_self_pad_left.clone();
            let parent_layout = state.sel_parent_layout.clone();

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
                        pos_mode.set(l.position_mode.to_string());
                        pt.set(l.pin_top);
                        pr.set(l.pin_right);
                        pb.set(l.pin_bottom);
                        pl.set(l.pin_left);
                        spt.set(l.self_pad_top);
                        spr.set(l.self_pad_right);
                        spb.set(l.self_pad_bottom);
                        spl.set(l.self_pad_left);
                        let pl2 = l
                            .parent
                            .and_then(|pid| ls.iter().find(|p| p.id == pid))
                            .map(|p| p.layout.to_string())
                            .unwrap_or_else(|| "NONE".to_string());
                        parent_layout.set(pl2);
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
                pos_mode.set("RELATIVE".to_string());
                pt.set(false);
                pr.set(false);
                pb.set(false);
                pl.set(false);
                spt.set(0.0);
                spr.set(0.0);
                spb.set(0.0);
                spl.set(0.0);
                parent_layout.set("NONE".to_string());
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
            let pos_mode = state.sel_position_mode.clone();
            let pt = state.sel_pin_top.clone();
            let pr = state.sel_pin_right.clone();
            let pb = state.sel_pin_bottom.clone();
            let pl = state.sel_pin_left.clone();
            let spt = state.sel_self_pad_top.clone();
            let spr = state.sel_self_pad_right.clone();
            let spb = state.sel_self_pad_bottom.clone();
            let spl = state.sel_self_pad_left.clone();
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
                let pos_modev = pos_mode.get();
                let ptv = pt.get();
                let prv = pr.get();
                let pbv = pb.get();
                let plv = pl.get();
                let sptv = spt.get();
                let sprv = spr.get();
                let spbv = spb.get();
                let splv = spl.get();

                let layout_static: &'static str = match layoutv.as_str() {
                    "H" => "H",
                    "V" => "V",
                    "Z" => "Z",
                    "G" => "G",
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
                let pos_mode_static: &'static str = match pos_modev.as_str() {
                    "ABSOLUTE" => "ABSOLUTE",
                    _ => "RELATIVE",
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
                    && cur_l.align_v == align_v_static
                    && cur_l.position_mode == pos_mode_static
                    && cur_l.pin_top == ptv
                    && cur_l.pin_right == prv
                    && cur_l.pin_bottom == pbv
                    && cur_l.pin_left == plv
                    && (cur_l.self_pad_top - sptv).abs() < 0.01
                    && (cur_l.self_pad_right - sprv).abs() < 0.01
                    && (cur_l.self_pad_bottom - spbv).abs() < 0.01
                    && (cur_l.self_pad_left - splv).abs() < 0.01;
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
                        l.position_mode = pos_mode_static;
                        l.pin_top = ptv;
                        l.pin_right = prv;
                        l.pin_bottom = pbv;
                        l.pin_left = plv;
                        l.self_pad_top = sptv.max(0.0);
                        l.self_pad_right = sprv.max(0.0);
                        l.self_pad_bottom = spbv.max(0.0);
                        l.self_pad_left = splv.max(0.0);
                    }
                });

                state_c.propagate_layout();
            }));
        }

        state
    }

    pub(crate) fn palette(&self) -> crate::theme::colors::Palette {
        if self.is_night.get() {
            crate::theme::colors::NIGHT
        } else {
            crate::theme::colors::DAY
        }
    }
    
    pub(crate) fn open_node_graph(&self, env: &Environment) {
        let Some(windows) = env.get::<Windows>() else {
            self.toast_warn("Nie znaleziono usługi okien");
            return;
        };
        let spec = crate::node_graph::node_graph_spec(self.clone());
        windows.open(spec);
        self.toast("Otwarto Node Graph");
    }
}
