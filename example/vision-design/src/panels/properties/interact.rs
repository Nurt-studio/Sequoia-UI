//! Interact tab (design doc §5.3): the component contract (props/events),
//! state variables, inline rules and model validation. Since Phase 3 the
//! same behavior is also visualized by the node graph — this panel is the
//! quick fix surface, the graph is the live view + DAG validator.

use sequoia_ui::prelude::*;

use crate::model::behavior::{
    Action, Rule, Severity, Trigger, Value, ValueExpr, Var, VarType, severity_mark,
};
use crate::model::svelte_ast::{ProtoAction, ProtoTransition, ProtoTrigger};
use crate::state::{BehaviorCtx as Ctx, EditorState, PickerOption};
use crate::theme::colors;

use super::controls::mono_lbl;
use super::section::section;

pub(super) fn body_interact(state: &EditorState) -> AnyView {
    // Tracked read: every document edit rebuilds the tab.
    let _doc = state.root.get();
    let Some(ctx) = state.selected_behavior_ctx() else {
        return AnyView::new(
            Text::new("Select an element to edit its behavior")
                .size(11.0)
                .wrap()
                .color(colors::active().icon_inactive),
        );
    };

    if ctx.is_instance {
        return AnyView::new(
            Text::new(
                "An instance is a black box — its behavior lives in the master \
                 component. Select the master (◇) to edit its rules.",
            )
            .size(11.0)
            .wrap()
            .color(colors::active().icon_inactive),
        );
    }

    let mut col = Column::new().gap(10.0);
    col = col
        .child(section(
            "Prototype",
            state.sections.prototype.clone(),
            body_prototype(state),
        ))
        .child(section(
            "Interface",
            state.sections.interface.clone(),
            body_interface(state),
        ))
        .child(section(
            "State",
            state.sections.state_vars.clone(),
            body_vars(state, &ctx),
        ))
        .child(section(
            "Rules",
            state.sections.rules.clone(),
            body_rules(state, &ctx),
        ));

    if !ctx.issues.is_empty() {
        col = col.child(body_issues(&ctx));
    }

    col = col.child(open_graph_button(state));
    AnyView::new(col)
}

// ────────────────────────────────────────────────────── prototype

fn proto_trigger_options() -> Vec<PickerOption> {
    [("click", "On Click"), ("hover", "On Hover")]
        .iter()
        .map(|(v, l)| PickerOption {
            label: l.to_string(),
            value: v.to_string(),
        })
        .collect()
}

fn proto_action_options() -> Vec<PickerOption> {
    [
        ("navigate", "Navigate to"),
        ("overlay", "Open overlay"),
        ("back", "Back"),
    ]
    .iter()
    .map(|(v, l)| PickerOption {
        label: l.to_string(),
        value: v.to_string(),
    })
    .collect()
}

fn proto_transition_options() -> Vec<PickerOption> {
    [
        ProtoTransition::Instant,
        ProtoTransition::Dissolve,
        ProtoTransition::SlideLeft,
        ProtoTransition::SlideRight,
        ProtoTransition::SlideUp,
        ProtoTransition::SlideDown,
    ]
    .iter()
    .map(|t| PickerOption {
        label: t.label().to_string(),
        value: t.as_str().to_string(),
    })
    .collect()
}

/// Prototype section of the Interact tab: one connection per element.
fn body_prototype(state: &EditorState) -> AnyView {
    let mut col = Column::new().gap(8.0);
    let Some(link) = state.selected_prototype() else {
        col = col.child(
            Text::new(
                "No prototype link. Add one to jump between screens or open \
                 an overlay while presenting.",
            )
            .size(11.0)
            .wrap()
            .color(colors::active().icon_inactive),
        );
        let st = state.clone();
        col = col.child(tiny_button("+ Add link", move || st.add_prototype_link()));
        return AnyView::new(col);
    };

    // Trigger + action.
    let st = state.clone();
    let trigger = link.trigger;
    let action = link.action;
    let actions = state.clone();
    col = col.child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child_flex(
                picker(
                    state,
                    trigger.label().to_string(),
                    proto_trigger_options(),
                    trigger.as_str(),
                    100.0,
                    move |v| {
                        if let Some(t) = ProtoTrigger::from_str(v) {
                            st.update_prototype(|l| l.trigger = t);
                        }
                    },
                ),
                1.0,
            )
            .child_flex(
                picker(
                    state,
                    action.label().to_string(),
                    proto_action_options(),
                    action.as_str(),
                    110.0,
                    move |v| {
                        if let Some(a) = ProtoAction::from_str(v) {
                            actions.update_prototype(|l| l.action = a);
                        }
                    },
                ),
                1.0,
            ),
    );

    // Target frame (not used by Back).
    if link.action != ProtoAction::Back {
        let targets = state.prototype_target_options();
        let selected = link.target.map(|t| t.to_string()).unwrap_or_default();
        let label = targets
            .iter()
            .find(|o| o.value == selected)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| "Choose screen…".into());
        let st = state.clone();
        col = col.child(picker(state, label, targets, &selected, 150.0, move |v| {
            if let Ok(id) = v.parse::<u64>() {
                st.update_prototype(|l| l.target = Some(id));
            }
        }));
    }

    // Transition + duration.
    let st = state.clone();
    let transition = link.transition;
    let duration = Signal::new(link.duration_ms.to_string());
    let st_dur = state.clone();
    col = col.child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child_flex(
                picker(
                    state,
                    transition.label().to_string(),
                    proto_transition_options(),
                    transition.as_str(),
                    112.0,
                    move |v| {
                        if let Some(t) = ProtoTransition::from_str(v) {
                            st.update_prototype(|l| l.transition = t);
                        }
                    },
                ),
                1.0,
            )
            .child(
                Container::new()
                    .width(52.0)
                    .child(text_field(duration, move |text| {
                        if let Ok(ms) = text.trim().parse::<u32>() {
                            st_dur.update_prototype(|l| l.duration_ms = ms.min(10_000));
                        }
                    })),
            )
            .child(
                Text::new("ms")
                    .size(11.0)
                    .font("IBM Plex Mono")
                    .color(colors::active().icon_inactive),
            ),
    );

    let st = state.clone();
    col = col.child(tiny_button("Remove link", move || {
        st.remove_prototype_link()
    }));
    AnyView::new(col)
}

// ──────────────────────────────────────────────────────── widgets

fn caption(text: &str) -> impl View + 'static {
    Text::new(text.to_string())
        .size(11.0)
        .font("IBM Plex Mono")
        .color(colors::active().icon_inactive)
}

fn tiny_button(label: &str, on_click: impl Fn() + 'static) -> impl View + 'static {
    Container::new()
        .height(22.0)
        .padding_xy(8.0, 0.0)
        .center()
        .radius(6.0)
        .background(colors::active().bg_lighter)
        .border(1.0, colors::active().stroke_soft)
        .hover(colors::active().bg_darker)
        .on_click(on_click)
        .child(
            Text::new(label.to_string())
                .size(11.0)
                .color(Color::hex(0xE8E8E8)),
        )
}

fn remove_button(on_click: impl Fn() + 'static) -> impl View + 'static {
    Container::new()
        .width(18.0)
        .height(18.0)
        .center()
        .radius(4.0)
        .hover(Color::hexa(0xFA4D5633))
        .on_click(on_click)
        .child(
            Text::new("✕")
                .size(14.0)
                .color(colors::active().icon_inactive),
        )
}

fn text_field(sig: Signal<String>, on_submit: impl Fn(String) + 'static) -> impl View + 'static {
    TextField::new(sig)
        .font_size(13.0)
        .text_color(Color::hex(0xE8E8E8))
        .background(colors::active().bg_darker)
        .border(1.0, colors::active().stroke_soft)
        .radius(6.0)
        .padding(Edges::xy(6.0, 4.0))
        .on_submit(move |text: String| on_submit(text))
}

/// Shared dropdown trigger (`components::picker_btn`).
fn picker(
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
        menu_width: 190.0,
    }
}

// ─────────────────────────────────────────────────────── interface

/// The master's contract, editable since Phase 3: props (name · kind ·
/// target) and events. The same target list feeds Show/Hide and the node
/// graph, so the whole tool agrees on names.
fn body_interface(state: &EditorState) -> AnyView {
    let Some(it) = state.selected_master_interface() else {
        return AnyView::new(
            Text::new("Not a master — the contract belongs to the component (◇).")
                .size(11.0)
                .wrap()
                .color(colors::active().icon_inactive),
        );
    };
    let mut col = Column::new().gap(6.0);

    // ── props ──
    col = col.child(caption("PROPS"));
    if it.props.is_empty() {
        col = col.child(
            Text::new("No props — the component accepts nothing yet.")
                .size(11.0)
                .wrap()
                .color(colors::active().icon_inactive),
        );
    }
    for p in &it.props {
        col = col.child(prop_row(state, p));
    }
    let st = state.clone();
    col = col.child(tiny_button("+ Add prop", move || st.add_interface_prop()));

    // ── events ──
    col = col.child(caption("EVENTS"));
    if it.events.is_empty() {
        col = col.child(
            Text::new("No events — Emit has nothing to call yet.")
                .size(11.0)
                .wrap()
                .color(colors::active().icon_inactive),
        );
    }
    for ev in &it.events {
        col = col.child(event_row(state, ev));
    }
    let st = state.clone();
    col = col.child(tiny_button("+ Add event", move || st.add_interface_event()));

    // ── slots (Phase 4) ──
    col = col.child(caption("SLOTS"));
    if it.slots.is_empty() {
        col = col.child(
            Text::new("No slots — the parent cannot inject content yet.")
                .size(11.0)
                .wrap()
                .color(colors::active().icon_inactive),
        );
    }
    for s in &it.slots {
        col = col.child(slot_row(state, s));
    }
    let st = state.clone();
    col = col.child(tiny_button("+ Add slot", move || st.add_interface_slot()));

    AnyView::new(col)
}

/// One slot: name · container picker · ✕. The container is the "hole" —
/// its fallback children show when an instance provides no content.
fn slot_row(state: &EditorState, s: &crate::model::svelte_ast::SlotDef) -> AnyView {
    let mut row = Row::new().gap(6.0).align_items(Align::Center).child(
        Container::new().width(72.0).child(
            Text::new(s.name.clone())
                .size(12.0)
                .font("IBM Plex Mono")
                .color(Color::hex(0xCFA6F5))
                .truncate_fill(),
        ),
    );

    // container picker (elements of the master subtree, like prop targets)
    let st_target = state.clone();
    let name = s.name.clone();
    let targets = state.master_target_options();
    let target_label = targets
        .iter()
        .find(|o| o.value == s.container.to_string())
        .map(|o| o.label.clone())
        .unwrap_or_else(|| format!("#{}", s.container));
    row = row.child_flex(
        picker(
            state,
            target_label,
            targets,
            &s.container.to_string(),
            96.0,
            move |value| {
                if let Ok(tid) = value.parse::<u64>() {
                    st_target.set_interface_slot_container(&name, tid);
                }
            },
        ),
        1.0,
    );

    let st = state.clone();
    let name = s.name.clone();
    row = row.child(remove_button(move || st.remove_interface_slot(&name)));

    AnyView::new(row)
}

/// One prop: name field + kind picker + target picker + ✕.
fn prop_row(state: &EditorState, p: &crate::model::svelte_ast::PropDef) -> AnyView {
    use crate::model::svelte_ast::PropKind;
    let st = state.clone();
    let old = p.name.clone();

    let name_sig = Signal::new(p.name.clone());
    let mut row = Row::new().gap(4.0).align_items(Align::Center).child_flex(
        text_field(name_sig, move |new: String| {
            st.rename_interface_prop(&old, &new)
        }),
        1.0,
    );

    // kind picker (text / fill / color)
    let st_kind = state.clone();
    let name = p.name.clone();
    let kind_label = p.kind.as_str().to_string();
    row = row.child(picker(
        state,
        kind_label,
        [("text", "text"), ("fill", "fill"), ("text_color", "color")]
            .iter()
            .map(|(v, l)| PickerOption {
                label: l.to_string(),
                value: v.to_string(),
            })
            .collect(),
        p.kind.as_str(),
        62.0,
        move |kind| {
            if let Some(k) = PropKind::from_str(kind) {
                st_kind.set_interface_prop_kind(&name, k);
            }
        },
    ));

    // target picker (elements of the master subtree)
    let st_target = state.clone();
    let name = p.name.clone();
    let targets = state.master_target_options();
    let target_label = targets
        .iter()
        .find(|o| o.value == p.target.to_string())
        .map(|o| o.label.clone())
        .unwrap_or_else(|| format!("#{}", p.target));
    row = row.child(picker(
        state,
        target_label,
        targets,
        &p.target.to_string(),
        82.0,
        move |value| {
            if let Ok(tid) = value.parse::<u64>() {
                st_target.set_interface_prop_target(&name, tid);
            }
        },
    ));

    let st = state.clone();
    let name = p.name.clone();
    row = row.child(remove_button(move || st.remove_interface_prop(&name)));

    AnyView::new(row)
}

/// One event: name field + ✕.
fn event_row(state: &EditorState, ev: &str) -> AnyView {
    let st = state.clone();
    let old = ev.to_string();
    let sig = Signal::new(ev.to_string());
    let st2 = state.clone();
    let name = ev.to_string();
    AnyView::new(
        Row::new()
            .gap(4.0)
            .align_items(Align::Center)
            .child(
                Container::new()
                    .width(6.0)
                    .height(6.0)
                    .radius(3.0)
                    .background(Color::hex(0xCFA6F5)),
            )
            .child_flex(
                text_field(sig, move |new: String| {
                    st.rename_interface_event(&old, &new)
                }),
                1.0,
            )
            .child(remove_button(move || st2.remove_interface_event(&name))),
    )
}

// ─────────────────────────────────────────────────────────── vars

fn body_vars(state: &EditorState, ctx: &Ctx) -> AnyView {
    let mut col = Column::new().gap(6.0);

    if ctx.scope_vars.is_empty() {
        col = col.child(
            Text::new("No variables in scope. Add one — rules and bindings can use it.")
                .size(11.0)
                .wrap()
                .color(colors::active().icon_inactive),
        );
    } else {
        if !ctx.var_owner_is_self {
            col = col.child(caption(&format!("ON: {}", ctx.var_owner_name)));
        }
        for var in &ctx.scope_vars {
            col = col.child(var_row(state, var, ctx.var_owner_is_self));
        }
    }

    col = col.child(Row::new().child(tiny_button("+ Add variable", {
        let st = state.clone();
        move || st.add_var()
    })));
    AnyView::new(col)
}

/// One variable: name field + type segments (B/N/S) + initial editor + ✕.
fn var_row(state: &EditorState, var: &Var, owner_is_self: bool) -> AnyView {
    let name_sig = Signal::new(var.name.clone());
    let st_rename = state.clone();
    let old_name = var.name.clone();

    let mut row = Row::new().gap(4.0).align_items(Align::Center).child_flex(
        TextField::new(name_sig)
            .font_size(13.0)
            .text_color(Color::hex(0xE8E8E8))
            .background(colors::active().bg_darker)
            .border(1.0, colors::active().stroke_soft)
            .radius(6.0)
            .padding(Edges::xy(6.0, 4.0))
            .on_submit(move |new: String| st_rename.rename_var(&old_name, &new)),
        1.0,
    );

    for (label, ty) in [
        ("B", VarType::Bool),
        ("N", VarType::Number),
        ("S", VarType::String),
    ] {
        let st = state.clone();
        let name = var.name.clone();
        let active = var.ty == ty;
        row = row.child(
            Container::new()
                .width(20.0)
                .height(22.0)
                .center()
                .radius(4.0)
                .background(if active {
                    Color::hex(0x333336)
                } else {
                    Color::TRANSPARENT
                })
                .border(
                    1.0,
                    if active {
                        colors::active().stroke
                    } else {
                        Color::TRANSPARENT
                    },
                )
                .on_click(move || st.set_var_type(&name, ty))
                .child(
                    Text::new(label)
                        .size(12.0)
                        .font("IBM Plex Mono")
                        .color(if active {
                            Color::WHITE
                        } else {
                            colors::active().icon_inactive
                        }),
                ),
        );
    }

    if owner_is_self {
        let st = state.clone();
        let name = var.name.clone();
        row = row.child(remove_button(move || st.remove_var(&name)));
    } else {
        row = row.child(Container::new().width(18.0));
    }

    // Initial value editor, per type.
    let initial: AnyView = match var.ty {
        VarType::Bool => {
            let mut r = Row::new()
                .gap(4.0)
                .align_items(Align::Center)
                .child(mono_lbl("≡"));
            for (label, val) in [("false", false), ("true", true)] {
                let st = state.clone();
                let name = var.name.clone();
                let active = matches!(var.initial, Value::Bool(b) if b == val);
                r = r.child(
                    Container::new()
                        .height(20.0)
                        .flex(1.0)
                        .center()
                        .radius(4.0)
                        .background(if active {
                            Color::hex(0x333336)
                        } else {
                            colors::active().bg_lighter
                        })
                        .on_click(move || st.set_var_initial_bool(&name, val))
                        .child(Text::new(label).size(12.0).font("IBM Plex Mono").color(
                            if active {
                                Color::WHITE
                            } else {
                                colors::active().icon_inactive
                            },
                        )),
                );
            }
            AnyView::new(r)
        }
        VarType::Number => {
            let st = state.clone();
            let name = var.name.clone();
            let current = match var.initial {
                Value::Number(n) => {
                    if n.fract() == 0.0 {
                        format!("{}", n as i64)
                    } else {
                        format!("{n}")
                    }
                }
                _ => "0".into(),
            };
            let sig = Signal::new(current);
            AnyView::new(
                Row::new()
                    .gap(4.0)
                    .align_items(Align::Center)
                    .child(mono_lbl("≡"))
                    .child_flex(
                        text_field(sig, move |text: String| {
                            if let Ok(v) = text.trim().parse::<f32>() {
                                st.set_var_initial_number(&name, v);
                            }
                        }),
                        1.0,
                    ),
            )
        }
        VarType::String => {
            let st = state.clone();
            let name = var.name.clone();
            let current = match &var.initial {
                Value::Str(s) => s.clone(),
                _ => String::new(),
            };
            let sig = Signal::new(current);
            AnyView::new(
                Row::new()
                    .gap(4.0)
                    .align_items(Align::Center)
                    .child(mono_lbl("≡"))
                    .child_flex(
                        text_field(sig, move |text: String| {
                            st.set_var_initial_str(&name, text);
                        }),
                        1.0,
                    ),
            )
        }
    };

    AnyView::new(
        Column::new()
            .gap(4.0)
            .child(AnyView::new(row))
            .child(initial),
    )
}

// ─────────────────────────────────────────────────────────── rules

fn trigger_options() -> Vec<PickerOption> {
    [
        ("click", "On Click"),
        ("hover_in", "On Hover In"),
        ("hover_out", "On Hover Out"),
        ("mount", "On Mount"),
        ("key", "On Key…"),
        ("prop", "On Prop Changed…"),
        ("event", "On Event…"),
    ]
    .iter()
    .map(|(v, l)| PickerOption {
        label: l.to_string(),
        value: v.to_string(),
    })
    .collect()
}

fn trigger_from_kind(kind: &str, param: Option<&str>) -> Trigger {
    let param = param.unwrap_or("");
    match kind {
        "click" => Trigger::OnClick,
        "hover_in" => Trigger::OnHoverIn,
        "hover_out" => Trigger::OnHoverOut,
        "mount" => Trigger::OnMount,
        "key" => Trigger::OnKey(if param.is_empty() {
            "Enter".into()
        } else {
            param.to_string()
        }),
        "prop" => Trigger::OnPropChanged(if param.is_empty() {
            "title".into()
        } else {
            param.to_string()
        }),
        "event" => Trigger::OnEvent(if param.is_empty() {
            "selected".into()
        } else {
            param.to_string()
        }),
        _ => Trigger::OnClick,
    }
}

fn body_rules(state: &EditorState, ctx: &Ctx) -> AnyView {
    let mut col = Column::new().gap(8.0);

    if ctx.behavior.rules.is_empty() {
        col = col.child(
            Text::new(
                "No rules on this element. A rule listens on THIS element \
                 (click, hover, key…) and runs actions.",
            )
            .size(11.0)
            .wrap()
            .color(colors::active().icon_inactive),
        );
    }

    for (ri, rule) in ctx.behavior.rules.iter().enumerate() {
        col = col.child(rule_card(state, ctx, ri, rule));
    }

    // + Add rule — the trigger kind picker.
    let st = state.clone();
    col = col.child(picker(
        state,
        "+ Add rule".into(),
        trigger_options(),
        "",
        100.0,
        move |kind| st.add_rule(trigger_from_kind(kind, None)),
    ));
    AnyView::new(col)
}

fn rule_card(state: &EditorState, ctx: &Ctx, ri: usize, rule: &Rule) -> AnyView {
    let mut card = Column::new()
        .gap(6.0)
        .padding(Edges::all(8.0))
        .radius(8.0)
        .background(colors::active().bg_lighter)
        .border(1.0, colors::active().stroke_soft);

    // Header: ⚡ + trigger picker + remove.
    let st = state.clone();
    let param = rule.trigger.param().map(|p| p.to_string());
    card = card.child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(Text::new("⚡").size(14.0).color(Color::hex(0xF1C21B)))
            .child_flex(
                picker(
                    state,
                    rule.trigger.label(),
                    trigger_options(),
                    rule.trigger.kind_name(),
                    112.0,
                    move |kind| {
                        st.set_rule_trigger(ri, trigger_from_kind(kind, param.as_deref()));
                    },
                ),
                1.0,
            )
            .child(remove_button({
                let st = state.clone();
                move || st.remove_rule(ri)
            })),
    );

    // Key parameter for On Key.
    if let Trigger::OnKey(key) = &rule.trigger {
        let st = state.clone();
        let sig = Signal::new(key.clone());
        card = card.child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(caption("KEY"))
                .child_flex(
                    text_field(sig, move |text: String| st.set_rule_key(ri, text)),
                    1.0,
                ),
        );
    }

    // Actions.
    if rule.actions.is_empty() {
        card = card.child(
            Text::new("no actions — validation warns")
                .size(11.0)
                .color(colors::active().warn),
        );
    }
    for (ai, action) in rule.actions.iter().enumerate() {
        card = card.child(action_block(state, ctx, ActionPath::Top(ri, ai), action));
    }

    // + Add action.
    let st = state.clone();
    card = card.child(picker(
        state,
        "+ action".into(),
        vec![
            PickerOption {
                label: "Toggle variable".into(),
                value: "toggle".into(),
            },
            PickerOption {
                label: "Set variable".into(),
                value: "set".into(),
            },
            PickerOption {
                label: "Show target".into(),
                value: "show".into(),
            },
            PickerOption {
                label: "Hide target".into(),
                value: "hide".into(),
            },
            PickerOption {
                label: "Emit event".into(),
                value: "emit".into(),
            },
            PickerOption {
                label: "Delay…".into(),
                value: "delay".into(),
            },
        ],
        "",
        92.0,
        move |kind| add_default_action(&st, ri, kind),
    ));

    AnyView::new(card)
}

/// Adds a sensible default action of the given kind to rule `ri`.
fn add_default_action(state: &EditorState, ri: usize, kind: &str) {
    if let Some(a) = default_action(state, kind) {
        state.add_action(ri, a);
    }
}

/// Where an action lives: top level in a rule, or nested inside a Delay.
#[derive(Clone, Copy)]
enum ActionPath {
    Top(usize, usize),
    /// rule, delay index, nested index.
    Nested(usize, usize, usize),
}

impl ActionPath {
    fn replace(&self, st: &EditorState, a: Action) {
        match *self {
            ActionPath::Top(r, i) => st.replace_action(r, i, a),
            ActionPath::Nested(r, d, i) => st.replace_nested_action(r, d, i, a),
        }
    }
    fn remove(&self, st: &EditorState) {
        match *self {
            ActionPath::Top(r, i) => st.remove_action(r, i),
            ActionPath::Nested(r, d, i) => st.remove_nested_action(r, d, i),
        }
    }
}

/// One action: picker row (var / target / event) + an inline value editor
/// for Set. Picking replaces the action in place, keeping its kind.
fn action_block(state: &EditorState, ctx: &Ctx, path: ActionPath, action: &Action) -> AnyView {
    if let Action::Delay(ms, nested) = action {
        return delay_block(state, ctx, path, *ms, nested);
    }

    let (label, options): (String, Vec<PickerOption>) = match action {
        Action::ToggleVar(_) | Action::SetVar(..) => (action.label(), state.var_picker_options()),
        Action::Show { .. } | Action::Hide { .. } => {
            (action.label(), state.target_picker_options())
        }
        Action::Emit(_) => {
            let opts = state.event_picker_options();
            if opts.is_empty() {
                (
                    action.label(),
                    vec![PickerOption {
                        label: "confirm".into(),
                        value: "confirm".into(),
                    }],
                )
            } else {
                (action.label(), opts)
            }
        }
        Action::Delay(..) => unreachable!("handled above"),
    };

    // Capture what has to survive when the picked value replaces the target.
    let kind = action.kind_name();
    let keep_expr = match action {
        Action::SetVar(_, e) => Some(e.clone()),
        _ => None,
    };
    let keep_transition = match action {
        Action::Show { transition, .. } | Action::Hide { transition, .. } => *transition,
        _ => None,
    };

    let st = state.clone();
    let row = picker(state, label, options, "", 122.0, move |value| {
        if value.is_empty() {
            return;
        }
        let new_action = match kind {
            "toggle" => Action::ToggleVar(value.to_string()),
            "set" => Action::SetVar(
                value.to_string(),
                keep_expr
                    .clone()
                    .unwrap_or(ValueExpr::Value(Value::Bool(false))),
            ),
            "show" => Action::Show {
                target: value.to_string(),
                transition: keep_transition,
            },
            "hide" => Action::Hide {
                target: value.to_string(),
                transition: keep_transition,
            },
            "emit" => Action::Emit(value.to_string()),
            _ => return,
        };
        path.replace(&st, new_action);
    });

    let row_view: AnyView = AnyView::new(
        Row::new()
            .gap(4.0)
            .align_items(Align::Center)
            .child(
                Text::new("→")
                    .size(14.0)
                    .color(colors::active().icon_inactive),
            )
            .child_flex(row, 1.0)
            .child(remove_button({
                let st = state.clone();
                move || path.remove(&st)
            })),
    );

    // Extra editor under the picker for Set values.
    if let Action::SetVar(var, expr) = action {
        let ty = ctx
            .scope_vars
            .iter()
            .find(|v| &v.name == var)
            .map(|v| v.ty)
            .unwrap_or(VarType::Bool);
        let editor: AnyView = match ty {
            VarType::Bool => {
                let st = state.clone();
                let v = var.clone();
                let mut r = Row::new().gap(4.0).padding(Edges::xy(14.0, 0.0));
                for (label, val) in [("false", false), ("true", true)] {
                    let st = st.clone();
                    let v = v.clone();
                    let active = matches!(expr, ValueExpr::Value(Value::Bool(b)) if *b == val);
                    r = r.child(
                        Container::new()
                            .height(20.0)
                            .flex(1.0)
                            .center()
                            .radius(4.0)
                            .background(if active {
                                Color::hex(0x333336)
                            } else {
                                colors::active().bg_lighter
                            })
                            .on_click(move || {
                                path.replace(
                                    &st,
                                    Action::SetVar(v.clone(), ValueExpr::Value(Value::Bool(val))),
                                );
                            })
                            .child(Text::new(label).size(12.0).font("IBM Plex Mono").color(
                                if active {
                                    Color::WHITE
                                } else {
                                    colors::active().icon_inactive
                                },
                            )),
                    );
                }
                AnyView::new(r)
            }
            VarType::Number | VarType::String => {
                let st = state.clone();
                let v = var.clone();
                let current = match expr {
                    ValueExpr::Value(Value::Str(s)) => s.clone(),
                    ValueExpr::Value(Value::Number(n)) => {
                        if n.fract() == 0.0 {
                            format!("{}", *n as i64)
                        } else {
                            format!("{n}")
                        }
                    }
                    _ => String::new(),
                };
                let sig = Signal::new(current);
                let is_number = ty == VarType::Number;
                AnyView::new(
                    Container::new()
                        .padding(Edges::xy(14.0, 0.0))
                        .child(text_field(sig, move |text: String| {
                            let value = if is_number {
                                text.trim().parse::<f32>().map(Value::Number).ok()
                            } else {
                                Some(Value::Str(text))
                            };
                            if let Some(value) = value {
                                path.replace(
                                    &st,
                                    Action::SetVar(v.clone(), ValueExpr::Value(value)),
                                );
                            } else {
                                st.toast_warn("Not a number");
                            }
                        })),
                )
            }
        };
        return AnyView::new(Column::new().gap(4.0).child(row_view).child(editor));
    }

    row_view
}

/// The Delay step: duration + the nested sequence that runs when the timer
/// fires (codegen emits `setTimeout`). The nested list has its own
/// "+ action" picker — without Delay, so nesting stays one level in the UI.
fn delay_block(
    state: &EditorState,
    ctx: &Ctx,
    path: ActionPath,
    ms: u32,
    nested: &[Action],
) -> AnyView {
    let (ri, ai) = match path {
        ActionPath::Top(r, i) => (r, i),
        ActionPath::Nested(..) => {
            // JSON-authored Delay inside a Delay: rendered read-only.
            return AnyView::new(
                Row::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .child(
                        Text::new("→")
                            .size(14.0)
                            .color(colors::active().icon_inactive),
                    )
                    .child(
                        Text::new(format!("⏱ Delay {} ms (nested Delay — edit via JSON)", ms))
                            .size(11.0)
                            .color(colors::active().icon_inactive),
                    ),
            );
        }
    };

    let mut card = Column::new()
        .gap(6.0)
        .padding(Edges::all(8.0))
        .radius(8.0)
        .background(colors::active().bg_darker)
        .border(1.0, colors::active().stroke_soft);

    // Header: ⏱ + duration field + remove.
    let st_ms = state.clone();
    let sig = Signal::new(ms.to_string());
    card = card.child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(Text::new("⏱").size(14.0).color(Color::hex(0xF1C21B)))
            .child(Text::new("Wait").size(12.0).color(colors::active().text))
            .child_flex(
                text_field(sig, move |text: String| {
                    if let Ok(v) = text.trim().parse::<u32>() {
                        st_ms.set_delay_ms(ri, ai, v);
                    } else {
                        st_ms.toast_warn("Not a number");
                    }
                }),
                1.0,
            )
            .child(
                Text::new("ms, then")
                    .size(11.0)
                    .color(colors::active().icon_inactive),
            )
            .child(remove_button({
                let st = state.clone();
                move || path.remove(&st)
            })),
    );

    // Nested steps.
    if nested.is_empty() {
        card = card.child(
            Text::new("nothing after the timer yet")
                .size(11.0)
                .color(colors::active().icon_inactive),
        );
    }

    for (i, action) in nested.iter().enumerate() {
        // A nested Delay (JSON-authored) renders as a read-only row via
        // action_block's own branch.
        card = card.child(action_block(
            state,
            ctx,
            ActionPath::Nested(ri, ai, i),
            action,
        ));
    }

    // + action after the delay (no Delay — one nesting level in the UI).
    let st = state.clone();
    card = card.child(picker(
        state,
        "+ after delay".into(),
        vec![
            PickerOption {
                label: "Toggle variable".into(),
                value: "toggle".into(),
            },
            PickerOption {
                label: "Set variable".into(),
                value: "set".into(),
            },
            PickerOption {
                label: "Show target".into(),
                value: "show".into(),
            },
            PickerOption {
                label: "Hide target".into(),
                value: "hide".into(),
            },
            PickerOption {
                label: "Emit event".into(),
                value: "emit".into(),
            },
        ],
        "",
        108.0,
        move |kind| {
            if let Some(a) = default_action(&st, kind) {
                st.add_nested_action(ri, ai, a);
            }
        },
    ));

    AnyView::new(card)
}

/// Sensible default action of the given kind, built from the current scope.
fn default_action(state: &EditorState, kind: &str) -> Option<Action> {
    let ctx = state.selected_behavior_ctx()?;
    let first_var = ctx
        .scope_vars
        .first()
        .map(|v| v.name.clone())
        .unwrap_or_else(|| "open".into());
    let first_bool = ctx
        .scope_vars
        .iter()
        .find(|v| v.ty == VarType::Bool)
        .map(|v| v.name.clone())
        .unwrap_or_else(|| first_var.clone());
    let first_target = ctx
        .targets
        .iter()
        .find(|(_, _, bound)| *bound)
        .map(|(_, name, _)| name.clone())
        .or_else(|| ctx.targets.iter().nth(1).map(|(_, n, _)| n.clone()))
        .unwrap_or_else(|| "Panel".into());
    let first_event = ctx
        .master_events
        .first()
        .cloned()
        .unwrap_or_else(|| "confirm".into());

    Some(match kind {
        "toggle" => Action::ToggleVar(first_bool),
        "set" => Action::SetVar(first_var, ValueExpr::Value(Value::Bool(false))),
        "show" => Action::Show {
            target: first_target,
            transition: None,
        },
        "hide" => Action::Hide {
            target: first_target,
            transition: None,
        },
        "emit" => Action::Emit(first_event),
        "delay" => Action::Delay(200, vec![]),
        _ => return None,
    })
}

// ─────────────────────────────────────────────────────── validation

fn body_issues(ctx: &Ctx) -> AnyView {
    let mut col = Column::new()
        .gap(4.0)
        .padding(Edges::all(8.0))
        .radius(8.0)
        .background(colors::active().bg_lighter)
        .border(1.0, colors::active().stroke_soft)
        .child(caption("VALIDATION"));
    for issue in &ctx.issues {
        let (dot, mark) = match issue.severity {
            Severity::Error => (colors::active().danger, severity_mark(Severity::Error)),
            Severity::Warn => (colors::active().warn, severity_mark(Severity::Warn)),
            Severity::Info => (
                colors::active().icon_inactive,
                severity_mark(Severity::Info),
            ),
        };
        col = col.child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Container::new()
                        .width(12.0)
                        .height(12.0)
                        .center()
                        .radius(6.0)
                        .background(dot)
                        .child(Text::new(mark).size(11.0).color(Color::hex(0x111111))),
                )
                .child_flex(
                    Text::new(issue.message.clone())
                        .size(11.0)
                        .color(colors::active().text),
                    1.0,
                ),
        );
    }
    AnyView::new(col)
}

fn open_graph_button(state: &EditorState) -> impl View + 'static {
    let st = state.clone();
    Container::new()
        .fill_width()
        .center()
        .padding(Edges::xy(0.0, 6.0))
        .radius(6.0)
        .background(colors::active().bg_lighter)
        .border(1.0, colors::active().stroke_soft)
        .hover(colors::active().bg_darker)
        .on_click_env(move |env| st.open_node_graph(env))
        .child(
            Text::new("Open in Node Graph ▸")
                .size(12.0)
                .semibold()
                .color(Color::hex(0xCFA6F5)),
        )
}
