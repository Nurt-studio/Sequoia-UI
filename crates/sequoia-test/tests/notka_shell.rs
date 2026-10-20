use sequoia_core::Color;
use sequoia_elements::{Container, TextArea};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn find_scroll(h: &Harness) -> ElementId {
    fn rec(h: &Harness, id: ElementId) -> Option<ElementId> {
        if h.type_name_of(id)
            .is_some_and(|n| n.ends_with("ScrollView"))
        {
            return Some(id);
        }
        h.children(id).into_iter().find_map(|c| rec(h, c))
    }
    rec(h, h.root().unwrap()).expect("ScrollView in tree")
}

fn shell() -> (Harness, ElementId) {
    let sv = ScrollView::new(Unit);
    let texts: Vec<Signal<String>> = (0..15)
        .map(|i| Signal::new(format!("blok {i} z jakas trescia")))
        .collect();
    let rows: Vec<AnyView> = texts
        .iter()
        .map(|t| AnyView::new(TextArea::new(t.clone()).font_size(16.0).expand()))
        .collect();
    let page = Column::new()
        .fill_size()
        .child(Container::new().fill_width().height(225.0))
        .child(Column::new().fill_width().children(rows));
    let h = Harness::new(
        Row::new()
            .fill_size()
            .child(
                Container::new()
                    .width(268.0)
                    .fill_height()
                    .background(Color::hex(0x202124)),
            )
            .child(
                Container::new().expand().fill_height().child(
                    Column::new()
                        .fill_size()
                        .child(Container::new().fill_width().height(48.0))
                        .child(sv.clone().with_child(page)),
                ),
            ),
    );
    h.layout(1280.0, 800.0);
    h.paint();
    h.tick();
    let el = find_scroll(&h);
    (h, el)
}

#[test]
fn scrollview_in_row_shell_gets_bounded_viewport() {
    let (h, el) = shell();
    let sv = h.view::<ScrollView>(el).unwrap();
    let rt = h.runtime();
    assert!(
        (sv.viewport_size(&rt, el).height - 752.0).abs() < 1.0,
        "viewport 800-48: {:?}",
        sv.viewport_size(&rt, el)
    );
    assert!(sv.can_scroll_y(&rt, el), "tresc wyzsza od viewportu");
}

#[test]
fn wheel_scrolls_shell_page() {
    let (h, el) = shell();
    let sv = h.view::<ScrollView>(el).unwrap();
    let rt = h.runtime();
    assert!(sv.can_scroll_y(&rt, el));
    h.wheel(800.0, 400.0, -120.0);
    h.paint();
    h.tick();
    assert!(sv.offset_y(&rt, el) > 0.0, "wheel ma przewijac strone");
}
