//! Editor state: signals, effects, facade.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_ui::prelude::*;

use crate::core::layer::Layer;
use crate::model::svelte_ast::{SvelteNode, next_id};
use crate::model::tree_ops;

mod behavior;
mod components;
mod document_ops;
mod history;
pub(crate) mod present;
mod projects;
mod text_edit;
mod toasts;
mod ui;
mod view_ops;

pub(crate) use behavior::BehaviorCtx;
pub(crate) use components::InstanceCtx;
pub(crate) use present::PresentState;
pub(crate) use projects::ProjectTab;
pub(crate) use ui::{
    ColorPickerState, LayerDrop, Menu, Menus, PanelSections, PickerMenu, PickerOption,
    PickerTarget, Tab, ToastEntry, ToastKind,
};

#[derive(Clone)]
pub(crate) struct EditorState {
    pub(crate) is_night: Signal<bool>,
    pub(crate) doc_name: Signal<String>,
    /// Open projects (tabs). The live `root`/selection below mirror the
    /// active tab; the others keep their own documents and history.
    pub(crate) projects: Signal<Vec<ProjectTab>>,
    pub(crate) active_project: Signal<u64>,
    next_project_id: Rc<Cell<u64>>,
    pub(crate) zoom: Signal<f32>,
    pub(crate) status: Signal<String>,
    pub(crate) menus: Menus,
    pub(crate) tool: Signal<&'static str>,
    pub(crate) editing_doc: Signal<bool>,
    /// Off by default: an invisible 24px grid must not snap shapes while the
    /// user draws. The topbar button turns it on (and the grid is now visible).
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
    /// "Hide hidden layers" filter of the Layers panel (eye in the header).
    /// `false` = filter on: rows with the eye off are not listed.
    pub(crate) show_hidden_layers: Signal<bool>,
    /// Side panel widths, dragged with the resize handles around the canvas.
    pub(crate) left_panel_w: Signal<f32>,
    pub(crate) right_panel_w: Signal<f32>,
    /// Layers-panel drag & drop: the dragged row and the current drop mark.
    pub(crate) lyr_drag_id: Signal<Option<u64>>,
    pub(crate) lyr_drop: Signal<Option<LayerDrop>>,
    pub(crate) sec_page_open: Signal<bool>,
    pub(crate) sec_comp_open: Signal<bool>,

    pub(crate) cmd_k_open: Signal<bool>,
    pub(crate) help_open: Signal<bool>,
    pub(crate) cmdk_query: Signal<String>,

    pub(crate) picker: ColorPickerState,
    /// Shared dropdown for the properties panel (vars, targets, triggers).
    pub(crate) menu_picker: PickerMenu,

    pub(crate) editing_text: Signal<bool>,
    /// Caret offset (bytes) into `sel_text` while a text layer is edited inline.
    pub(crate) text_caret: Signal<usize>,
    pub(crate) text_edit_orig: Rc<RefCell<String>>,
    pub(crate) editing_layer: Signal<Option<u64>>,
    pub(crate) rename_buffer: Signal<String>,
    pub(crate) last_canvas_click: Rc<RefCell<(Option<u64>, Instant)>>,
    /// Last press on the title-bar background, for double-click maximize.
    /// Per-window state instead of a thread-local, so extra windows and tests
    /// never share click history.
    pub(crate) last_caption_click: Rc<RefCell<Option<(Instant, Point)>>>,
    pub(crate) interaction: Rc<crate::canvas::Interaction>,

    pub(crate) toasts: Signal<Vec<ToastEntry>>,
    toast_seq: Rc<RefCell<u64>>,

    pub(crate) history: Rc<RefCell<Vec<Option<SvelteNode>>>>,
    pub(crate) future: Rc<RefCell<Vec<Option<SvelteNode>>>>,
    pub(crate) clipboard: Rc<RefCell<Vec<SvelteNode>>>,
    bindings: Rc<RefCell<EffectScope>>,

    pub(crate) active_tab: Signal<Tab>,
    pub(crate) sections: PanelSections,

    /// Phase 4: the on-canvas behavior interpreter (design doc §7).
    pub(crate) present: PresentState,

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
    /// Alignment of wrapped lines (`align-content`).
    pub(crate) sel_align_content: Signal<String>,
    pub(crate) sel_wrap: Signal<bool>,
    /// Spacing between wrapped lines (`row_gap`).
    pub(crate) sel_gap_row: Signal<f32>,
    /// Figma-style sizing: `HUG`, `FILL` or `FIXED`.
    pub(crate) sel_size_w: Signal<String>,
    pub(crate) sel_size_h: Signal<String>,

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

    // ── Phase 1: components ──
    /// The selection is a master component (◇).
    pub(crate) sel_is_component: Signal<bool>,
    /// The selection is an instance of a component (◈).
    pub(crate) sel_is_instance: Signal<bool>,
    /// The selection is a node driven by a master prop (`prop: title`).
    pub(crate) sel_prop_chip: Signal<Option<String>>,
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
        let initial_name = String::from("Vision Design");
        let initial_projects = vec![
            ProjectTab {
                id: 1,
                name: initial_name.clone(),
                root: root.clone(),
                selected,
                zoom: 1.0,
                pan_x: 0.0,
                pan_y: 0.0,
                history: Vec::new(),
                future: Vec::new(),
            },
            ProjectTab::new(
                2,
                "Landing Page",
                Some(crate::document::demo::landing_tree()),
            ),
        ];

        let state = Self {
            is_night: Signal::new(true),
            doc_name: Signal::new(initial_name),
            projects: Signal::new(initial_projects),
            active_project: Signal::new(1),
            next_project_id: Rc::new(Cell::new(3)),
            zoom: Signal::new(1.0),
            status: Signal::new(String::new()),
            menus: Menus::new(),
            tool: Signal::new("V"),
            editing_doc: Signal::new(false),
            grid_on: Signal::new(false),
            pan_x: Signal::new(0.0),
            pan_y: Signal::new(0.0),
            viewport_w: Signal::new(0.0),
            viewport_h: Signal::new(0.0),
            ctx_menu_x: Signal::new(0.0),
            ctx_menu_y: Signal::new(0.0),
            ctx_menu_open: Signal::new(false),
            picker: ColorPickerState::new(),
            menu_picker: PickerMenu::default(),
            can_undo: Signal::new(false),
            can_redo: Signal::new(false),

            root: Signal::new(root),
            selected_layer: Signal::new(selected),
            selected_layers: Signal::new(selected.map(|id| vec![id]).unwrap_or_default()),
            layers: Signal::new(Vec::new()),
            layer_query: Signal::new(String::new()),
            show_hidden_layers: Signal::new(false),
            left_panel_w: Signal::new(228.0),
            right_panel_w: Signal::new(238.0),
            lyr_drag_id: Signal::new(None),
            lyr_drop: Signal::new(None),
            sec_page_open: Signal::new(true),
            sec_comp_open: Signal::new(true),

            cmd_k_open: Signal::new(false),
            help_open: Signal::new(false),
            cmdk_query: Signal::new(String::new()),

            editing_text: Signal::new(false),
            text_caret: Signal::new(0),
            text_edit_orig: Rc::new(RefCell::new(String::new())),
            editing_layer: Signal::new(None),
            rename_buffer: Signal::new(String::new()),
            last_canvas_click: Rc::new(RefCell::new((None, Instant::now()))),
            last_caption_click: Rc::new(RefCell::new(None)),
            interaction: Rc::new(crate::canvas::Interaction::new()),

            toasts: Signal::new(Vec::new()),
            toast_seq: Rc::new(RefCell::new(0)),

            history: Rc::new(RefCell::new(Vec::new())),
            future: Rc::new(RefCell::new(Vec::new())),
            clipboard: Rc::new(RefCell::new(Vec::new())),
            bindings: Rc::new(RefCell::new(EffectScope::new())),

            active_tab: Signal::new(Tab::Design),
            sections: PanelSections::new(),

            present: PresentState::new(),

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
            sel_align_content: Signal::new("start".to_string()),
            sel_wrap: Signal::new(false),
            sel_gap_row: Signal::new(0.0),
            sel_size_w: Signal::new("FIXED".to_string()),
            sel_size_h: Signal::new("FIXED".to_string()),

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

            sel_is_component: Signal::new(false),
            sel_is_instance: Signal::new(false),
            sel_prop_chip: Signal::new(None),
        };

        // ── EFFECT 1: root + selection → sel_* ──
        {
            let root = state.root.clone();
            let selected = state.selected_layer.clone();
            let flat = Signal::new(state.flat_snapshot());

            // Memo: root → flat. In Present mode the runtime variables take
            // part, so flipping a var re-lays-out the canvas (design doc §7).
            {
                let root2 = root.clone();
                let flat2 = flat.clone();
                let layers2 = state.layers.clone();
                let present_on = state.present.on.clone();
                let present_vars = state.present.vars.clone();
                let present_anims = state.present.anims.clone();
                state.bindings.borrow_mut().push(effect(move || {
                    let _ = root2.get();
                    let _ = present_on.get();
                    let vars = present_vars.get();
                    let anims = present_anims.get();
                    let runtime = crate::model::layout_engine::PresentRuntime {
                        vars,
                        keep_visible: crate::state::present::leaving_targets(&anims),
                    };
                    let nodes = crate::model::layout_engine::compute_layout_with_runtime(
                        &root2.get_untracked(),
                        &runtime,
                    );
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
            let align_content = state.sel_align_content.clone();
            let wrap = state.sel_wrap.clone();
            let gap_row = state.sel_gap_row.clone();
            let size_w = state.sel_size_w.clone();
            let size_h = state.sel_size_h.clone();
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
            let is_component = state.sel_is_component.clone();
            let is_instance = state.sel_is_instance.clone();
            let prop_chip = state.sel_prop_chip.clone();

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
                        &align_content,
                        &wrap,
                        &gap_row,
                        &size_w,
                        &size_h,
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
                    is_component.set(false);
                    is_instance.set(false);
                    prop_chip.set(None);
                    return;
                };
                let Some(node) = rv.as_ref().and_then(|r| tree_ops::find(r, id)) else {
                    return;
                };
                match node {
                    SvelteNode::Element(e) => {
                        name.set(e.name.clone());
                        kind.set(format!("{:?}", e.tag).to_uppercase());
                        is_component.set(e.component);
                        is_instance.set(e.instance);
                        prop_chip.set(None);
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
                        gap_row.set(e.style.row_gap);
                        pad_h.set(e.style.padding.left);
                        pad_v.set(e.style.padding.top);
                        wrap.set(e.style.flex_wrap == crate::model::svelte_ast::Wrap::Wrap);
                        align_content.set(ui_align(e.style.align_content).to_string());
                        // The panel's grid is spatial: in a column the vertical
                        // alignment is the primary axis.
                        let column = e.style.display == crate::model::svelte_ast::Display::Flex
                            && e.style.flex_dir == crate::model::svelte_ast::FlexDir::Col;
                        let (sh, sv) = if column {
                            (e.style.align_items, e.style.justify_content)
                        } else {
                            (e.style.justify_content, e.style.align_items)
                        };
                        align_h.set(ui_align(sh).to_string());
                        align_v.set(ui_align(sv).to_string());
                        size_w.set(size_mode(e.style.width).to_string());
                        size_h.set(size_mode(e.style.height).to_string());
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
                        is_component.set(false);
                        is_instance.set(false);
                        prop_chip.set(prop_binding_for(rv.as_ref(), id));
                        let (fx, fy, fw, fh_) = flat_rect_of(&flat, id);
                        x.set(fx);
                        y.set(fy);
                        w.set(fw);
                        h.set(fh_);
                        op.set(t.style.opacity);
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
                        align_content.set("start".into());
                        wrap.set(false);
                        gap_row.set(0.0);
                        size_w.set(if t.width.is_some() { "FIXED" } else { "HUG" }.into());
                        size_h.set(if t.height.is_some() { "FIXED" } else { "HUG" }.into());
                        pos_mode.set(if t.absolute { "ABSOLUTE" } else { "RELATIVE" }.into());
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
            let align_content = state.sel_align_content.clone();
            let wrap = state.sel_wrap.clone();
            let gap_row = state.sel_gap_row.clone();
            let size_w = state.sel_size_w.clone();
            let size_h = state.sel_size_h.clone();
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
                let acv = align_content.get();
                let wrapv = wrap.get();
                let gaprv = gap_row.get();
                let sizewv = size_w.get();
                let sizehv = size_h.get();
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
                    &layoutv, gapv, padv, padvv, &ahv, &avv, &acv, wrapv, gaprv, &sizewv, &sizehv,
                    &pos_modev, ptv, prv, pbv, plv, sptv, sprv, spbv, splv,
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
                                &textv, fsv, &layoutv, gapv, padv, padvv, &ahv, &avv, &acv, wrapv,
                                gaprv, &sizewv, &sizehv, &pos_modev, ptv, prv, pbv, plv, sptv,
                                sprv, spbv, splv,
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

        // ── EFFECT 6: doc_name → active project tab ──
        //
        // Renaming the document in the topbar also relabels its tab. The
        // active id is read untracked so switching tabs does not copy the
        // previous name onto the newly selected tab.
        {
            let name = state.doc_name.clone();
            let projects = state.projects.clone();
            let active = state.active_project.clone();
            state.bindings.borrow_mut().push(effect(move || {
                let n = name.get();
                let id = active.get_untracked();
                projects.update(|tabs| {
                    if let Some(tab) = tabs.iter_mut().find(|t| t.id == id)
                        && tab.name != n
                    {
                        tab.name = n.clone();
                    }
                });
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

/// Name of the master prop driving the node with `id`, if the node lives
/// inside a master with an interface (drives the "🔗 prop: …" chip).
fn prop_binding_for(root: Option<&SvelteNode>, id: u64) -> Option<String> {
    let root = root?;
    // Climb from the node towards the root; the first master whose interface
    // binds this node id wins.
    let mut cursor = tree_ops::find(root, id)?;
    loop {
        if let SvelteNode::Element(e) = cursor {
            if let Some(it) = e.interface.as_ref()
                && let Some(p) = it.props.iter().find(|p| p.target == id)
            {
                return Some(p.name.clone());
            }
        }
        cursor = tree_ops::parent_of(root, cursor.id())?;
    }
}

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
    // Hidden layers stay in the list: the Layers panel shows them dimmed
    // (with the eye off) and the "hide hidden" filter removes them from the
    // view. Canvas painting and hit tests skip `!visible` themselves.
    //
    // Opacity is inherited like in CSS: a 50% frame makes its 50% child draw
    // at 25%. The flat list is pre-order, but resolve parents explicitly so
    // the result does not depend on walk order.
    let own: std::collections::HashMap<_, f32> = flat
        .iter()
        .map(|f| (f.id, f.opacity().clamp(0.0, 100.0)))
        .collect();
    let parent_of: std::collections::HashMap<_, _> =
        flat.iter().map(|f| (f.id, f.parent)).collect();
    let mut effective = std::collections::HashMap::with_capacity(flat.len());
    for f in flat {
        let mut op = own.get(&f.id).copied().unwrap_or(100.0);
        let mut cur = parent_of.get(&f.id).copied().flatten();
        let mut guard = 0;
        while let Some(pid) = cur {
            if guard > 512 {
                break;
            }
            guard += 1;
            op *= own.get(&pid).copied().unwrap_or(100.0) / 100.0;
            cur = parent_of.get(&pid).copied().flatten();
        }
        effective.insert(f.id, op);
    }
    flat.iter()
        .map(|f| {
            let mut layer = layer_from_flat(f);
            if let Some(op) = effective.get(&f.id) {
                layer.opacity = *op;
            }
            layer
        })
        .collect()
}

fn layer_from_flat(f: &crate::model::layout_engine::FlatNode) -> Layer {
    let default_style = crate::model::svelte_ast::Style::default();
    let style = f.style().unwrap_or(&default_style);
    let text_absolute = matches!(
        &f.kind,
        crate::model::layout_engine::FlatKind::Text { absolute: true, .. }
    );
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
    // Component flags flow into the layer panel (◇ master / ◈ instance).
    let (layer_component, layer_instance) = match &f.kind {
        crate::model::layout_engine::FlatKind::Element {
            component,
            instance,
            ..
        } => (*component, *instance),
        _ => (false, false),
    };
    Layer {
        id: f.id,
        name: f.name.clone(),
        icon,
        kind,
        level: 0,
        parent: f.parent,
        visible: f.visible,
        locked: f.locked,
        component: layer_component,
        instance: layer_instance,
        slot: f.slot.clone(),
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
        position_mode: if text_absolute
            || style.position == crate::model::svelte_ast::Position::Absolute
        {
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

fn dimension_for(mode: &str, value: f32) -> crate::model::svelte_ast::Dimension {
    use crate::model::svelte_ast::Dimension;
    match mode {
        "HUG" => Dimension::Auto,
        "FILL" => Dimension::Fill,
        _ => Dimension::Fixed(value.max(1.0)),
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
    align_content: &Signal<String>,
    wrap: &Signal<bool>,
    gap_row: &Signal<f32>,
    size_w: &Signal<String>,
    size_h: &Signal<String>,
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
    align_content.set("start".into());
    wrap.set(false);
    gap_row.set(0.0);
    size_w.set("FIXED".into());
    size_h.set("FIXED".into());
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
    ac: &str,
    wrap: bool,
    gap_row: f32,
    size_w: &str,
    size_h: &str,
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
            size_matches(e.style.width, size_w, w)
                && size_matches(e.style.height, size_h, h)
                && e.name == name
                && (e.style.opacity - op).abs() < 0.01
                && (e.style.radius - r).abs() < 0.01
                && e.style.fill == Some(f)
                && e.style.fill_hex == fh
                && e.style.stroke == Some(s)
                && e.style.stroke_hex == sh
                && (e.style.stroke_width - sw).abs() < 0.01
                && (e.style.gap - gap).abs() < 0.01
                && (e.style.row_gap - gap_row).abs() < 0.01
                && (e.style.flex_wrap == crate::model::svelte_ast::Wrap::Wrap) == wrap
                && ac == ui_align(e.style.align_content)
                && (e.style.padding.left - pad_h).abs() < 0.01
                && (e.style.padding.top - pad_v).abs() < 0.01
                && e.style.top == Some(y)
                && e.style.left == Some(x)
                && layout_matches(&e.style.display, &e.style.flex_dir, layout)
                && ah
                    == spatial_h(
                        e.style.display,
                        e.style.flex_dir,
                        e.style.justify_content,
                        e.style.align_items,
                    )
                && av
                    == spatial_v(
                        e.style.display,
                        e.style.flex_dir,
                        e.style.justify_content,
                        e.style.align_items,
                    )
        }
        SvelteNode::Text(t) => {
            t.content == text
                && (t.style.font_size - fsize).abs() < 0.01
                && (t.style.opacity - op).abs() < 0.01
                && t.style.color == f
                && text_size_matches(t.width, size_w, w)
                && text_size_matches(t.height, size_h, h)
                && t.name == name
        }
    }
}

fn size_matches(d: crate::model::svelte_ast::Dimension, mode: &str, laid_out: f32) -> bool {
    use crate::model::svelte_ast::Dimension;
    match (mode, d) {
        ("HUG", Dimension::Auto) => true,
        ("FILL", Dimension::Fill) => true,
        ("FIXED", Dimension::Fixed(v)) => (v - laid_out).abs() < 0.01,
        _ => false,
    }
}

fn text_size_matches(value: Option<f32>, mode: &str, laid_out: f32) -> bool {
    match mode {
        "HUG" => value.is_none(),
        _ => value.is_some_and(|v| (v - laid_out).abs() < 0.01),
    }
}

fn size_mode(d: crate::model::svelte_ast::Dimension) -> &'static str {
    use crate::model::svelte_ast::Dimension;
    match d {
        Dimension::Auto => "HUG",
        Dimension::Fill => "FILL",
        Dimension::Fixed(_) => "FIXED",
    }
}

/// UI label for an alignment value (shared by both axes).
fn ui_align(a: crate::model::svelte_ast::Align) -> &'static str {
    use crate::model::svelte_ast::Align;
    match a {
        Align::Center => "center",
        Align::End => "end",
        Align::SpaceBetween => "space-between",
        Align::Stretch => "stretch",
        _ => "start",
    }
}

fn ui_to_align(v: &str) -> crate::model::svelte_ast::Align {
    use crate::model::svelte_ast::Align;
    match v {
        "center" => Align::Center,
        "end" => Align::End,
        "space-between" => Align::SpaceBetween,
        "stretch" => Align::Stretch,
        _ => Align::Start,
    }
}

/// Horizontal alignment shown in the panel for the given style.
fn spatial_h(
    display: crate::model::svelte_ast::Display,
    dir: crate::model::svelte_ast::FlexDir,
    justify: crate::model::svelte_ast::Align,
    items: crate::model::svelte_ast::Align,
) -> &'static str {
    use crate::model::svelte_ast::{Display, FlexDir};
    if display == Display::Flex && dir == FlexDir::Col {
        ui_align(items)
    } else {
        ui_align(justify)
    }
}

/// Vertical alignment shown in the panel for the given style.
fn spatial_v(
    display: crate::model::svelte_ast::Display,
    dir: crate::model::svelte_ast::FlexDir,
    justify: crate::model::svelte_ast::Align,
    items: crate::model::svelte_ast::Align,
) -> &'static str {
    use crate::model::svelte_ast::{Display, FlexDir};
    if display == Display::Flex && dir == FlexDir::Col {
        ui_align(justify)
    } else {
        ui_align(items)
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
    ac: &str,
    wrap: bool,
    gap_row: f32,
    size_w: &str,
    size_h: &str,
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
    use crate::model::svelte_ast::{Align, Display, FlexDir, Position, Wrap};
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
        // The panel grid is spatial; the primary axis is the layout direction.
        let (primary, counter) = if layout == "V" { (av, ah) } else { (ah, av) };
        st.justify_content = match primary {
            "center" => Align::Center,
            "end" => Align::End,
            "space-between" => Align::SpaceBetween,
            _ => Align::Start,
        };
        st.align_items = match counter {
            "center" => Align::Center,
            "end" => Align::End,
            "stretch" => Align::Stretch,
            _ => Align::Start,
        };
        st.align_content = match ac {
            "center" => Align::Center,
            "end" => Align::End,
            "space-between" => Align::SpaceBetween,
            "stretch" => Align::Stretch,
            _ => Align::Start,
        };
        st.flex_wrap = if wrap { Wrap::Wrap } else { Wrap::NoWrap };
        st.row_gap = gap_row.max(0.0);
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
            // Dimensions: Figma-style hug / fill / fixed.
            e.style.width = dimension_for(size_w, w);
            e.style.height = dimension_for(size_h, h);
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
            t.style.opacity = op;
            t.width = if size_w == "HUG" { None } else { Some(w) };
            t.height = if size_h == "HUG" { None } else { Some(h) };
            if pos_mode == "ABSOLUTE" {
                t.absolute = true;
                t.x = x;
                t.y = y;
            }
        }
    }
}
