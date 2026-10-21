use sequoia_core::RenderItem;
use sequoia_elements::Button;
use sequoia_style::Theme;
use sequoia_test::Harness;
use sequoia_text::TextSystem;

#[test]
fn button_labels_are_centered_at_their_painted_size() {
    let theme = Theme::dark();
    let mut metrics = TextSystem::new();

    for label in ["− 1", "Reset", "+ 1"] {
        let h = Harness::new(Button::labeled(label));
        h.layout(200.0, 80.0);

        let button = h.rect(h.root().expect("button")).expect("button rect");
        let button_center = (button.left() + button.right()) * 0.5;
        let expected_width = metrics.text_width(label, theme.font_size) + 20.0;
        assert!(
            (button.size.width - expected_width).abs() < 0.01,
            "label {label:?}: button width {} vs measured width {expected_width}",
            button.size.width
        );

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        for item in h.paint().items() {
            if let RenderItem::Text(text) = item {
                for glyph in &text.glyphs {
                    min_x = min_x.min(glyph.rect.left());
                    max_x = max_x.max(glyph.rect.right());
                }
            }
        }

        let ink_center = (min_x + max_x) * 0.5;
        assert!(
            (ink_center - button_center).abs() <= 1.0,
            "label {label:?}: ink center {ink_center} vs button center {button_center}"
        );
    }
}
