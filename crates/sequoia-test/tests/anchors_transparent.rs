use sequoia_elements::{Button, Drawer, Modal, ToastQueue, ToastStack};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn shell(
    clicked: Signal<bool>,
    modal_open: Signal<bool>,
    drawer_open: Signal<bool>,
    queue: ToastQueue,
) -> impl View {
    Stack::new()
        .fill_size()
        .child_at(
            Column::new()
                .fill_size()
                .child(Button::labeled("Klik").on_click({
                    let clicked = clicked.clone();
                    move || clicked.set(true)
                })),
            StackPos::fill(),
        )
        .child_at(Modal::new(modal_open.clone()).scrim(), StackPos::fill())
        .child_at(
            Modal::new(modal_open).width(200.0).child(Text::new("M")),
            StackPos::fill(),
        )
        .child_at(Drawer::new(drawer_open.clone()).scrim(), StackPos::fill())
        .child_at(
            Drawer::new(drawer_open).child(Text::new("D")),
            StackPos::fill(),
        )
        .child_at(ToastStack::new(queue), StackPos::fill())
}

#[test]
fn closed_overlay_anchors_let_clicks_through() {
    let clicked = Signal::new(false);
    let h = Harness::new(shell(
        clicked.clone(),
        Signal::new(false),
        Signal::new(false),
        ToastQueue::new(),
    ));
    h.layout(400.0, 300.0);
    h.paint();
    h.tick();

    let stack = h.root().unwrap();
    let col = h.children(stack)[0];
    let btn = h.children(col)[0];
    let r = h.rect(btn).unwrap();
    h.click(r.left() + r.size.width * 0.5, r.top() + r.size.height * 0.5);
    assert!(
        clicked.get_untracked(),
        "klik dociera do przycisku pod kotwicami"
    );
}
