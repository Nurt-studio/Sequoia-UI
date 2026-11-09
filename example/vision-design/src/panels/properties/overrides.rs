//! Overrides section — the knobs of an instance (◈).
//!
//! The interior of an instance belongs to its master; this section is the
//! only sanctioned way to change it. Rows come from the master's interface;
//! edits go through `EditorState::set_override`, which updates the
//! materialized copy and records the override for codegen.

use sequoia_ui::prelude::*;

use crate::model::svelte_ast::PropKind;
use crate::state::{EditorState, InstanceCtx, PickerOption};
use crate::theme::colors;

pub(super) fn body_overrides(state: &EditorState) -> AnyView {
    // Built per selection change (RightBody rebuilds on selection); rows read
    // the master contract untracked, so typing here does not rebuild them.
    let Some(ctx) = state.selected_instance_ctx() else {
        return AnyView::new(
            Text::new("Master component not found for this instance")
                .size(11.0)
                .color(colors::active().icon_inactive),
        );
    };

    let mut col = Column::new().gap(8.0);
    for def in &ctx.master.props {
        let prop = def.name.clone();
        // Effective value: explicit override, else the master's default.
        let current = ctx
            .overrides
            .iter()
            .find(|o| o.prop == def.name)
            .map(|o| o.value.clone())
            .unwrap_or(def.default.clone());
        let sig = Signal::new(current);

        let submit_state = state.clone();
        let submit_prop = prop.clone();
        let blur_state = state.clone();
        let blur_prop = prop.clone();
        let blur_sig = sig.clone();

        col = col.child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Container::new().width(78.0).child(
                        Text::new(def.name.clone())
                            .size(11.0)
                            .color(colors::active().text)
                            .truncate_fill(),
                    ),
                )
                .child_flex(
                    TextField::new(sig)
                        .font_size(12.0)
                        .text_color(Color::hex(0xE8E8E8))
                        .background(colors::active().bg_lighter)
                        .border(1.0, colors::active().stroke_soft)
                        .radius(6.0)
                        .padding(Edges::xy(8.0, 6.0))
                        .on_submit(move |text: String| {
                            submit_state.set_override(&submit_prop, text);
                        })
                        .on_edit_end(move |_| {
                            blur_state.set_override(&blur_prop, blur_sig.get_untracked());
                        }),
                    1.0,
                ),
        );
    }

    if !ctx.master.events.is_empty() {
        col = col.child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Text::new("Events")
                        .size(9.5)
                        .font("IBM Plex Mono")
                        .color(colors::active().icon_inactive),
                )
                .child(
                    Text::new(ctx.master.events.join("  ·  "))
                        .size(10.5)
                        .font("IBM Plex Mono")
                        .color(colors::active().icon_inactive),
                ),
        );
    }

    // Slots (Phase 4): the master's "holes" and the content this instance
    // provides. Filling clones a sibling layer into the slot.
    if !ctx.master.slots.is_empty() {
        col = col.child(
            Text::new("SLOTS")
                .size(9.5)
                .font("IBM Plex Mono")
                .color(colors::active().icon_inactive),
        );
        let fill_options = state.slot_fill_options();
        for s in &ctx.master.slots {
            col = col.child(instance_slot_row(state, &ctx, s, &fill_options));
        }
    }

    AnyView::new(col)
}

/// One slot of the selected instance: name · content status · Fill picker ·
/// Clear. The materialized content shows on the canvas immediately (the
/// layout engine injects slot content before flattening).
fn instance_slot_row(
    state: &EditorState,
    ctx: &InstanceCtx,
    s: &crate::model::svelte_ast::SlotDef,
    fill_options: &[PickerOption],
) -> AnyView {
    let content = ctx
        .slot_content
        .iter()
        .find(|sc| sc.slot == s.name)
        .map(|sc| sc.nodes.len())
        .unwrap_or(0);
    let status = if content == 0 {
        "empty — fallback".to_string()
    } else if content == 1 {
        "1 node".to_string()
    } else {
        format!("{content} nodes")
    };

    let mut row = Row::new()
        .gap(6.0)
        .align_items(Align::Center)
        .child(
            Container::new().width(64.0).child(
                Text::new(s.name.clone())
                    .size(11.0)
                    .font("IBM Plex Mono")
                    .color(Color::hex(0xCFA6F5))
                    .truncate_fill(),
            ),
        )
        .child(
            Text::new(status)
                .size(9.5)
                .font("IBM Plex Mono")
                .color(colors::active().icon_inactive),
        );

    if fill_options.is_empty() {
        row = row.child(
            Text::new("no siblings to fill from")
                .size(9.5)
                .color(colors::active().icon_inactive),
        );
    } else {
        let st_fill = state.clone();
        let slot_name = s.name.clone();
        row = row.child_flex(
            picker_btn(
                state,
                "Fill ▾".into(),
                fill_options.to_vec(),
                "",
                64.0,
                move |value| {
                    if let Ok(src) = value.parse::<u64>() {
                        st_fill.set_slot_content(&slot_name, src);
                    }
                },
            ),
            1.0,
        );
    }

    if content > 0 {
        let st_clear = state.clone();
        let slot_name = s.name.clone();
        row = row.child(
            Container::new()
                .width(44.0)
                .center()
                .padding(Edges::xy(0.0, 4.0))
                .radius(6.0)
                .background(colors::active().bg_lighter)
                .border(1.0, colors::active().stroke_soft)
                .hover(colors::active().bg_darker)
                .on_click(move || st_clear.clear_slot_content(&slot_name))
                .child(Text::new("Clear").size(10.0).color(colors::active().text)),
        );
    }

    AnyView::new(row)
}

/// Read-only COMPONENT section for a master (◇): the contract it publishes.
pub(super) fn body_interface(state: &EditorState) -> AnyView {
    let Some(it) = state.selected_master_interface() else {
        return AnyView::new(
            Text::new("No interface defined — a component without props")
                .size(11.0)
                .color(colors::active().icon_inactive),
        );
    };

    let mut col = Column::new().gap(8.0);

    if !it.props.is_empty() {
        col = col.child(
            Text::new("PROPS")
                .size(9.5)
                .font("IBM Plex Mono")
                .color(colors::active().icon_inactive),
        );
        for p in it.props {
            let kind = match p.kind {
                PropKind::Text => "text",
                PropKind::Fill => "fill",
                PropKind::TextColor => "color",
            };
            col = col.child(
                Row::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .child(
                        Text::new("•")
                            .size(10.0)
                            .color(colors::active().icon_inactive),
                    )
                    .child(
                        Text::new(p.name)
                            .size(11.0)
                            .color(colors::active().text)
                            .truncate_fill(),
                    )
                    .child(
                        Text::new(kind)
                            .size(9.5)
                            .font("IBM Plex Mono")
                            .color(Color::hex(0xCFA6F5)),
                    )
                    .child(
                        Text::new(p.default)
                            .size(10.5)
                            .font("IBM Plex Mono")
                            .color(colors::active().icon_inactive)
                            .truncate_fill(),
                    ),
            );
        }
    }

    if !it.events.is_empty() {
        col = col.child(
            Text::new("EVENTS")
                .size(9.5)
                .font("IBM Plex Mono")
                .color(colors::active().icon_inactive),
        );
        for ev in it.events {
            col = col.child(
                Row::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .child(
                        Container::new()
                            .width(6.0)
                            .height(6.0)
                            .radius(3.0)
                            .background(Color::hex(0xCFA6F5)),
                    )
                    .child(
                        Text::new(ev)
                            .size(11.0)
                            .font("IBM Plex Mono")
                            .color(colors::active().text),
                    ),
            );
        }
    }

    if !it.slots.is_empty() {
        col = col.child(
            Text::new("SLOTS")
                .size(9.5)
                .font("IBM Plex Mono")
                .color(colors::active().icon_inactive),
        );
        for s in it.slots {
            col = col.child(
                Row::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .child(
                        Container::new()
                            .width(6.0)
                            .height(6.0)
                            .radius(3.0)
                            .border(1.0, Color::hex(0xCFA6F5)),
                    )
                    .child(
                        Text::new(s.name)
                            .size(11.0)
                            .font("IBM Plex Mono")
                            .color(colors::active().text),
                    ),
            );
        }
    }

    col = col.child(
        Container::new()
            .fill_width()
            .center()
            .padding(Edges::xy(0.0, 6.0))
            .radius(6.0)
            .background(colors::active().bg_lighter)
            .border(1.0, colors::active().stroke_soft)
            .hover(colors::active().bg_darker)
            .on_click({
                let st = state.clone();
                move || st.active_tab.set(crate::state::Tab::Interact)
            })
            .child(
                Text::new("Interface & behavior ▸")
                    .size(11.0)
                    .semibold()
                    .color(Color::hex(0xCFA6F5)),
            ),
    );

    AnyView::new(col)
}

/// Small dropdown button (same look as the Interact tab's pickers).
fn picker_btn(
    state: &EditorState,
    label: String,
    options: Vec<PickerOption>,
    selected: &str,
    width: f32,
    on_pick: impl Fn(&str) + 'static,
) -> impl View + 'static {
    crate::components::picker_btn::PickerBtn {
        state: state.clone(),
        label,
        options,
        selected: selected.to_string(),
        on_pick: std::rc::Rc::new(on_pick),
        width,
    }
}
