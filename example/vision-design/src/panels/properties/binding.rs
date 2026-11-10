//! Design tab sections for Phase 2 behaviors (design doc §5.2):
//! - BINDING — `{#if var}` visibility with an optional transition,
//! - HOVER (CSS) — pure-CSS hover state (`§6.1`: styling hover stays CSS).

use sequoia_ui::prelude::*;

use crate::state::EditorState;
use crate::theme::colors;

use super::section::section;

pub(super) fn body_binding(state: &EditorState) -> AnyView {
    // Tracked read: rebuild on every document edit (dropdown options follow).
    let _doc = state.root.get();
    let Some(ctx) = state.selected_behavior_ctx() else {
        return AnyView::new(Unit);
    };
    if ctx.is_instance {
        return AnyView::new(
            Text::new("Visibility of an instance is set on the page (see Code).")
                .size(11.0)
                .wrap()
                .color(colors::active().icon_inactive),
        );
    }

    let mut col = Column::new().gap(6.0);

    if ctx.scope_vars.is_empty() {
        col = col.child(
            Text::new("No variables in scope — add one in the Interact tab.")
                .size(11.0)
                .wrap()
                .color(colors::active().icon_inactive),
        );
        return AnyView::new(col);
    }

    // „Widoczny gdy" — the binding variable.
    let current = ctx
        .binding
        .as_ref()
        .map(|b| b.var.clone())
        .unwrap_or_default();
    let mut options = vec![crate::state::PickerOption {
        label: "(always visible)".into(),
        value: String::new(),
    }];
    for v in &ctx.scope_vars {
        options.push(crate::state::PickerOption {
            label: v.name.clone(),
            value: v.name.clone(),
        });
    }
    let st = state.clone();
    col = col.child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(
                Container::new().width(64.0).child(
                    Text::new("Visible if")
                        .size(11.0)
                        .color(colors::active().text),
                ),
            )
            .child_flex(
                crate::components::picker_btn::PickerBtn {
                    state: state.clone(),
                    label: if current.is_empty() {
                        "(always)".into()
                    } else {
                        current.clone()
                    },
                    options,
                    selected: current,
                    on_pick: std::rc::Rc::new(move |value: &str| {
                        st.set_binding_var(if value.is_empty() {
                            None
                        } else {
                            Some(value.to_string())
                        });
                    }),
                    width: 118.0,
                    menu_width: 190.0,
                },
                1.0,
            ),
    );

    // Transition kind + duration.
    let (kind, ms) = match ctx.binding.as_ref().and_then(|b| b.transition) {
        Some(t) => (t.kind_name(), t.ms() as f32),
        None => ("", 200.0),
    };
    let mut trow = Row::new().gap(6.0).align_items(Align::Center).child(
        Container::new().width(64.0).child(
            Text::new("Transition")
                .size(11.0)
                .color(colors::active().text),
        ),
    );
    for (label, k) in [
        ("—", ""),
        ("slide", "slide"),
        ("fade", "fade"),
        ("scale", "scale"),
    ] {
        let st = state.clone();
        let active = kind == k;
        trow = trow.child(
            Container::new()
                .height(22.0)
                .flex(1.0)
                .center()
                .radius(4.0)
                .background(if active {
                    Color::hex(0x333336)
                } else {
                    Color::TRANSPARENT
                })
                .on_click({
                    let has_binding = ctx.binding.is_some();
                    move || {
                        if !has_binding {
                            st.toast_warn("Pick a variable first");
                            return;
                        }
                        st.set_binding_transition(k, ms);
                    }
                })
                .child(
                    Text::new(label)
                        .size(11.0)
                        .font("IBM Plex Mono")
                        .color(if active {
                            Color::WHITE
                        } else {
                            colors::active().icon_inactive
                        }),
                ),
        );
    }
    col = col.child(trow);

    if !kind.is_empty() {
        let st = state.clone();
        let ms_sig = Signal::new(format!("{ms}"));
        col = col.child(
            Row::new()
                .gap(6.0)
                .align_items(Align::Center)
                .child(
                    Container::new().width(64.0).child(
                        Text::new("Duration")
                            .size(11.0)
                            .color(colors::active().text),
                    ),
                )
                .child_flex(
                    TextField::new(ms_sig)
                        .font_size(13.0)
                        .text_color(Color::hex(0xE8E8E8))
                        .background(colors::active().bg_lighter)
                        .border(1.0, colors::active().stroke_soft)
                        .radius(6.0)
                        .padding(Edges::xy(6.0, 4.0))
                        .on_submit(move |text: String| {
                            if let Ok(v) = text.trim().trim_end_matches("ms").trim().parse::<f32>()
                            {
                                st.set_binding_transition(kind, v.clamp(0.0, 10_000.0));
                            }
                        }),
                    1.0,
                )
                .child(
                    Text::new("ms")
                        .size(11.0)
                        .font("IBM Plex Mono")
                        .color(colors::active().icon_inactive),
                ),
        );
    }

    // {#if} preview of the generated markup.
    if let Some(b) = &ctx.binding {
        let preview = match b.transition {
            Some(t) => format!("{{#if {}}} + transition:{}", b.var, t.kind_name()),
            None => format!("{{#if {}}}", b.var),
        };
        col = col.child(
            Text::new(preview)
                .size(13.0)
                .font("IBM Plex Mono")
                .color(Color::hex(0xCFA6F5)),
        );
    }

    AnyView::new(col)
}

pub(super) fn binding_section(state: &EditorState) -> AnyView {
    section(
        "Binding",
        state.sections.binding.clone(),
        body_binding(state),
    )
}

// ─────────────────────────────────────────────────────── hover (CSS)

fn hex_field(
    label: &'static str,
    current: Option<&String>,
    on_set: impl Fn(Option<String>) + 'static,
) -> impl View + 'static {
    let sig = Signal::new(current.cloned().unwrap_or_default());
    Row::new()
        .gap(6.0)
        .align_items(Align::Center)
        .child(
            Container::new()
                .width(64.0)
                .child(Text::new(label).size(11.0).color(colors::active().text)),
        )
        .child_flex(
            TextField::new(sig)
                .font_size(13.0)
                .text_color(Color::hex(0xE8E8E8))
                .background(colors::active().bg_lighter)
                .border(1.0, colors::active().stroke_soft)
                .radius(6.0)
                .padding(Edges::xy(6.0, 4.0))
                .on_submit(move |text: String| {
                    let t = text.trim().to_string();
                    if t.is_empty() {
                        on_set(None);
                    } else if t.starts_with('#') {
                        on_set(Some(t));
                    } else {
                        on_set(Some(format!("#{t}")));
                    }
                }),
            1.0,
        )
}

pub(super) fn body_hover(state: &EditorState) -> AnyView {
    let _doc = state.root.get();
    let Some(ctx) = state.selected_behavior_ctx() else {
        return AnyView::new(Unit);
    };
    let hover = ctx.hover.clone().unwrap_or_default();

    let mut col = Column::new().gap(6.0);
    col = col.child(
        Text::new("Pure CSS (`.class:hover`) — no JavaScript")
            .size(11.0)
            .font("IBM Plex Mono")
            .color(colors::active().icon_inactive),
    );

    let st = state.clone();
    col = col.child(hex_field("Background", hover.fill.as_ref(), move |v| {
        st.set_hover_fill(v)
    }));
    let st = state.clone();
    col = col.child(hex_field("Text color", hover.color.as_ref(), move |v| {
        st.set_hover_color(v)
    }));

    // Opacity: 0–1, empty = not set.
    let st = state.clone();
    let op_sig = Signal::new(hover.opacity.map(|o| format!("{o}")).unwrap_or_default());
    col = col.child(
        Row::new()
            .gap(6.0)
            .align_items(Align::Center)
            .child(
                Container::new()
                    .width(64.0)
                    .child(Text::new("Opacity").size(11.0).color(colors::active().text)),
            )
            .child_flex(
                TextField::new(op_sig)
                    .font_size(13.0)
                    .text_color(Color::hex(0xE8E8E8))
                    .background(colors::active().bg_lighter)
                    .border(1.0, colors::active().stroke_soft)
                    .radius(6.0)
                    .padding(Edges::xy(6.0, 4.0))
                    .on_submit(move |text: String| {
                        let t = text.trim().to_string();
                        if t.is_empty() {
                            st.set_hover_opacity(None);
                        } else if let Ok(v) = t.parse::<f32>() {
                            st.set_hover_opacity(Some(v.clamp(0.0, 1.0)));
                        }
                    }),
                1.0,
            )
            .child(
                Text::new("0…1")
                    .size(11.0)
                    .font("IBM Plex Mono")
                    .color(colors::active().icon_inactive),
            ),
    );

    // CSS preview like the generated sheet.
    if !hover.is_empty() {
        let mut rules: Vec<String> = Vec::new();
        if let Some(f) = &hover.fill {
            rules.push(format!("background: {f}"));
        }
        if let Some(c) = &hover.color {
            rules.push(format!("color: {c}"));
        }
        if let Some(o) = hover.opacity {
            rules.push(format!("opacity: {o}"));
        }
        col = col.child(
            Text::new(format!(":hover {{ {} }}", rules.join("; ")))
                .size(13.0)
                .font("IBM Plex Mono")
                .color(Color::hex(0xCFA6F5)),
        );
    }

    AnyView::new(col)
}

pub(super) fn hover_section(state: &EditorState) -> AnyView {
    section(
        "Hover (CSS)",
        state.sections.hover.clone(),
        body_hover(state),
    )
}
