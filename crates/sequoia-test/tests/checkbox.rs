use std::cell::Cell;
use std::rc::Rc;

use sequoia_elements::Checkbox;
use sequoia_reactive::Signal;
use sequoia_test::Harness;

#[test]
fn click_toggles_and_fires_callback() {
    let checked = Signal::new(false);
    let last = Rc::new(Cell::new(None::<bool>));
    let h = Harness::new(Checkbox::new(checked.clone()).on_toggle({
        let last = last.clone();
        move |v| last.set(Some(v))
    }));
    h.layout(200.0, 60.0);
    h.click(8.0, 8.0);
    assert!(checked.get_untracked());
    assert_eq!(last.get(), Some(true));
    h.click(8.0, 8.0);
    assert!(!checked.get_untracked());
    assert_eq!(last.get(), Some(false));
}

#[test]
fn keyboard_space_toggles_when_focused() {
    use sequoia_input::Key;
    let checked = Signal::new(false);
    let h = Harness::new(Checkbox::new(checked.clone()));
    h.layout(200.0, 60.0);

    h.click(8.0, 8.0);
    assert!(checked.get_untracked());
    h.key(Key::SPACE);
    assert!(!checked.get_untracked(), "space toggles");
    h.key(Key::ENTER);
    assert!(checked.get_untracked(), "enter toggles");
}

#[test]
fn disabled_ignores_input() {
    let checked = Signal::new(false);
    let h = Harness::new(Checkbox::new(checked.clone()).disabled(true));
    h.layout(200.0, 60.0);
    h.click(8.0, 8.0);
    assert!(!checked.get_untracked());
}

#[test]
fn check_icon_is_centered_in_box() {
    let checked = Signal::new(true);
    let h = Harness::new(Checkbox::new(checked).box_size(20.0));
    h.layout(200.0, 60.0);

    let mut stack = vec![h.root().expect("root")];
    let mut icon = None;
    while let Some(id) = stack.pop() {
        if h.type_name_of(id).is_some_and(|name| name.contains("Icon")) {
            icon = Some(id);
            break;
        }
        stack.extend(h.children(id));
    }

    let rect = h.rect(icon.expect("check icon")).expect("icon rect");
    assert!((rect.left() - 2.0).abs() < 0.01, "left: {rect:?}");
    assert!((rect.top() - 2.0).abs() < 0.01, "top: {rect:?}");
    assert!((rect.size.width - 16.0).abs() < 0.01, "width: {rect:?}");
    assert!((rect.size.height - 16.0).abs() < 0.01, "height: {rect:?}");
}
