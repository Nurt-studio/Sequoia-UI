#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_core::{Point, Vec2};
use sequoia_elements::{Container, Draggable, Droppable};
use sequoia_input::CursorIcon;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

fn source_target(on_drop: impl Fn(String, DropPosition) + 'static) -> impl View + 'static {
    Row::new()
        .child(
            Draggable::new(Text::new("src"), "task-1".to_string())
                .width(100.0)
                .height(40.0),
        )
        .child(
            Droppable::new(Container::new().child(Text::new("dst")), on_drop)
                .width(100.0)
                .height(40.0),
        )
}

#[test]
fn click_without_move_is_not_a_drag() {
    let drops = Signal::new(0u32);
    let starts = Signal::new(0u32);
    let ends = Signal::new(0u32);
    let (d, s, e) = (drops.clone(), starts.clone(), ends.clone());
    let h = Harness::new(
        Row::new().child(
            Draggable::new(Text::new("src"), "x".to_string())
                .width(100.0)
                .height(40.0)
                .on_drag_start(move || s.update(|n| *n += 1))
                .on_drag_end(move |_| e.update(|n| *n += 1)),
        ),
    );
    h.layout(400.0, 200.0);
    h.pointer_down(10.0, 10.0);
    assert!(h.has_pending_drag(), "down registers a pending drag");
    assert!(h.drag_session().is_none(), "no session without movement");
    h.pointer_up(10.0, 10.0);
    h.tick();
    assert!(h.drag_session().is_none());
    assert!(!h.has_pending_drag());
    assert_eq!(starts.get_untracked(), 0);
    assert_eq!(ends.get_untracked(), 0);
    assert_eq!(drops.get_untracked(), 0);
    let _ = d;
}

#[test]
fn click_inside_draggable_still_clicks_child() {
    let fired = Signal::new(0u32);
    let f = fired.clone();
    let h = Harness::new(
        Row::new().child(
            Draggable::new(
                Container::new()
                    .child(Text::new("btn"))
                    .on_click(move || f.update(|n| *n += 1)),
                "x".to_string(),
            )
            .width(100.0)
            .height(40.0),
        ),
    );
    h.layout(400.0, 200.0);
    h.click(10.0, 10.0);
    h.tick();
    assert_eq!(
        fired.get_untracked(),
        1,
        "click on a child works despite the wrapper"
    );
}

#[test]
fn small_move_below_threshold_is_click() {
    let h = Harness::new(source_target(|_: String, _: DropPosition| {}));
    h.layout(400.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_move(11.0, 11.0);
    assert!(
        h.drag_session().is_none(),
        "sub-threshold move is not a drag"
    );
    h.pointer_up(11.0, 11.0);
    h.tick();
    assert!(h.drag_session().is_none());
}

#[test]
fn drag_starts_after_threshold_with_grab_offset() {
    let h = Harness::new(source_target(|_: String, _: DropPosition| {}));
    h.layout(400.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_move(150.0, 20.0);
    let s = h
        .drag_session()
        .expect("session after crossing the threshold");
    assert_eq!(s.origin, Point::new(10.0, 10.0));
    assert_eq!(
        s.grab_offset,
        Vec2::new(10.0, 10.0),
        "grab = down - source corner"
    );
    assert_eq!(s.current, Point::new(150.0, 20.0), "precise position");
    assert!(s.target.is_some(), "target under the cursor detected");
    assert_eq!(h.cursor_icon(), CursorIcon::Grabbing);
}

#[test]
fn drop_delivers_typed_payload_and_local_position() {
    let got = Signal::new(String::new());
    let at = Signal::new((0.0f32, 0.0f32, 0.0f32, 0.0f32));
    let ended = Signal::new((false, false));
    let (g, a, en) = (got.clone(), at.clone(), ended.clone());
    let h = Harness::new(source_target(move |id: String, pos: DropPosition| {
        g.set(id);
        a.set((pos.local.x, pos.local.y, pos.fraction.x, pos.fraction.y));
    }));

    let _ = en;
    h.layout(400.0, 200.0);
    h.drag((10.0, 10.0), (150.0, 20.0));
    h.tick();
    assert_eq!(got.get_untracked(), "task-1");
    let (lx, ly, fx, fy) = at.get_untracked();
    assert_eq!((lx, ly), (50.0, 20.0), "local to the target (100,0)");
    assert!((fx - 0.5).abs() < 1e-6 && (fy - 0.5).abs() < 1e-6);
    assert!(h.drag_session().is_none(), "session closed after drop");
    assert_eq!(h.overlay_count(), 0, "preview cleaned up");
}

#[test]
fn drop_end_reports_dropped() {
    let ended = Signal::new((false, false));
    let en = ended.clone();
    let h = Harness::new(
        Row::new().child(
            Draggable::new(Text::new("src"), "x".to_string())
                .width(100.0)
                .height(40.0)
                .on_drag_end(move |end| en.set((end.dropped, end.cancelled))),
        ),
    );

    h.layout(400.0, 200.0);
    h.drag((10.0, 10.0), (300.0, 150.0));
    h.tick();
    assert_eq!(ended.get_untracked(), (false, false));
}

#[test]
fn drop_on_target_reports_dropped_true() {
    let ended = Signal::new((false, false));
    let en = ended.clone();
    let h = Harness::new(
        Row::new()
            .child(
                Draggable::new(Text::new("src"), "x".to_string())
                    .width(100.0)
                    .height(40.0)
                    .on_drag_end(move |end| en.set((end.dropped, end.cancelled))),
            )
            .child(
                Droppable::new(Text::new("dst"), |_: String, _: DropPosition| {})
                    .width(100.0)
                    .height(40.0),
            ),
    );
    h.layout(400.0, 200.0);
    h.drag((10.0, 10.0), (150.0, 20.0));
    h.tick();
    assert_eq!(ended.get_untracked(), (true, false));
}

#[test]
fn wrong_type_is_rejected() {
    let drops = Signal::new(0u32);
    let d = drops.clone();
    let h = Harness::new(
        Row::new()
            .child(
                Draggable::new(Text::new("src"), "text".to_string())
                    .width(100.0)
                    .height(40.0),
            )
            .child(
                Droppable::new(Text::new("dst"), move |_: u32, _: DropPosition| {
                    d.update(|n| *n += 1);
                })
                .width(100.0)
                .height(40.0),
            ),
    );
    h.layout(400.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_move(150.0, 20.0);
    let s = h
        .drag_session()
        .expect("session starts regardless of target");
    assert_eq!(s.target, None, "mismatched type means no target");
    h.pointer_up(150.0, 20.0);
    h.tick();
    assert_eq!(drops.get_untracked(), 0, "handler not called");
}

#[test]
fn escape_cancels_drag() {
    let drops = Signal::new(0u32);
    let ended = Signal::new((false, false));
    let (d, en) = (drops.clone(), ended.clone());
    let h = Harness::new(
        Row::new()
            .child(
                Draggable::new(Text::new("src"), "x".to_string())
                    .width(100.0)
                    .height(40.0)
                    .preview(Text::new("ghost"))
                    .on_drag_end(move |end| en.set((end.dropped, end.cancelled))),
            )
            .child(
                Droppable::new(Text::new("dst"), move |_: String, _: DropPosition| {
                    d.update(|n| *n += 1);
                })
                .width(100.0)
                .height(40.0),
            ),
    );
    h.layout(400.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_move(150.0, 20.0);
    assert!(h.drag_session().is_some());
    assert_eq!(h.overlay_count(), 1, "preview visible");
    h.drag_cancel();
    h.tick();
    assert!(h.drag_session().is_none());
    assert_eq!(h.overlay_count(), 0, "preview removed");
    assert_eq!(drops.get_untracked(), 0);
    assert_eq!(ended.get_untracked(), (false, true));
}

#[test]
fn right_button_cancels_drag() {
    use sequoia_input::PointerButton;
    let ended = Signal::new((false, false));
    let en = ended.clone();
    let h = Harness::new(
        Draggable::new(Text::new("src"), "x".to_string())
            .width(100.0)
            .height(40.0)
            .on_drag_end(move |end| en.set((end.dropped, end.cancelled))),
    );
    h.layout(400.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_move(60.0, 10.0);
    assert!(h.drag_session().is_some());
    h.pointer_down_at(60.0, 10.0, PointerButton::Right);
    h.tick();
    assert!(h.drag_session().is_none());
    assert_eq!(ended.get_untracked(), (false, true));
}

#[test]
fn preview_overlay_follows_cursor_minus_grab() {
    let h = Harness::new(
        Row::new().child(
            Draggable::new(Text::new("src"), "x".to_string())
                .width(100.0)
                .height(40.0)
                .preview(Text::new("ghost")),
        ),
    );
    h.layout(400.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_move(150.0, 20.0);
    h.paint();
    h.tick();
    let s = h.drag_session().expect("session");
    let pv = s.preview.expect("preview");
    let r = h.overlay_rect(pv).expect("preview rect after layout");
    assert_eq!(
        (r.left(), r.top()),
        (140.0, 10.0),
        "preview at current - grab (150-10, 20-10)"
    );

    h.pointer_move(170.0, 30.0);
    h.paint();
    h.tick();
    let r2 = h.overlay_rect(pv).expect("rect after move");
    assert_eq!((r2.left(), r2.top()), (160.0, 20.0));
    h.pointer_up(170.0, 30.0);
    h.tick();
    assert_eq!(h.overlay_count(), 0);
}

#[test]
fn highlight_mirror_tracks_target() {
    let over = Signal::new(false);
    let h = Harness::new(
        Row::new()
            .child(
                Draggable::new(Text::new("src"), "x".to_string())
                    .width(100.0)
                    .height(40.0),
            )
            .child(
                Droppable::new(Text::new("dst"), |_: String, _: DropPosition| {})
                    .highlight(over.clone())
                    .width(100.0)
                    .height(40.0),
            ),
    );
    h.layout(400.0, 200.0);
    h.paint();
    h.tick();
    assert!(!over.get_untracked());
    h.pointer_down(10.0, 10.0);
    h.pointer_move(150.0, 20.0);
    h.paint();
    h.tick();
    assert!(over.get_untracked(), "mirror over the target");
    h.pointer_move(350.0, 150.0);
    h.paint();
    h.tick();
    assert!(!over.get_untracked(), "mirror off outside the target");
    h.pointer_up(350.0, 150.0);
    h.paint();
    h.tick();
    assert!(!over.get_untracked());
}

#[test]
fn reorder_list_via_insertion_index() {
    use sequoia_view::move_item;
    let items = Signal::new(vec![0u32, 1, 2, 3]);
    let it = items.clone();
    let mut root = Droppable::new(
        Column::new()
            .child(
                Draggable::new(Text::new("r0"), 0usize)
                    .width(100.0)
                    .height(20.0),
            )
            .child(
                Draggable::new(Text::new("r1"), 1usize)
                    .width(100.0)
                    .height(20.0),
            )
            .child(
                Draggable::new(Text::new("r2"), 2usize)
                    .width(100.0)
                    .height(20.0),
            )
            .child(
                Draggable::new(Text::new("r3"), 3usize)
                    .width(100.0)
                    .height(20.0),
            ),
        move |from: usize, pos: DropPosition| {
            let slot = pos.insertion_index(4, Axis::Vertical);
            it.update(|v| move_item(v, from, slot));
        },
    )
    .width(100.0)
    .height(80.0);
    let _ = &mut root;
    let h = Harness::new(root);
    h.layout(400.0, 200.0);

    h.drag((50.0, 10.0), (50.0, 65.0));
    h.tick();
    assert_eq!(items.get_untracked(), vec![1, 2, 0, 3]);
}

#[test]
fn rebuild_does_not_kill_session() {
    let n = Signal::new(0u32);
    let nn = n.clone();
    let h = Harness::new(
        Row::new()
            .child(
                Draggable::new(
                    Text::with(move || format!("n={}", nn.get())),
                    "x".to_string(),
                )
                .width(100.0)
                .height(40.0),
            )
            .child(
                Droppable::new(Text::new("dst"), |_: String, _: DropPosition| {})
                    .width(100.0)
                    .height(40.0),
            ),
    );
    h.layout(400.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_move(60.0, 10.0);
    let id_before = h.drag_session().expect("sesja").id;

    n.set(1);
    h.tick();
    h.layout(400.0, 200.0);
    let s = h.drag_session().expect("session survives rebuild");
    assert_eq!(s.id, id_before);
    assert_eq!(s.current, Point::new(60.0, 10.0), "position kept");
    h.pointer_move(150.0, 20.0);
    h.pointer_up(150.0, 20.0);
    h.tick();
    assert!(h.drag_session().is_none());
}

#[test]
fn nested_draggable_inner_wins() {
    let got = Signal::new(String::new());
    let g = got.clone();
    let h = Harness::new(
        Row::new()
            .child(
                Draggable::new(
                    Draggable::new(Text::new("inner"), "inner".to_string())
                        .width(100.0)
                        .height(40.0),
                    "outer".to_string(),
                )
                .width(100.0)
                .height(40.0),
            )
            .child(
                Droppable::new(Text::new("dst"), move |id: String, _: DropPosition| {
                    g.set(id);
                })
                .width(100.0)
                .height(40.0),
            ),
    );
    h.layout(400.0, 200.0);
    h.drag((10.0, 10.0), (150.0, 20.0));
    h.tick();
    assert_eq!(got.get_untracked(), "inner");
}

#[test]
fn over_callback_sees_insertion_slot() {
    let slots = Signal::new(Vec::<usize>::new());
    let sl = slots.clone();
    let h = Harness::new(
        Row::new()
            .child(
                Draggable::new(Text::new("src"), "x".to_string())
                    .width(100.0)
                    .height(40.0),
            )
            .child(
                Droppable::new(Text::new("dst"), |_: String, _: DropPosition| {})
                    .on_over(move |pos| {
                        sl.update(|v| v.push(pos.insertion_index(4, Axis::Vertical)));
                    })
                    .width(100.0)
                    .height(40.0),
            ),
    );
    h.layout(400.0, 200.0);
    h.pointer_down(10.0, 10.0);
    h.pointer_move(150.0, 2.0);
    h.pointer_move(190.0, 30.0);
    h.pointer_up(190.0, 30.0);
    h.tick();
    let v = slots.get_untracked();
    assert!(v.contains(&0), "slot 0 widziany: {v:?}");
    assert!(v.contains(&3), "slot 3 widziany: {v:?}");
}
