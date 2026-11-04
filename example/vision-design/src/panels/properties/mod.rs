//! Right panel: tabs, selection header and section layout.
use sequoia_ui::prelude::*;

use crate::state::{EditorState, Tab};
use crate::theme::colors;

use self::alignment::body_alignment;
use self::appearance::body_appearance;
use self::auto_layout::body_auto_layout;
use self::code::{code_body, is_code_tab};
use self::constraints::body_constraints;
use self::export::body_export;
use self::fill::body_fill;
use self::inspect::{empty_box, inspect_body};
use self::position::body_position;
use self::section::section;
use self::size::body_size;
use self::stroke::body_stroke;
use self::text::body_text;

mod alignment;
mod appearance;
mod auto_layout;
mod code;
mod constraints;
mod controls;
mod export;
mod fill;
mod inspect;
mod position;
mod section;
mod size;
mod stroke;
mod text;

const PANEL_W: f32 = 238.0;

fn tab_button(
    label: &'static str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> impl View + 'static {
    let text_color = if active {
        Color::WHITE
    } else {
        colors::active().icon_inactive
    };
    let underline_color = if active {
        Color::hex(0xE8E8E8)
    } else {
        Color::TRANSPARENT
    };
    Container::new().height(32.0).on_click(on_click).child(
        Column::new()
            .align_items(Align::Center)
            .child(
                Container::new()
                    .fill_height()
                    .padding(Edges::xy(6.0, 0.0))
                    .center()
                    .child(Text::new(label).size(12.0).semibold().color(text_color)),
            )
            .child(
                Container::new()
                    .height(2.0)
                    .fill_width()
                    .background(underline_color),
            ),
    )
}

#[derive(Clone)]
struct TabStrip {
    state: EditorState,
}

impl View for TabStrip {
    fn body(&self) -> impl View + 'static {
        let tab = self.state.active_tab.get();
        let kind = self.state.sel_kind.get();
        let td = self.state.active_tab.clone();
        let ti = self.state.active_tab.clone();
        let tc = self.state.active_tab.clone();
        Row::new()
            .fill_width()
            .padding(Edges::xy(10.0, 0.0))
            .border_bottom(1.0, colors::active().stroke_soft)
            .align_items(Align::Center)
            .gap(10.0)
            .child(tab_button("Design", tab == Tab::Design, move || {
                td.set(Tab::Design)
            }))
            .child(tab_button("Inspect", tab == Tab::Inspect, move || {
                ti.set(Tab::Inspect)
            }))
            .child(tab_button("Code", tab == Tab::Code, move || {
                tc.set(Tab::Code)
            }))
            .child(Spacer::flex(1.0))
            .child(
                Text::new(kind)
                    .size(11.0)
                    .font("IBM Plex Mono")
                    .color(colors::active().icon_inactive),
            )
    }
}

fn tab_strip(state: &EditorState) -> impl View + 'static {
    TabStrip {
        state: state.clone(),
    }
}

fn sel_head(state: &EditorState) -> impl View + 'static {
    let dims = {
        let w = state.sel_w.clone();
        let h = state.sel_h.clone();
        memo(move || format!("{:.0} × {:.0}", w.get(), h.get()))
    };
    Row::new()
        .padding(Edges::xy(12.0, 10.0))
        .border_bottom(1.0, colors::active().stroke_soft)
        .gap(8.0)
        .align_items(Align::Center)
        .child(
            Container::new()
                .width(28.0)
                .height(28.0)
                .center()
                .radius(6.0)
                .background(colors::active().bg_lighter)
                .border(1.0, colors::active().stroke_soft)
                .child(
                    Icon::new("tool/rect")
                        .size(14.0)
                        .color(colors::active().icon_active),
                ),
        )
        .child_flex(
            TextField::new(state.sel_name.clone())
                .font_size(13.0)
                .text_color(Color::hex(0xF0F0F0))
                .background(Color::TRANSPARENT)
                .border(1.0, Color::TRANSPARENT),
            1.0,
        )
        .child(
            Text::with(move || dims.get())
                .size(11.0)
                .font("IBM Plex Mono")
                .color(colors::active().icon_inactive),
        )
}

struct RightBody {
    state: EditorState,
}

impl View for RightBody {
    fn body(&self) -> impl View + 'static {
        let has_sel = self.state.selected_layer.get().is_some();
        let tab = self.state.active_tab.get();
        if !has_sel {
            AnyView::new(empty_box())
        } else if tab == Tab::Inspect {
            AnyView::new(
                Column::new()
                    .child(sel_head(&self.state))
                    .child(inspect_body(&self.state)),
            )
        } else {
            let kind = self.state.sel_kind.get();
            let is_text = kind == "TEXT";
            let is_frame = kind == "FRAME";
            // NEW: is the parent an auto-layout frame?
            let parent_has_al = self.state.sel_parent_layout.get() != "NONE";

            let mut sections: Vec<AnyView> = vec![
                section(
                    "Alignment",
                    self.state.sections.alignment.clone(),
                    body_alignment(&self.state),
                ),
                section(
                    "Position",
                    self.state.sections.position.clone(),
                    body_position(&self.state),
                ),
                section(
                    "Size",
                    self.state.sections.size.clone(),
                    body_size(&self.state),
                ),
                section(
                    "Appearance",
                    self.state.sections.appearance.clone(),
                    body_appearance(&self.state),
                ),
            ];
            if is_text {
                sections.push(section(
                    "Text",
                    self.state.sections.fill.clone(),
                    body_text(&self.state),
                ));
            }
            if is_frame {
                sections.push(section(
                    "Auto layout",
                    self.state.sections.auto_layout.clone(),
                    body_auto_layout(&self.state),
                ));
            }
            if parent_has_al {
                sections.push(section(
                    "Constraints",
                    self.state.sections.constraints.clone(),
                    body_constraints(&self.state),
                ));
            }
            sections.push(section(
                "Fill",
                self.state.sections.fill.clone(),
                body_fill(&self.state),
            ));
            sections.push(section(
                "Stroke",
                self.state.sections.stroke.clone(),
                body_stroke(&self.state),
            ));
            sections.push(section(
                "Export",
                self.state.sections.export.clone(),
                body_export(&self.state),
            ));

            AnyView::new(
                Column::new()
                    .child(sel_head(&self.state))
                    .children(sections)
                    .child(Spacer::fixed(16.0)),
            )
        }
    }
}

pub(crate) fn panel_right(state: &EditorState) -> impl View + 'static {
    Column::new()
        .width(PANEL_W)
        .fill_height()
        .background(colors::active().surface)
        .border_left(1.0, colors::active().stroke_soft)
        .child(tab_strip(state))
        .child(code_or_scroll(state))
}

/// The "Code" tab manages scrolling on its own (without an external ScrollView).
fn code_or_scroll(state: &EditorState) -> AnyView {
    if is_code_tab(state) {
        AnyView::new(code_body(state))
    } else {
        AnyView::new(
            ScrollView::new(RightBody {
                state: state.clone(),
            })
            .vertical(),
        )
    }
}