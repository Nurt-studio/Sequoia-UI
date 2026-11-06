//! Editor state: signals, effects, facade.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_ui::prelude::*;

use crate::core::layer::Layer;
use crate::model::svelte_ast::{SvelteNode, next_id};
use crate::model::tree_ops;

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

    pub(crate) root: Signal<Option<SvelteNode>>,
    pub(crate) selected_layer: Signal<Option<u64>>,
    /// Everything selected on the canvas; `selected_layer` stays the primary
    /// item used by the properties panel.
    pub(crate) selected_layers: Signal<Vec<u64>>,
    pub(crate) layers: Signal<Vec<Layer>>,
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

    pub(crate) history: Rc<RefCell<Vec<Option<SvelteNode>>>>,
    pub(crate) future: Rc<RefCell<Vec<Option<SvelteNode>>>>,
    pub(crate) clipboard: Rc<RefCell<Vec<SvelteNode>>>,
    bindings: Rc<RefCell<EffectScope>>,

    pub(crate) active_tab: Signal<Tab>,
    pub(crate) sections: PanelSections,

    // Properties panel signals (the same as before)
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

    pub(crate) sel_position_mode: Signal<String>,
    pub(crate) sel_pin_top: Signal<bool>,
    pub(crate) sel_pin_right: Signal<bool>,
    pub(crate) sel_pin_bottom: Signal<bool>,
    pub(crate) sel_pin_left: Signal<bool>,
    pub(crate) sel_self_pad_top: Signal<f32>,
    pub(crate) sel_self_pad_right: Signal<f32>,
    pub(crate) sel_self_pad_bottom: Signal<f32>,
    pub(crate) sel_self_pad_left: Signal<f32>,
    pub(crate) sel_parent_layout: Signal<String>,

    pub(crate) code_source: Signal<String>,
    pub(crate) code_editing: Signal<bool>,
    pub(crate) code_error: Signal<Option<String>>,

    /// Code editor buffer (live text from `TextArea`).
    pub(crate) code_buffer: Signal<String>,
    /// The buffer differs from the generated code (unsaved changes).
    pub(crate) code_dirty: Signal<bool>,
}

impl EditorState {
    pub(crate) fn new() -> Self {
        Self::with_seed(None, None)
    }
    pub(crate) fn demo() -> Self {
        let root = crate::document::demo::demo_tree();
        Self::with_seed(Some(root), Some(2))
    }

    pub(crate) fn with_seed(root: Option<SvelteNode>, selected: Option<u64>) -> Self {
        let state = Self {
            is_night: Signal::new(true),
            doc_name: Signal::new(String::from("Vision Design - Untitled")),
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

            root: Signal::new(root),
            selected_layer: Signal::new(selected),
            selected_layers: Signal::new(selected.map(|id| vec![id]).unwrap_or_default()),
            layers: Signal::new(Vec::new()),
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

            code_source: Signal::new(String::new()),
            code_editing: Signal::new(false),
            code_error: Signal::new(None),

            code_buffer: Signal::new(String::new()),
            code_dirty: Signal::new(false),
        };

        // ── EFFECT 1: root + selection → sel_* ──
        {
            let root = state.root.clone();
            let selected = state.selected_layer.clone();
            let flat = Signal::new(state.flat_snapshot());

            // Memo: root → flat
            {
                let root2 = root.clone();
                let flat2 = flat.clone();
                let layers2 = state.layers.clone();
                state.bindings.borrow_mut().push(effect(move || {
                    let _ = root2.get();
                    let nodes = crate::model::layout_engine::compute_layout(&root2.get_untracked());
                    layers2.set(layers_from_flat(&nodes));
                    flat2.set(nodes);
                }));
            }

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
                let sel = selected.get();
                let rv = root.get();
                let Some(id) = sel else {
                    clear_props(
                        &name,
                        &kind,
                        &x,
                        &y,
                        &w,
                        &h,
                        &op,
                        &r,
                        &f,
                        &fh,
                        &s,
                        &sh,
                        &sw,
                        &text,
                        &fsize,
                        &layout,
                        &gap,
                        &pad_h,
                        &pad_v,
                        &align_h,
                        &align_v,
                        &pos_mode,
                        &pt,
                        &pr,
                        &pb,
                        &pl,
                        &spt,
                        &spr,
                        &spb,
                        &spl,
                        &parent_layout,
                    );
                    return;
                };
                let Some(node) = rv.as_ref().and_then(|r| tree_ops::find(r, id)) else {
                    return;
                };
                match node {
                    SvelteNode::Element(e) => {
                        name.set(e.name.clone());
                        kind.set(format!("{:?}", e.tag).to_uppercase());
                        let (fx, fy, fw, fh_) = flat_rect_of(&flat, id);
                        x.set(fx);
                        y.set(fy);
                        w.set(fw);
                        h.set(fh_);
                        op.set(e.style.opacity);
                        r.set(e.style.radius);
                        if let Some(c) = e.style.fill {
                            f.set(c);
                            fh.set(e.style.fill_hex.clone());
                        }
                        if let Some(c) = e.style.stroke {
                            s.set(c);
                            sh.set(e.style.stroke_hex.clone());
                        }
                        sw.set(e.style.stroke_width);
                        text.set(String::new());
                        fsize.set(14.0);
                        layout.set(
                            match e.style.display {
                                crate::model::svelte_ast::Display::Flex => {
                                    if e.style.flex_dir == crate::model::svelte_ast::FlexDir::Row {
                                        "H"
                                    } else {
                                        "V"
                                    }
                                }
                                crate::model::svelte_ast::Display::Grid => "G",
                                _ => "NONE",
                            }
                            .into(),
                        );
                        gap.set(e.style.gap);
                        pad_h.set(e.style.padding.left);
                        pad_v.set(e.style.padding.top);
                        align_h.set(
                            match e.style.justify_content {
                                crate::model::svelte_ast::Align::Center => "center",
                                crate::model::svelte_ast::Align::End => "end",
                                _ => "start",
                            }
                            .into(),
                        );
                        align_v.set(
                            match e.style.align_items {
                                crate::model::svelte_ast::Align::Center => "center",
                                crate::model::svelte_ast::Align::End => "end",
                                _ => "start",
                            }
                            .into(),
                        );
                        pos_mode.set(
                            match e.style.position {
                                crate::model::svelte_ast::Position::Absolute => "ABSOLUTE",
                                _ => "RELATIVE",
                            }
                            .into(),
                        );
                        pt.set(e.style.top.is_some());
                        pr.set(e.style.right.is_some());
                        pb.set(e.style.bottom.is_some());
                        pl.set(e.style.left.is_some());
                        spt.set(e.style.top.unwrap_or(0.0));
                        spr.set(e.style.right.unwrap_or(0.0));
                        spb.set(e.style.bottom.unwrap_or(0.0));
                        spl.set(e.style.left.unwrap_or(0.0));
                        let pl2 = rv
                            .as_ref()
                            .and_then(|r| tree_ops::parent_of(r, id))
                            .and_then(|p| match p {
                                SvelteNode::Element(pe) => Some(match pe.style.display {
                                    crate::model::svelte_ast::Display::Flex => {
                                        if pe.style.flex_dir
                                            == crate::model::svelte_ast::FlexDir::Row
                                        {
                                            "H"
                                        } else {
                                            "V"
                                        }
                                    }
                                    crate::model::svelte_ast::Display::Grid => "G",
                                    _ => "NONE",
                                }),
                                _ => None,
                            })
                            .unwrap_or("NONE");
                        parent_layout.set(pl2.to_string());
                    }
                    SvelteNode::Text(t) => {
                        name.set(t.name.clone());
                        kind.set("TEXT".into());
                        let (fx, fy, fw, fh_) = flat_rect_of(&flat, id);
                        x.set(fx);
                        y.set(fy);
                        w.set(fw);
                        h.set(fh_);
                        op.set(100.0);
                        r.set(0.0);
                        f.set(t.style.color);
                        fh.set(t.style.color_hex.clone());
                        s.set(Color::TRANSPARENT);
                        sh.set("#00000000".into());
                        sw.set(0.0);
                        text.set(t.content.clone());
                        fsize.set(t.style.font_size);
                        layout.set("NONE".into());
                        gap.set(0.0);
                        pad_h.set(0.0);
                        pad_v.set(0.0);
                        align_h.set("start".into());
                        align_v.set("start".into());
                        pos_mode.set("RELATIVE".into());
                        pt.set(false);
                        pr.set(false);
                        pb.set(false);
                        pl.set(false);
                        spt.set(0.0);
                        spr.set(0.0);
                        spb.set(0.0);
                        spl.set(0.0);
                        parent_layout.set("NONE".into());
                    }
                }
            }));
        }

        // ── EFFECT 2: sel_* → node mutation ──
        {
            let sel = state.selected_layer.clone();
            let root = state.root.clone();
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
            let st = state.clone();
            let last_push: Rc<RefCell<Instant>> =
                Rc::new(RefCell::new(Instant::now() - Duration::from_secs(60)));

            state.bindings.borrow_mut().push(effect(move || {
                let sel_id = sel.get_untracked();
                let Some(id) = sel_id else {
                    return;
                };

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
                let ahv = align_h.get();
                let avv = align_v.get();
                let pos_modev = pos_mode.get();
                let ptv = pt.get();
                let prv = pr.get();
                let pbv = pb.get();
                let plv = pl.get();
                let sptv = spt.get();
                let sprv = spr.get();
                let spbv = spb.get();
                let splv = spl.get();

                // quick check whether anything changed
                let root_now = root.get_untracked();
                let Some(node) = root_now.as_ref().and_then(|r| tree_ops::find(r, id)) else {
                    return;
                };
                let same = node_same_as_props(
                    node, &nmv, xv, yv, wv, hv, opv, rv, fv, &fhv, sv, &shv, swv, &textv, fsv,
                    &layoutv, gapv, padv, padvv, &ahv, &avv, &pos_modev, ptv, prv, pbv, plv, sptv,
                    sprv, spbv, splv,
                );
                if same {
                    return;
                }

                let now = Instant::now();
                if now.duration_since(*last_push.borrow()) > Duration::from_millis(400) {
                    *last_push.borrow_mut() = now;
                    st.push_history();
                }

                root.update(|r| {
                    if let Some(rr) = r {
                        if let Some(node) = tree_ops::find_mut(rr, id) {
                            apply_props_to_node(
                                node, &nmv, xv, yv, wv, hv, opv, rv, fv, &fhv, sv, &shv, swv,
                                &textv, fsv, &layoutv, gapv, padv, padvv, &ahv, &avv, &pos_modev,
                                ptv, prv, pbv, plv, sptv, sprv, spbv, splv,
                            );
                        }
                    }
                });
            }));
        }

        // ── EFFECT 3: root → code_source ──
        {
            let root = state.root.clone();
            let code = state.code_source.clone();
            let buffer = state.code_buffer.clone();
            let dirty = state.code_dirty.clone();
            state.bindings.borrow_mut().push(effect(move || {
                let r = root.get();
                let new = crate::codegen::to_svelte(&r);
                if code.get_untracked() != new {
                    code.set(new.clone());
                    // With no unsaved changes in the editor, the preview follows the document.
                    if !dirty.get_untracked() {
                        buffer.set(new);
                    }
                }
            }));
        }

        // ── EFFECT 4: code_buffer → code_dirty ──
        {
            let buffer = state.code_buffer.clone();
            let source = state.code_source.clone();
            let dirty = state.code_dirty.clone();
            state.bindings.borrow_mut().push(effect(move || {
                let b = buffer.get();
                dirty.set(b != source.get_untracked());
            }));
        }

        // ── EFFECT 5: selected_layer → selected_layers ──
        //
        // Picking a single item (canvas click, layer list, history) collapses
        // any marquee selection down to that item. Marquee selection itself
        // writes `selected_layers` right after setting the primary id.
        {
            let selected = state.selected_layer.clone();
            let selected_layers = state.selected_layers.clone();
            state.bindings.borrow_mut().push(effect(move || {
                let primary = selected.get();
                let want = primary.map(|id| vec![id]).unwrap_or_default();
                if selected_layers.with_untracked(|cur| cur != &want) {
                    selected_layers.set(want);
                }
            }));
        }

        state
    }

    pub(crate) fn flat_snapshot(&self) -> Vec<crate::model::layout_engine::FlatNode> {
        crate::model::layout_engine::compute_layout(&self.root.get_untracked())
    }

    pub(crate) fn next_node_id(&self) -> u64 {
        next_id(&self.root.get_untracked())
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
            self.toast_warn("Window service not available");
            return;
        };
        let spec = crate::node_graph::node_graph_spec(self.clone());
        windows.open(spec);
        self.toast("Opened Node Graph");
    }
}

// ── helpers ──

fn flat_rect_of(
    flat: &Signal<Vec<crate::model::layout_engine::FlatNode>>,
    id: u64,
) -> (f32, f32, f32, f32) {
    flat.with_untracked(|fs| {
        if let Some(f) = fs.iter().find(|f| f.id == id) {
            (
                f.rect.left(),
                f.rect.top(),
                f.rect.size.width,
                f.rect.size.height,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        }
    })
}

fn layers_from_flat(flat: &[crate::model::layout_engine::FlatNode]) -> Vec<Layer> {
    flat.iter()
        .filter(|f| f.visible)
        .map(|f| layer_from_flat(f))
        .collect()
}

fn layer_from_flat(f: &crate::model::layout_engine::FlatNode) -> Layer {
    let default_style = crate::model::svelte_ast::Style::default();
    let style = f.style().unwrap_or(&default_style);
    let (kind, icon, hex, text, font_size) = match &f.kind {
        crate::model::layout_engine::FlatKind::Text {
            content, style: ts, ..
        } => (
            "TEXT",
            "tool/text",
            ts.color.to_hex_string(),
            content.clone(),
            ts.font_size,
        ),
        crate::model::layout_engine::FlatKind::Element { style: es, .. } => {
            let (kind, icon) = if es.radius >= 999.0 {
                ("ELLIPSE", "tool/ellipse")
            } else if es.display == crate::model::svelte_ast::Display::Flex {
                ("FRAME", "tool/frame")
            } else {
                ("RECTANGLE", "tool/rect")
            };
            (
                kind,
                icon,
                es.fill
                    .clone()
                    .unwrap_or(Color::TRANSPARENT)
                    .to_hex_string(),
                String::new(),
                14.0,
            )
        }
    };
    let rect = f.rect;
    let fill_color = if kind == "TEXT" {
        Color::parse_hex(&hex).unwrap_or(Color::WHITE)
    } else {
        style.fill.unwrap_or(Color::TRANSPARENT)
    };
    Layer {
        id: f.id,
        name: f.name.clone(),
        icon,
        kind,
        level: 0,
        parent: None,
        visible: f.visible,
        locked: f.locked,
        component: false,
        instance: false,
        x: rect.left(),
        y: rect.top(),
        w: rect.size.width,
        h: rect.size.height,
        opacity: style.opacity,
        radius: style.radius,
        fill: fill_color,
        fill_hex: if kind == "TEXT" {
            hex.clone()
        } else {
            style.fill_hex.clone()
        },
        stroke: style.stroke.unwrap_or(Color::TRANSPARENT),
        stroke_hex: style.stroke_hex.clone(),
        stroke_width: style.stroke_width,
        text,
        font_size,
        layout: if kind == "FRAME" {
            if style.display == crate::model::svelte_ast::Display::Flex
                && style.flex_dir == crate::model::svelte_ast::FlexDir::Row
            {
                "H"
            } else if style.display == crate::model::svelte_ast::Display::Flex {
                "V"
            } else {
                "NONE"
            }
        } else {
            "NONE"
        },
        gap: style.gap,
        pad_l: style.padding.left,
        pad_t: style.padding.top,
        pad_r: style.padding.right,
        pad_b: style.padding.bottom,
        align_h: "start",
        align_v: "start",
        position_mode: if style.position == crate::model::svelte_ast::Position::Absolute {
            "ABSOLUTE"
        } else {
            "RELATIVE"
        },
        pin_top: style.top.is_some(),
        pin_right: style.right.is_some(),
        pin_bottom: style.bottom.is_some(),
        pin_left: style.left.is_some(),
        self_pad_top: style.top.unwrap_or(0.0),
        self_pad_right: style.right.unwrap_or(0.0),
        self_pad_bottom: style.bottom.unwrap_or(0.0),
        self_pad_left: style.left.unwrap_or(0.0),
    }
}

#[allow(clippy::too_many_arguments)]
fn clear_props(
    name: &Signal<String>,
    kind: &Signal<String>,
    x: &Signal<f32>,
    y: &Signal<f32>,
    w: &Signal<f32>,
    h: &Signal<f32>,
    op: &Signal<f32>,
    r: &Signal<f32>,
    f: &Signal<Color>,
    fh: &Signal<String>,
    s: &Signal<Color>,
    sh: &Signal<String>,
    sw: &Signal<f32>,
    text: &Signal<String>,
    fsize: &Signal<f32>,
    layout: &Signal<String>,
    gap: &Signal<f32>,
    pad_h: &Signal<f32>,
    pad_v: &Signal<f32>,
    align_h: &Signal<String>,
    align_v: &Signal<String>,
    pos_mode: &Signal<String>,
    pt: &Signal<bool>,
    pr: &Signal<bool>,
    pb: &Signal<bool>,
    pl: &Signal<bool>,
    spt: &Signal<f32>,
    spr: &Signal<f32>,
    spb: &Signal<f32>,
    spl: &Signal<f32>,
    parent_layout: &Signal<String>,
) {
    name.set(String::new());
    kind.set(String::new());
    x.set(0.0);
    y.set(0.0);
    w.set(0.0);
    h.set(0.0);
    op.set(100.0);
    r.set(0.0);
    f.set(Color::hex(0x0D99FF));
    fh.set("#0D99FF".into());
    s.set(Color::TRANSPARENT);
    sh.set("#00000000".into());
    sw.set(0.0);
    text.set(String::new());
    fsize.set(14.0);
    layout.set("NONE".into());
    gap.set(0.0);
    pad_h.set(0.0);
    pad_v.set(0.0);
    align_h.set("start".into());
    align_v.set("start".into());
    pos_mode.set("RELATIVE".into());
    pt.set(false);
    pr.set(false);
    pb.set(false);
    pl.set(false);
    spt.set(0.0);
    spr.set(0.0);
    spb.set(0.0);
    spl.set(0.0);
    parent_layout.set("NONE".into());
}

#[allow(clippy::too_many_arguments)]
fn node_same_as_props(
    node: &SvelteNode,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    op: f32,
    r: f32,
    f: Color,
    fh: &str,
    s: Color,
    sh: &str,
    sw: f32,
    text: &str,
    fsize: f32,
    layout: &str,
    gap: f32,
    pad_h: f32,
    pad_v: f32,
    ah: &str,
    av: &str,
    _pos_mode: &str,
    _pt: bool,
    _pr: bool,
    _pb: bool,
    _pl: bool,
    _spt: f32,
    _spr: f32,
    _spb: f32,
    _spl: f32,
) -> bool {
    match node {
        SvelteNode::Element(e) => {
            (e.style.width.value().unwrap_or(0.0) - w).abs() < 0.01
                && (e.style.height.value().unwrap_or(0.0) - h).abs() < 0.01
                && e.name == name
                && (e.style.opacity - op).abs() < 0.01
                && (e.style.radius - r).abs() < 0.01
                && e.style.fill == Some(f)
                && e.style.fill_hex == fh
                && e.style.stroke == Some(s)
                && e.style.stroke_hex == sh
                && (e.style.stroke_width - sw).abs() < 0.01
                && (e.style.gap - gap).abs() < 0.01
                && (e.style.padding.left - pad_h).abs() < 0.01
                && (e.style.padding.top - pad_v).abs() < 0.01
                && e.style.top == Some(y)
                && e.style.left == Some(x)
                && layout_matches(&e.style.display, &e.style.flex_dir, layout)
                && ah
                    == match e.style.justify_content {
                        crate::model::svelte_ast::Align::Center => "center",
                        crate::model::svelte_ast::Align::End => "end",
                        _ => "start",
                    }
                && av
                    == match e.style.align_items {
                        crate::model::svelte_ast::Align::Center => "center",
                        crate::model::svelte_ast::Align::End => "end",
                        _ => "start",
                    }
        }
        SvelteNode::Text(t) => {
            t.content == text
                && (t.style.font_size - fsize).abs() < 0.01
                && t.style.color == f
                && t.width == Some(w)
                && t.height == Some(h)
                && t.name == name
        }
    }
}

fn layout_matches(
    display: &crate::model::svelte_ast::Display,
    dir: &crate::model::svelte_ast::FlexDir,
    layout: &str,
) -> bool {
    use crate::model::svelte_ast::{Display, FlexDir};
    match (display, dir, layout) {
        (Display::Flex, FlexDir::Row, "H") => true,
        (Display::Flex, FlexDir::Col, "V") => true,
        (Display::Grid, _, "G") => true,
        (Display::None, _, "NONE") => true,
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_props_to_node(
    node: &mut SvelteNode,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    op: f32,
    r: f32,
    f: Color,
    fh: &str,
    s: Color,
    sh: &str,
    sw: f32,
    text: &str,
    fsize: f32,
    layout: &str,
    gap: f32,
    pad_h: f32,
    pad_v: f32,
    ah: &str,
    av: &str,
    pos_mode: &str,
    pt: bool,
    pr: bool,
    pb: bool,
    pl: bool,
    spt: f32,
    spr: f32,
    spb: f32,
    spl: f32,
) {
    use crate::model::svelte_ast::{Align, Dimension, Display, FlexDir, Position};
    let set_layout = |st: &mut crate::model::svelte_ast::Style| {
        match layout {
            "H" => {
                st.display = Display::Flex;
                st.flex_dir = FlexDir::Row;
            }
            "V" => {
                st.display = Display::Flex;
                st.flex_dir = FlexDir::Col;
            }
            "G" => {
                st.display = Display::Grid;
                if st.grid_cols.is_none() {
                    st.grid_cols = Some(2);
                }
            }
            _ => st.display = Display::None,
        }
        st.justify_content = match ah {
            "center" => Align::Center,
            "end" => Align::End,
            _ => Align::Start,
        };
        st.align_items = match av {
            "center" => Align::Center,
            "end" => Align::End,
            _ => Align::Start,
        };
    };
    match node {
        SvelteNode::Element(e) => {
            e.name = name.into();
            e.style.opacity = op;
            e.style.radius = r;
            e.style.fill = Some(f);
            e.style.fill_hex = fh.into();
            e.style.stroke = Some(s);
            e.style.stroke_hex = sh.into();
            e.style.stroke_width = sw;
            e.style.gap = gap.max(0.0);
            e.style.padding.left = pad_h.max(0.0);
            e.style.padding.right = pad_h.max(0.0);
            e.style.padding.top = pad_v.max(0.0);
            e.style.padding.bottom = pad_v.max(0.0);
            set_layout(&mut e.style);
            // Dimensions
            e.style.width = if w > 0.0 {
                Dimension::Fixed(w)
            } else {
                Dimension::Auto
            };
            e.style.height = if h > 0.0 {
                Dimension::Fixed(h)
            } else {
                Dimension::Auto
            };
            // Position
            if pos_mode == "ABSOLUTE" {
                e.style.position = Position::Absolute;
                if pt {
                    e.style.top = Some(spt);
                } else {
                    e.style.top = None;
                }
                if pr {
                    e.style.right = Some(spr);
                } else {
                    e.style.right = None;
                }
                if pb {
                    e.style.bottom = Some(spb);
                } else {
                    e.style.bottom = None;
                }
                if pl {
                    e.style.left = Some(spl);
                } else {
                    e.style.left = None;
                }
                // x/y as top/left fallback
                if !pl && !pr {
                    e.style.left = Some(x);
                }
                if !pt && !pb {
                    e.style.top = Some(y);
                }
            } else {
                e.style.position = Position::Relative;
                e.style.top = Some(y);
                e.style.left = Some(x);
            }
        }
        SvelteNode::Text(t) => {
            t.name = name.into();
            t.content = text.into();
            t.style.font_size = fsize;
            t.style.color = f;
            t.style.color_hex = fh.into();
            t.width = Some(w);
            t.height = Some(h);
        }
    }
}
