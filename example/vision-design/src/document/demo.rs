use crate::core::layer::Layer;
use crate::model::behavior::{
    Action, Behavior, Binding, HoverStyle, Rule, Transition, Trigger, Var, VarType,
};
use crate::model::component_ops::instantiate;
use crate::model::svelte_ast::{
    Align, ComponentInterface, Dimension, Display, Element, GradientDef, GradientKind,
    GradientStopDef, Override, Position, PropDef, PropKind, PrototypeLink, ShadowEffect, Style,
    SvelteNode, Tag, TextNode, TextStyle, new_frame, new_rect, new_text,
};
use sequoia_ui::prelude::Color;

/// A grid of `n` small rectangles across the 1440x900 canvas.
///
/// Backs the windowed stress mode (`--stress <n>`) so the canvas can be panned,
/// zoomed and marquee-selected with thousands of layers present.
pub(crate) fn stress_tree(n: usize) -> SvelteNode {
    const W: f32 = 1440.0;
    const H: f32 = 900.0;
    const CELL: f32 = 12.0;
    const GAP: f32 = 2.0;
    const MARGIN: f32 = 10.0;

    let cols = ((n as f32 * (W / H)).sqrt().ceil() as usize).max(1);
    let rows = n.div_ceil(cols);
    let step = CELL + GAP;
    let mut children = Vec::with_capacity(n);
    for i in 0..n {
        let col = i % cols;
        let row = i / cols;
        let mut node = new_rect(
            i as u64 + 2,
            MARGIN + col as f32 * step,
            MARGIN + row as f32 * step,
            CELL,
            CELL,
            0x0D99FF,
        );
        if let SvelteNode::Element(e) = &mut node {
            e.style.radius = 0.0;
        }
        children.push(node);
    }

    let mut root = new_frame(
        1,
        0.0,
        0.0,
        MARGIN * 2.0 + cols as f32 * step,
        MARGIN * 2.0 + rows as f32 * step,
    );
    if let SvelteNode::Element(e) = &mut root {
        e.name = "Stress".into();
        e.children = children;
    }
    root
}

pub(crate) fn demo_layers() -> Vec<Layer> {
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
        slot: None,
        x,
        y,
        w,
        h,
        opacity: 100.0,
        radius,
        fill: Color::hex(u32::from_str_radix(&hex[1..], 16).unwrap()),
        fill_hex: hex.into(),
        gradient: None,
        effects: Vec::new(),
        blur: 0.0,
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
        position_mode: "RELATIVE",
        pin_top: false,
        pin_right: false,
        pin_bottom: false,
        pin_left: false,
        self_pad_top: 0.0,
        self_pad_right: 0.0,
        self_pad_bottom: 0.0,
        self_pad_left: 0.0,
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
            slot: None,
            x: 100.0,
            y: 260.0,
            w: 380.0,
            h: 40.0,
            opacity: 100.0,
            radius: 0.0,
            fill: Color::hex(0xFFFFFF),
            fill_hex: "#FFFFFF".into(),
            gradient: None,
            effects: Vec::new(),
            blur: 0.0,
            stroke: Color::TRANSPARENT,
            stroke_hex: "#00000000".into(),
            stroke_width: 0.0,
            text: "Design with Sequoia".into(),
            font_size: 28.0,
            layout: "NONE",
            gap: 0.0,
            pad_l: 0.0,
            pad_t: 0.0,
            pad_r: 0.0,
            pad_b: 0.0,
            align_h: "start",
            align_v: "start",
            position_mode: "RELATIVE",
            pin_top: false,
            pin_right: false,
            pin_bottom: false,
            pin_left: false,
            self_pad_top: 0.0,
            self_pad_right: 0.0,
            self_pad_bottom: 0.0,
            self_pad_left: 0.0,
        },
    ]
}
/// Master component "Overlay" — a small dialog card with a text/color
/// interface, plus one instance with overrides. Lets the Phase 1 features
/// (Overrides panel, multi-file export) be tried without any setup.
fn overlay_master() -> SvelteNode {
    let title_text = SvelteNode::Text(TextNode {
        id: 6,
        name: "Dialog title".into(),
        content: "Dialog Window".into(),
        style: TextStyle {
            font_size: 15.0,
            font_weight: 600,
            color: Color::hex(0xFFFFFF),
            color_hex: "#FFFFFF".into(),
            align: Align::Start,
            opacity: 100.0,
        },
        width: None,
        height: None,
        x: 0.0,
        y: 0.0,
        absolute: false,
        visible: true,
        locked: false,
    });

    let confirm_label = SvelteNode::Text(TextNode {
        id: 9,
        name: "Confirm label".into(),
        content: "OK".into(),
        style: TextStyle {
            font_size: 13.0,
            font_weight: 600,
            color: Color::hex(0xFFFFFF),
            color_hex: "#FFFFFF".into(),
            align: Align::Start,
            opacity: 100.0,
        },
        width: None,
        height: None,
        x: 0.0,
        y: 0.0,
        absolute: false,
        visible: true,
        locked: false,
    });
    let confirm_btn = SvelteNode::Element(Element {
        id: 8,
        name: "Confirm button".into(),
        tag: Tag::Button,
        style: Style {
            display: Display::None,
            position: Position::Relative,
            width: Dimension::Auto,
            height: Dimension::Auto,
            padding: sequoia_ui::prelude::Edges::xy(14.0, 7.0),
            fill: Some(Color::hex(0xFA4D56)),
            fill_hex: "#FA4D56".into(),
            radius: 8.0,
            ..Default::default()
        },
        children: vec![confirm_label],
        visible: true,
        locked: false,
        component: false,
        instance: false,
        interface: None,
        overrides: Vec::new(),
        slot_content: Vec::new(),
        behavior: Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnClick,
                actions: vec![Action::Emit("confirm".into())],
            }],
        }),
        binding: None,
        prototype: None,
        hover: Some(HoverStyle {
            fill: Some("#FF6B72".into()),
            color: None,
            opacity: None,
        }),
    });

    let cancel_label = SvelteNode::Text(TextNode {
        id: 11,
        name: "Cancel label".into(),
        content: "Cancel".into(),
        style: TextStyle {
            font_size: 13.0,
            font_weight: 600,
            color: Color::hex(0xE8E8E8),
            color_hex: "#E8E8E8".into(),
            align: Align::Start,
            opacity: 100.0,
        },
        width: None,
        height: None,
        x: 0.0,
        y: 0.0,
        absolute: false,
        visible: true,
        locked: false,
    });
    let cancel_btn = SvelteNode::Element(Element {
        id: 10,
        name: "Cancel button".into(),
        tag: Tag::Button,
        style: Style {
            display: Display::None,
            position: Position::Relative,
            width: Dimension::Auto,
            height: Dimension::Auto,
            padding: sequoia_ui::prelude::Edges::xy(14.0, 7.0),
            fill: Some(Color::hex(0x3A3A3E)),
            fill_hex: "#3A3A3E".into(),
            radius: 8.0,
            ..Default::default()
        },
        children: vec![cancel_label],
        visible: true,
        locked: false,
        component: false,
        instance: false,
        interface: None,
        overrides: Vec::new(),
        slot_content: Vec::new(),
        behavior: Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnClick,
                actions: vec![Action::Emit("cancel".into())],
            }],
        }),
        binding: None,
        prototype: None,
        hover: Some(HoverStyle {
            fill: Some("#4A4A4E".into()),
            color: None,
            opacity: None,
        }),
    });

    let actions_row = SvelteNode::Element(Element {
        id: 7,
        name: "Actions".into(),
        tag: Tag::Div,
        style: Style {
            display: Display::Flex,
            flex_dir: crate::model::svelte_ast::FlexDir::Row,
            gap: 8.0,
            justify_content: Align::End,
            position: Position::Relative,
            width: Dimension::Fill,
            height: Dimension::Auto,
            ..Default::default()
        },
        children: vec![cancel_btn, confirm_btn],
        visible: true,
        locked: false,
        component: false,
        instance: false,
        interface: None,
        overrides: Vec::new(),
        slot_content: Vec::new(),
        behavior: None,
        binding: None,
        hover: None,
        prototype: None,
    });

    SvelteNode::Element(Element {
        id: 5,
        name: "Overlay".into(),
        tag: Tag::Div,
        style: Style {
            display: Display::Flex,
            flex_dir: crate::model::svelte_ast::FlexDir::Col,
            gap: 12.0,
            align_items: Align::Start,
            position: Position::Absolute,
            width: Dimension::Fixed(280.0),
            height: Dimension::Auto,
            top: Some(330.0),
            left: Some(500.0),
            padding: sequoia_ui::prelude::Edges::all(16.0),
            fill: Some(Color::hex(0x26262A)),
            fill_hex: "#26262A".into(),
            radius: 12.0,
            effects: vec![ShadowEffect {
                color: Color::hexa(0x00000080),
                hex: "#00000080".into(),
                x: 0.0,
                y: 18.0,
                blur: 48.0,
                spread: -8.0,
                enabled: true,
            }],
            ..Default::default()
        },
        children: vec![title_text, actions_row],
        visible: true,
        locked: false,
        component: true,
        instance: false,
        interface: Some(ComponentInterface {
            props: vec![
                PropDef {
                    name: "title".into(),
                    default: "Dialog Window".into(),
                    target: 6,
                    kind: PropKind::Text,
                },
                PropDef {
                    name: "confirmTxt".into(),
                    default: "OK".into(),
                    target: 9,
                    kind: PropKind::Text,
                },
                PropDef {
                    name: "cancelTxt".into(),
                    default: "Cancel".into(),
                    target: 11,
                    kind: PropKind::Text,
                },
                PropDef {
                    name: "accent".into(),
                    default: "#FA4D56".into(),
                    target: 8,
                    kind: PropKind::Fill,
                },
            ],
            events: vec!["confirm".into(), "cancel".into()],
            slots: vec![],
        }),
        overrides: Vec::new(),
        slot_content: Vec::new(),
        behavior: None,
        binding: None,
        hover: None,
        prototype: None,
    })
}

/// Instance of `overlay_master` placed on the page with a few overrides
/// (copy + accent color), like dragging a component out of the library.
fn overlay_instance() -> SvelteNode {
    let master = match overlay_master() {
        SvelteNode::Element(e) => e,
        _ => unreachable!(),
    };
    let mut next = 12u64;
    let mut inst = instantiate(
        &master,
        vec![
            Override {
                prop: "title".into(),
                value: "Delete file?".into(),
            },
            Override {
                prop: "confirmTxt".into(),
                value: "Delete".into(),
            },
            Override {
                prop: "accent".into(),
                value: "#42BE65".into(),
            },
        ],
        vec![],
        &mut next,
    );
    if let SvelteNode::Element(e) = &mut inst {
        e.style.top = Some(360.0);
        e.style.left = Some(100.0);
        // Visibility of the whole instance rides on the page-level `open`
        // variable — {#if open} wraps the instance in App.svelte.
        e.binding = Some(Binding {
            var: "open".into(),
            transition: Some(Transition::Slide { ms: 200 }),
        });
    }
    inst
}

/// The page-level trigger for the demo: a button that toggles the `open`
/// variable living on the root frame — exactly the §4.5 example
/// (On Click → Toggle open → {#if open} around the dialog instance).
fn show_dialog_button() -> SvelteNode {
    let mut btn = new_button_for_demo(19, 100.0, 500.0, "Show dialog");
    if let SvelteNode::Element(e) = &mut btn {
        e.style.fill = Some(Color::hex(0x0D99FF));
        e.style.fill_hex = "#0D99FF".into();
        e.behavior = Some(Behavior {
            vars: vec![],
            rules: vec![Rule {
                trigger: Trigger::OnClick,
                actions: vec![Action::ToggleVar("open".into())],
            }],
        });
        e.hover = Some(HoverStyle {
            fill: Some("#3DB6FF".into()),
            color: None,
            opacity: None,
        });
    }
    btn
}

/// A ready-made labeled button on the page (ids: button `id`, label `id + 1`).
fn new_button_for_demo(id: u64, x: f32, y: f32, label: &str) -> SvelteNode {
    let mut text = crate::model::svelte_ast::new_text(id + 1, 0.0, 0.0, label);
    if let SvelteNode::Text(t) = &mut text {
        t.name = format!("Button label {id}");
        t.style.font_size = 13.0;
        t.style.font_weight = 600;
        t.width = None;
        t.height = None;
    }
    SvelteNode::Element(Element {
        id,
        name: format!("Button {id}"),
        tag: Tag::Button,
        style: Style {
            display: Display::None,
            position: Position::Absolute,
            width: Dimension::Auto,
            height: Dimension::Auto,
            top: Some(y),
            left: Some(x),
            padding: sequoia_ui::prelude::Edges::xy(16.0, 8.0),
            radius: 8.0,
            ..Default::default()
        },
        children: vec![text],
        visible: true,
        locked: false,
        component: false,
        instance: false,
        interface: None,
        overrides: Vec::new(),
        slot_content: Vec::new(),
        behavior: None,
        binding: None,
        hover: None,
        prototype: None,
    })
}

pub(crate) fn demo_tree() -> SvelteNode {
    let frame = Element {
        id: 1,
        name: "Frame 1".into(),
        tag: Tag::Div,
        style: Style {
            display: Display::Flex,
            position: Position::Relative,
            width: Dimension::Fixed(800.0),
            height: Dimension::Fixed(600.0),
            top: Some(0.0),
            left: Some(0.0),
            fill: Some(Color::hex(0x1B1B1B)),
            fill_hex: "#1B1B1B".into(),
            ..Default::default()
        },
        slot_content: Vec::new(),
        behavior: Some(Behavior {
            // Page-level state: the dialog visibility (design doc §4.5).
            vars: vec![Var {
                name: "open".into(),
                ty: VarType::Bool,
                initial: crate::model::behavior::Value::Bool(false),
            }],
            rules: vec![],
        }),
        binding: None,
        hover: None,
        prototype: None,
        children: vec![
            SvelteNode::Element(Element {
                id: 2,
                name: "Rectangle 2".into(),
                tag: Tag::Div,
                style: Style {
                    display: Display::None,
                    position: Position::Absolute,
                    width: Dimension::Fixed(220.0),
                    height: Dimension::Fixed(140.0),
                    top: Some(100.0),
                    left: Some(100.0),
                    fill: Some(Color::hex(0x0D99FF)),
                    fill_hex: "#0D99FF".into(),
                    radius: 8.0,
                    ..Default::default()
                },
                children: vec![],
                visible: true,
                locked: false,
                component: false,
                instance: false,
                interface: None,
                overrides: Vec::new(),
                slot_content: Vec::new(),
                behavior: None,
                binding: None,
                hover: None,
                prototype: None,
            }),
            SvelteNode::Element(Element {
                id: 3,
                name: "Ellipse 3".into(),
                tag: Tag::Div,
                style: Style {
                    display: Display::None,
                    position: Position::Absolute,
                    width: Dimension::Fixed(120.0),
                    height: Dimension::Fixed(120.0),
                    top: Some(100.0),
                    left: Some(360.0),
                    fill: Some(Color::hex(0x42BE65)),
                    fill_hex: "#42BE65".into(),
                    radius: 9999.0,
                    ..Default::default()
                },
                children: vec![],
                visible: true,
                locked: false,
                component: false,
                instance: false,
                interface: None,
                overrides: Vec::new(),
                slot_content: Vec::new(),
                behavior: None,
                binding: None,
                hover: None,
                prototype: None,
            }),
            SvelteNode::Text(TextNode {
                id: 4,
                name: "Title".into(),
                content: "Design with Sequoia".into(),
                style: TextStyle {
                    font_size: 28.0,
                    font_weight: 400,
                    color: Color::hex(0xFFFFFF),
                    color_hex: "#FFFFFF".into(),
                    align: Align::Start,
                    opacity: 100.0,
                },
                width: Some(380.0),
                height: Some(40.0),
                x: 0.0,
                y: 0.0,
                absolute: false,
                visible: true,
                locked: false,
            }),
            overlay_master(),
            overlay_instance(),
            show_dialog_button(),
        ],
        visible: true,
        locked: false,
        component: false,
        instance: false,
        interface: None,
        overrides: Vec::new(),
    };
    SvelteNode::Element(frame)
}

/// A two-screen starter document wired with prototype links, so Present mode
/// demonstrates screen navigation (`Home ⇄ Details`).
pub(crate) fn prototype_tree() -> SvelteNode {
    fn screen(
        id: u64,
        name: &str,
        fill: u32,
        label: &str,
        button: &str,
        target: u64,
    ) -> SvelteNode {
        let mut title = new_text(id + 1, 40.0, 40.0, label);
        if let SvelteNode::Text(t) = &mut title {
            t.name = format!("{name} title");
            t.style.font_size = 34.0;
            t.style.font_weight = 600;
            t.style.color = Color::WHITE;
            t.style.color_hex = "#FFFFFF".into();
            t.width = None;
            t.height = None;
            t.absolute = true;
            t.x = 40.0;
            t.y = 40.0;
        }

        let mut btn = crate::model::svelte_ast::new_button(id + 2, 40.0, 120.0);
        if let SvelteNode::Element(e) = &mut btn {
            e.name = format!("{name} button");
            e.style.fill = Some(Color::hex(0x0D99FF));
            e.style.fill_hex = "#0D99FF".into();
            e.style.top = Some(120.0);
            e.style.left = Some(40.0);
            e.prototype = Some(PrototypeLink::new(Some(target)));
            if let Some(SvelteNode::Text(t)) = e.children.first_mut() {
                t.content = button.into();
                t.width = None;
                t.height = None;
            }
        }

        let mut frame = new_frame(id, 0.0, 0.0, 800.0, 600.0);
        if let SvelteNode::Element(e) = &mut frame {
            e.name = name.into();
            e.style.fill = Some(Color::hex(fill));
            e.style.fill_hex = format!("#{fill:06X}");
            e.children = vec![title, btn];
        }
        frame
    }

    let mut home = screen(1, "Home", 0x16161A, "Home", "Open details", 10);
    if let SvelteNode::Element(e) = &mut home {
        e.style.gradient = Some(GradientDef {
            kind: GradientKind::Linear,
            angle: 160.0,
            stops: vec![
                GradientStopDef::new(Color::hex(0x1B2A4A), "#1B2A4A", 0.0),
                GradientStopDef::new(Color::hex(0x0D99FF), "#0D99FF", 1.0),
            ],
        });
    }
    let details = screen(10, "Details", 0x101018, "Details", "Back home", 1);

    let mut root = new_frame(20, 0.0, 0.0, 800.0, 600.0);
    if let SvelteNode::Element(e) = &mut root {
        e.name = "Prototype".into();
        // Invisible wrapper: one screen at a time is focused while presenting.
        e.style.display = Display::None;
        e.style.width = Dimension::Fixed(800.0);
        e.style.height = Dimension::Fixed(600.0);
        e.children = vec![home, details];
    }
    root
}

/// A second starter document, so the project tabs have something distinct to
/// switch to on the first launch.
pub(crate) fn landing_tree() -> SvelteNode {
    let mut heading = new_text(2, 48.0, 48.0, "Acme Dashboard");
    if let SvelteNode::Text(t) = &mut heading {
        t.name = "Heading".into();
        t.style.font_size = 30.0;
        t.style.font_weight = 600;
        t.style.color = Color::WHITE;
        t.style.color_hex = "#FFFFFF".into();
        t.width = None;
        t.height = None;
        t.absolute = true;
        t.x = 48.0;
        t.y = 48.0;
    }

    let mut subtitle = new_text(3, 48.0, 92.0, "Design system · 12 components");
    if let SvelteNode::Text(t) = &mut subtitle {
        t.name = "Subtitle".into();
        t.style.font_size = 14.0;
        t.style.color = Color::hex(0x8A8A93);
        t.style.color_hex = "#8A8A93".into();
        t.width = None;
        t.height = None;
        t.absolute = true;
        t.x = 48.0;
        t.y = 92.0;
    }

    let mut card_a = new_rect(4, 48.0, 150.0, 220.0, 140.0, 0x0D99FF);
    let mut card_b = new_rect(5, 288.0, 150.0, 220.0, 140.0, 0x42BE65);
    let mut card_c = new_rect(6, 528.0, 150.0, 220.0, 140.0, 0xF1C21B);
    for (card, name) in [
        (&mut card_a, "Card Blue"),
        (&mut card_b, "Card Green"),
        (&mut card_c, "Card Amber"),
    ] {
        if let SvelteNode::Element(e) = card {
            e.name = name.into();
            e.style.radius = 14.0;
            e.style.effects = vec![ShadowEffect::new()];
        }
    }
    // Showcase the fill + effect stack: gradient fill and a soft colored shadow.
    if let SvelteNode::Element(e) = &mut card_a {
        e.style.gradient = Some(GradientDef {
            kind: GradientKind::Linear,
            angle: 135.0,
            stops: vec![
                GradientStopDef::new(Color::hex(0x0D99FF), "#0D99FF", 0.0),
                GradientStopDef::new(Color::hex(0x6E56CF), "#6E56CF", 1.0),
            ],
        });
        e.style.effects = vec![ShadowEffect {
            color: Color::hexa(0x0D99FF66),
            hex: "#0D99FF66".into(),
            x: 0.0,
            y: 16.0,
            blur: 36.0,
            spread: -6.0,
            enabled: true,
        }];
    }
    if let SvelteNode::Element(e) = &mut card_c {
        e.style.blur = 3.0;
    }

    let mut root = new_frame(1, 0.0, 0.0, 800.0, 600.0);
    if let SvelteNode::Element(e) = &mut root {
        e.name = "Landing Page".into();
        e.style.fill = Some(Color::hex(0x16161A));
        e.style.fill_hex = "#16161A".into();
        e.children = vec![heading, subtitle, card_a, card_b, card_c];
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_contains_a_master_and_an_instance() {
        let tree = demo_tree();
        let master = crate::model::component_ops::find_master(&tree, "Overlay")
            .expect("demo has an Overlay master");
        assert!(master.interface.as_ref().unwrap().props.len() >= 3);

        let flat = crate::model::layout_engine::compute_layout(&Some(tree.clone()));
        let instance = flat.iter().find(|f| {
            matches!(
                &f.kind,
                crate::model::layout_engine::FlatKind::Element { instance: true, .. }
            )
        });
        assert!(instance.is_some(), "demo has an Overlay instance");
    }
}
