#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Color;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_ui::prelude::*;

#[test]
fn dialog_buttons_use_primary_and_outline_styles() {
    let h = Harness::new(
        Column::new().fill_size().child(
            Dialog::new(Signal::new(true), "Zapisać zmiany?")
                .content(Text::new("Treść"))
                .action("Zapisz", || {})
                .cancel("Anuluj"),
        ),
    );
    h.sync(600.0, 400.0);

    let list = h.paint();
    let quads: Vec<_> = list.quads().collect();
    assert!(
        quads
            .iter()
            .any(|q| Some(q.color) == ButtonStyle::PRIMARY.background),
        "action button uses the primary style: {:?}",
        quads.iter().map(|q| q.color).collect::<Vec<_>>()
    );
    assert!(
        quads
            .iter()
            .any(|q| q.color == Color::TRANSPARENT && q.has_border()),
        "cancel button is outlined"
    );
    assert!(
        !quads.iter().any(|q| q.color == Color::hex(0x2A2A2A)),
        "dialog must not fall back to the default dark button fill"
    );
}

#[test]
fn dialog_scrim_blurs_the_backdrop() {
    let dialog = Dialog::new(Signal::new(true), "T").cancel("Anuluj");
    let h = Harness::new(
        Stack::new()
            .fill_size()
            .child_at(dialog.scrim(), StackPos::fill()),
    );
    h.sync(600.0, 400.0);

    let list = h.paint();
    assert!(
        list.backdrops().iter().any(|span| span.draw.blur >= 10.0),
        "the dialog scrim requests a blur backdrop"
    );
}
