#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::prelude::*;
use sequoia_reactive::Signal;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

#[test]
fn panel_paints_inset_quad() {
    let rt = Runtime::new();
    rt.mount(
        Panel::new()
            .background(Color::RED)
            .width(200.0)
            .height(100.0),
    );

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    assert_eq!(list.quads().count(), 1);
    let q = list.quads().next().unwrap();
    assert_eq!(q.color, Color::RED);
    assert_eq!(q.rect.size, Size::new(200.0, 100.0));
}

#[test]
fn empty_runtime_paints_nothing() {
    let rt = Runtime::new();
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert!(list.is_empty());
}

struct DynPanel {
    clicks: Signal<u32>,
}

impl View for DynPanel {
    fn body(&self) -> impl View + 'static {
        let n = self.clicks.get();
        let color = if n.is_multiple_of(2) {
            Color::RED
        } else {
            Color::BLUE
        };

        Panel::new().background(color).width(50.0).height(50.0)
    }
}

#[test]
fn color_follows_signal_after_tick() {
    let clicks = Signal::new(0u32);
    let rt = Runtime::new();
    rt.mount(DynPanel {
        clicks: clicks.clone(),
    });

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 50.0, 50.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert_eq!(list.quads().next().unwrap().color, Color::RED);

    clicks.set(1);
    rt.tick();
    rt.layout(Rect::from_xywh(0.0, 0.0, 50.0, 50.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert_eq!(list.quads().next().unwrap().color, Color::BLUE);
}

#[test]
fn opacity_wrapper_modulates_child_alpha() {
    let rt = Runtime::new();
    rt.mount(Opacity::new(
        Panel::new()
            .background(Color::RED)
            .width(200.0)
            .height(100.0),
        0.5,
    ));

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let q = list.quads().next().expect("dziecko maluje quad");
    assert!(
        (q.color.a - 0.5).abs() < 1e-6,
        "alfa po Opacity(0.5): {}",
        q.color.a
    );
    assert_eq!(q.color.r, 1.0);
}

#[test]
fn opacity_wrappers_nest_multiplicatively() {
    let rt = Runtime::new();
    rt.mount(Opacity::new(
        Opacity::new(
            Panel::new().background(Color::RED).width(50.0).height(50.0),
            0.5,
        ),
        0.5,
    ));

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let q = list.quads().next().expect("quad");
    assert!((q.color.a - 0.25).abs() < 1e-6, "0.5 * 0.5: {}", q.color.a);
}

#[test]
fn translate_wrapper_shifts_child_paint() {
    let rt = Runtime::new();
    rt.mount(Translate::new(
        Panel::new()
            .background(Color::RED)
            .width(200.0)
            .height(100.0),
        10.0,
        20.0,
    ));

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let q = list.quads().next().expect("quad");
    assert_eq!(q.rect.left(), 10.0, "malarskie dx");
    assert_eq!(q.rect.top(), 20.0, "malarskie dy");
    assert_eq!(q.rect.size, Size::new(200.0, 100.0), "rozmiar bez zmian");
}

#[test]
fn nested_translate_wrappers_accumulate() {
    let rt = Runtime::new();
    rt.mount(Translate::new(
        Translate::new(
            Panel::new().background(Color::RED).width(50.0).height(50.0),
            5.0,
            0.0,
        ),
        10.0,
        20.0,
    ));

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let q = list.quads().next().expect("quad");
    assert_eq!(q.rect.left(), 15.0, "dx adds up down");
    assert_eq!(q.rect.top(), 20.0);
}

#[test]
fn text_view_produces_text_item() {
    let rt = Runtime::new();
    rt.mount(Text::new("Sequoia").size(24.0));

    let mut text = TextSystem::new();
    if text.shape("Sequoia", 24.0).glyph_count() == 0 {
        eprintln!("skip: no fonts in environment");
        return;
    }

    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 200.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let Some(RenderItem::Text(draw)) = list
        .items()
        .iter()
        .find(|i| matches!(i, RenderItem::Text(_)))
    else {
        panic!("expected RenderItem::Text");
    };
    assert!(!draw.glyphs.is_empty());
}

#[test]
fn text_style_applies_and_inline_overrides() {
    let base = TextStyle::new()
        .size(20.0)
        .color(Color::RED)
        .weight(700)
        .underline(true);

    let rt = Runtime::new();
    rt.mount(Text::new("Sequoia").style(base.clone()));

    let mut text = TextSystem::new();
    if text.shape("Sequoia", 20.0).glyph_count() == 0 {
        eprintln!("skip: no fonts in environment");
        return;
    }

    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let Some(RenderItem::Text(draw)) = list
        .items()
        .iter()
        .find(|i| matches!(i, RenderItem::Text(_)))
    else {
        panic!("expected RenderItem::Text");
    };
    assert_eq!(draw.color, Color::RED, "color from TextStyle");
    assert_eq!(list.quads().count(), 1, "underline from TextStyle");

    let rt = Runtime::new();
    rt.mount(
        Text::new("Sequoia")
            .style(base)
            .color(Color::BLUE)
            .underline(false),
    );
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 100.0), &mut text);
    rt.paint(&mut list, &mut text);

    let Some(RenderItem::Text(draw)) = list
        .items()
        .iter()
        .find(|i| matches!(i, RenderItem::Text(_)))
    else {
        panic!("expected RenderItem::Text");
    };
    assert_eq!(draw.color, Color::BLUE, "inline color wins");
    assert_eq!(list.quads().count(), 0, "inline underline(false) wins");
}

#[test]
fn text_decoration_paints_a_line_under_the_text() {
    let rt = Runtime::new();
    rt.mount(Text::new("Sequoia").size(20.0).underline(true));

    let mut text = TextSystem::new();
    if text.shape("Sequoia", 20.0).glyph_count() == 0 {
        eprintln!("skip: no fonts in environment");
        return;
    }

    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1, "underline is one quad");
    let d = quads[0];
    assert!(d.rect.size.width > 0.0, "line spans the text width");
    assert!(
        (1.0..3.0).contains(&d.rect.size.height),
        "thickness ~7% em: {}",
        d.rect.size.height
    );
    assert!(
        list.items()
            .iter()
            .any(|i| matches!(i, RenderItem::Text(_)))
    );
}

#[test]
fn text_without_decoration_paints_no_extra_quads() {
    let rt = Runtime::new();
    rt.mount(Text::new("Sequoia").size(20.0));
    let mut text = TextSystem::new();
    if text.shape("Sequoia", 20.0).glyph_count() == 0 {
        eprintln!("skip: no fonts in environment");
        return;
    }
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert_eq!(list.quads().count(), 0, "`Text` alone adds no quads");
}

struct TextOverPanel;

impl View for TextOverPanel {
    fn body(&self) -> impl View + 'static {
        Column::new()
            .child(Text::new("underneath"))
            .child(
                Panel::new()
                    .background(Color::RED)
                    .width(200.0)
                    .height(100.0),
            )
            .child(Text::new("on top"))
    }
}

#[test]
fn render_list_keeps_paint_order_across_types() {
    let rt = Runtime::new();
    rt.mount(TextOverPanel);

    let mut text = TextSystem::new();
    if text.shape("underneath", 14.0).glyph_count() == 0 {
        eprintln!("skip: no fonts in environment");
        return;
    }

    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    let kinds: Vec<char> = list
        .items()
        .iter()
        .map(|i| match i {
            RenderItem::Text(_) => 'T',
            RenderItem::Quad(_) => 'Q',
            RenderItem::Image(_) => 'I',
            RenderItem::Shadow(_) => 'S',
            RenderItem::Waveform(_) => 'W',

            _ => '?',
        })
        .collect();
    assert_eq!(kinds, vec!['T', 'Q', 'T'], "paint order: {kinds:?}");
}

struct BackdropApp {
    on: Signal<bool>,
}

impl View for BackdropApp {
    fn body(&self) -> impl View + 'static {
        let on = self.on.get();
        Stack::new()
            .width(400.0)
            .height(400.0)
            .fill(
                Panel::new()
                    .background(Color::RED)
                    .width(400.0)
                    .height(400.0),
            )
            .child(
                Panel::new()
                    .background(Color::BLUE)
                    .width(200.0)
                    .height(100.0)
                    .blur_behind_with(move |_| {
                        if on {
                            Backdrop::blur(16.0).radius(12.0)
                        } else {
                            Backdrop::NONE
                        }
                    }),
            )
    }
}

#[test]
fn backdrop_toggle_emits_marker_between_layers() {
    let on = Signal::new(false);
    let rt = Runtime::new();
    rt.mount(BackdropApp { on: on.clone() });

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert!(
        !list.has_backdrops(),
        "disabled effect does not emit marker"
    );

    on.set(true);
    rt.tick();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert_eq!(list.backdrops().len(), 1);
    let span = list.backdrops()[0];
    assert!(span.at > 0, "background below: at={}", span.at);
    assert!(
        span.at < list.len(),
        "content of the panel above the marker: at={} len={}",
        span.at,
        list.len()
    );
    assert_eq!(span.draw.blur, 16.0);
    assert_eq!(span.draw.radius, 12.0);
    assert_eq!(span.draw.rect.size, Size::new(200.0, 100.0));

    on.set(false);
    rt.tick();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert!(!list.has_backdrops(), "disabling clears the marker");
}

#[test]
fn filter_wrapper_opens_span_over_child_items() {
    let rt = Runtime::new();
    rt.mount(
        Panel::new()
            .background(Color::RED)
            .width(200.0)
            .height(100.0)
            .blur_own(16.0),
    );

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    assert!(list.has_filters(), "niepusty filtr otwiera zakres");
    assert_eq!(list.filter_spans().len(), 1);
    let span = list.filter_spans()[0];
    assert_eq!((span.start, span.end), (0, 1), "quad panelu w zakresie");
    assert_eq!(span.draw.blur, 16.0);
    assert_eq!(span.draw.rect.size, Size::new(200.0, 100.0));

    assert_eq!(list.quads().count(), 1);
}

#[test]
fn filter_mask_rides_span_to_renderer() {
    use sequoia_style::gradient;
    let rt = Runtime::new();
    rt.mount(
        Panel::new()
            .background(Color::RED)
            .width(200.0)
            .height(100.0)
            .blur_own_with(|b| b.blur_radius(16.0).mask(gradient(1.0, 0.0))),
    );

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    assert!(list.has_filters());
    let span = list.filter_spans()[0];
    assert_eq!(
        span.draw.mask.mask_vec(),
        [1.0, 1.0, 0.0, std::f32::consts::FRAC_PI_2],
        "rampa pionowa do shadera"
    );
}

#[test]
fn backdrop_filter_mask_rides_marker() {
    use sequoia_style::gradient;
    let rt = Runtime::new();
    rt.mount(
        Panel::new()
            .background(Color::RED)
            .width(200.0)
            .height(100.0)
            .blur_behind_with(|b| b.blur_radius(8.0).mask(gradient(1.0, 0.0))),
    );

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    assert!(list.has_backdrops());
    assert_eq!(
        list.backdrops()[0].draw.mask.mask_vec()[0],
        1.0,
        "marker carries ramp mask"
    );
}

#[test]
fn filter_none_costs_nothing() {
    let rt = Runtime::new();
    rt.mount(
        Panel::new()
            .background(Color::RED)
            .width(200.0)
            .height(100.0)
            .blur_own(0.0),
    );

    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    assert!(!list.has_filters(), "pusty filtr nie emituje nic");
    assert_eq!(list.quads().count(), 1);
}

#[test]
fn typing_keeps_frames_deterministic_and_blur_marker_stable() {
    let text = Signal::new("ala".to_string());
    let probe = text.clone();
    let rt = Runtime::new();
    rt.mount(
        Column::new()
            .child(Text::with(move || probe.get()))
            .child(Text::new("pill").blur_behind(4.0)),
    );

    let mut textsys = TextSystem::new();
    let mut frame = || {
        rt.layout(Rect::from_xywh(0.0, 0.0, 400.0, 400.0), &mut textsys);
        let mut list = RenderList::new();
        rt.paint(&mut list, &mut textsys);
        list
    };

    let dump = |list: &RenderList| format!("{:?}", list);
    assert_eq!(
        dump(&frame()),
        dump(&frame()),
        "klatki bez zmian identyczne"
    );

    let mark = |list: &RenderList| {
        assert_eq!(list.backdrops().len(), 1, "jeden marker blur");
        list.backdrops()[0].draw.rect
    };
    let r0 = mark(&frame());

    for word in ["ala ", "ala m", "ala ma", "ala ma ", "ala ma k"] {
        text.set(word.to_string());
        rt.tick();
        let list = frame();
        assert_eq!(mark(&list), r0, "marker stoi w miejscu podczas pisania");
        assert_eq!(dump(&list), dump(&frame()), "repeated paint stable");
    }
}
