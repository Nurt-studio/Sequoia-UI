use sequoia_elements::TextField;
use sequoia_input::Key;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn trapped_harness() -> Harness {
    Harness::new(
        Column::new()
            .child(TextField::new(Signal::new(String::new())))
            .child(
                FocusScope::new()
                    .trap(true)
                    .child(TextField::new(Signal::new(String::new())))
                    .child(TextField::new(Signal::new(String::new()))),
            )
            .child(TextField::new(Signal::new(String::new()))),
    )
}

fn focused_index(h: &Harness) -> Option<usize> {
    fn collect(h: &Harness, id: ElementId, out: &mut Vec<ElementId>) {
        if h.type_name_of(id).unwrap_or_default().contains("TextField") {
            out.push(id);
        }
        for c in h.children(id) {
            collect(h, c, out);
        }
    }
    let mut fields = Vec::new();
    if let Some(root) = h.root() {
        collect(h, root, &mut fields);
    }
    let focus = h.interact().focus?;
    fields.iter().position(|&e| e == focus)
}

#[test]
fn trap_cycles_inside_scope() {
    let h = trapped_harness();
    h.layout(400.0, 400.0);

    let root = h.root().unwrap();
    let scope = h.children(root)[1];
    let b = h.children(scope)[0];
    let b_rect = h.rect(b).unwrap();
    h.pointer_down(b_rect.left() + 4.0, b_rect.top() + 4.0);
    assert_eq!(focused_index(&h), Some(1));

    h.key(Key::TAB);
    assert_eq!(focused_index(&h), Some(2));
    h.key(Key::TAB);
    assert_eq!(focused_index(&h), Some(1), "trap wraps to B");
    h.key(Key::TAB);
    assert_eq!(focused_index(&h), Some(2));
}

#[test]
fn no_trap_tabs_through_everything() {
    let h = Harness::new(
        Column::new()
            .child(TextField::new(Signal::new(String::new())))
            .child(
                FocusScope::new()
                    .child(TextField::new(Signal::new(String::new())))
                    .child(TextField::new(Signal::new(String::new()))),
            )
            .child(TextField::new(Signal::new(String::new()))),
    );
    h.layout(400.0, 400.0);

    let root = h.root().unwrap();
    let a = h.children(root)[0];
    let a_rect = h.rect(a).unwrap();
    h.pointer_down(a_rect.left() + 4.0, a_rect.top() + 4.0);
    assert_eq!(focused_index(&h), Some(0));

    for expected in [1, 2, 3, 0] {
        h.key(Key::TAB);
        assert_eq!(focused_index(&h), Some(expected));
    }
}
