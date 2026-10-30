#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::Length;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn wrapped_text_fits_available_width() {
    let h = Harness::new(
        Column::new()
            .width(Length::Px(200.0))
            .padding(16.0)
            .child(
                Text::new(
                    "Jasne tlo, cienie, gradienty i blur sa widoczne od razu bez zadnych dodatkowych ustawien.",
                )
                .size(11.0)
                .wrap(),
            ),
    );
    h.layout(320.0, 200.0);

    let root = h.root().expect("root");
    let text = h.children(root)[0];
    let root_rect = h.rect(root).expect("root rect");
    let text_rect = h.rect(text).expect("text rect");
    let content_right = root_rect.right() - 16.0;
    assert!(
        text_rect.right() <= content_right + 0.01,
        "text overflows: text {text_rect:?} content right {content_right}"
    );
    assert!(
        text_rect.size.height > 20.0,
        "text should wrap to multiple lines: {text_rect:?}"
    );
}
