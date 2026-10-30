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

#[test]
fn text_input_focused_tracks_the_focused_widget() {
    use sequoia_elements::Button;

    let text = Signal::new(String::new());
    let h = Harness::new(
        Column::new()
            .child(Button::new("OK"))
            .child(TextField::new(text.clone())),
    );
    h.layout(300.0, 200.0);
    assert!(!h.runtime().text_input_focused(), "nic nie ma focusu");

    let root = h.root().unwrap();
    let kids = h.children(root);
    let button_rect = h.rect(kids[0]).unwrap();
    h.click(button_rect.center().x, button_rect.center().y);
    assert!(
        !h.runtime().text_input_focused(),
        "przycisk nie jest polem tekstowym"
    );

    let field_rect = h.rect(kids[1]).unwrap();
    h.click(field_rect.center().x, field_rect.center().y);
    assert!(
        h.runtime().text_input_focused(),
        "TextField z focusem blokuje accelerator Delete"
    );

    h.key(Key::from_char('a'));
    assert_eq!(text.get_untracked(), "a", "znak trafia do pola");
}
