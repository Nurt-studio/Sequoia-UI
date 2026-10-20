use sequoia_core::{Color, RenderItem};
use sequoia_elements::Kbd;
use sequoia_test::Harness;

#[test]
fn chip_paints_background_and_text() {
    let h = Harness::new(Kbd::new("Ctrl"));
    h.layout(300.0, 100.0);
    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1, "chip to jeden bordered quad");
    assert_eq!(quads[0].border_width, 1.0);
    assert!(
        list.items()
            .iter()
            .any(|i| matches!(i, RenderItem::Text(_))),
        "etykieta maluje tekst"
    );
}

#[test]
fn wider_label_gives_wider_chip() {
    let short = Harness::new(Kbd::new("K"));
    short.layout(300.0, 100.0);
    let long = Harness::new(Kbd::new("Control"));
    long.layout(300.0, 100.0);
    let sw = short.rect(short.root().unwrap()).unwrap().size.width;
    let lw = long.rect(long.root().unwrap()).unwrap().size.width;
    assert!(lw > sw, "szersza etykieta: {lw} > {sw}");
}

#[test]
fn empty_label_paints_nothing() {
    let h = Harness::new(Kbd::new(""));
    h.layout(300.0, 100.0);
    assert!(h.paint().is_empty(), "pusty Kbd nic nie maluje");
}

#[test]
fn style_override_changes_background() {
    let h = Harness::new(Kbd::new("K").background(Color::RED));
    h.layout(300.0, 100.0);
    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert_eq!(quads.len(), 1);
    assert_eq!(quads[0].color, Color::RED);
    assert_eq!(quads[0].border_width, 1.0);
}
