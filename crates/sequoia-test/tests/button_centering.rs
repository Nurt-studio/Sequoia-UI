use sequoia_core::RenderItem;
use sequoia_elements::Button;
use sequoia_style::Theme;
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::Align;

#[test]
fn button_labels_are_centered_at_their_painted_size() {
    let theme = Theme::dark();
    let mut metrics = TextSystem::new();

    for label in ["− 1", "Reset", "+ 1"] {
        let h = Harness::new(Button::labeled(label));
        h.layout(200.0, 80.0);

        let button = h.rect(h.root().expect("button")).expect("button rect");
        let button_center = (button.left() + button.right()) * 0.5;
        let expected_width = metrics.text_width(label, theme.font_size);
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

#[test]
fn align_controls_label_position_in_a_fixed_button() {
    for align in [Align::Start, Align::Center, Align::End] {
        let h = Harness::new(Button::labeled("16").align(align).width(45.0).height(20.0));
        h.layout(200.0, 80.0);

        let button = h.rect(h.root().expect("button")).expect("button rect");
        let button_center = (button.left() + button.right()) * 0.5;
        let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
        for item in h.paint().items() {
            if let RenderItem::Text(text) = item {
                for glyph in &text.glyphs {
                    min_x = min_x.min(glyph.rect.left());
                    max_x = max_x.max(glyph.rect.right());
                }
            }
        }
        let ink_center = (min_x + max_x) * 0.5;

        match align {
            Align::Start => assert!(
                max_x < button_center,
                "Start: ink ends at {max_x}, button center {button_center}"
            ),
            Align::Center => assert!(
                (ink_center - button_center).abs() <= 1.0,
                "Center: ink center {ink_center} vs {button_center}"
            ),
            Align::End => assert!(
                min_x > button_center,
                "End: ink starts at {min_x}, button center {button_center}"
            ),
            other => panic!("unexpected alignment {other:?}"),
        }
    }
}
