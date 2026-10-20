#![allow(clippy::unwrap_used)]

use sequoia_core::Length;
use sequoia_elements::{Draggable, Droppable};
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

struct Probe {
    hit: Signal<bool>,
}

impl View for Probe {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(8.0),
            ..LayoutStyle::default()
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        self.hit.set(ctx.is_drag_target());
    }
}

#[test]
fn child_inside_drop_target_reports_is_drag_target() {
    let hit = Signal::new(false);
    let h = Harness::new(
        Row::new()
            .child(
                Draggable::new(Text::new("src"), 7u32)
                    .width(100.0)
                    .height(40.0),
            )
            .child(
                Droppable::new(Probe { hit: hit.clone() }, |_: u32, _: DropPosition| {})
                    .width(100.0)
                    .height(40.0),
            ),
    );
    h.layout(400.0, 200.0);
    h.paint();
    h.tick();
    assert!(!hit.get_untracked(), "no target outside session");

    h.pointer_down(10.0, 10.0);
    h.pointer_move(150.0, 20.0);
    h.paint();
    h.tick();
    assert!(hit.get_untracked(), "target content reports is_drag_target");

    h.pointer_move(350.0, 150.0);
    h.paint();
    h.tick();
    assert!(!hit.get_untracked(), "off target clears");
    h.pointer_up(350.0, 150.0);
    h.tick();
}

#[test]
fn gap_drop_reorders() {
    let order = Signal::new(vec![0u32, 1, 2]);
    let o = order.clone();

    let mut col = Column::new().width(100.0).height(200.0).gap(2.0);
    col = col.child(
        Droppable::new(Unit, {
            let o = o.clone();
            move |id: u32, _pos: DropPosition| {
                o.update(|v| {
                    let mut ids = v.clone();
                    let p = ids.iter().position(|&x| x == id).unwrap();
                    move_item(&mut ids, p, 0);
                    *v = ids;
                });
            }
        })
        .width(Length::Fill(1.0))
        .height(8.0),
    );
    for i in 0..3u32 {
        col = col.child(
            Draggable::new(Text::new(format!("k{i}")), i)
                .width(100.0)
                .height(40.0),
        );
        let o2 = o.clone();
        let gap_index = (i + 1) as usize;
        col = col.child(
            Droppable::new(Unit, move |id: u32, _pos: DropPosition| {
                o2.update(|v| {
                    let mut ids = v.clone();
                    let p = ids.iter().position(|&x| x == id).unwrap();
                    move_item(&mut ids, p, gap_index);
                    *v = ids;
                });
            })
            .width(Length::Fill(1.0))
            .height(8.0),
        );
    }
    let h = Harness::new(col);
    h.layout(400.0, 400.0);

    h.drag((50.0, 30.0), (50.0, 160.0));
    h.tick();
    assert_eq!(
        order.get_untracked(),
        vec![1, 2, 0],
        "card 0 after card 2 (gap 3 in old coords -> [1,2,0])"
    );
}

#[test]
fn card_drop_maps_midpoint_to_gap() {
    let order = Signal::new(vec![0u32, 1, 2]);
    let o = order.clone();
    let mut col = Column::new().width(100.0).height(200.0);
    for i in 0..3u32 {
        let o2 = o.clone();
        col = col.child(
            Droppable::new(
                Draggable::new(Text::new(format!("k{i}")), i)
                    .width(Length::Fill(1.0))
                    .height(Length::Fill(1.0)),
                move |id: u32, pos: DropPosition| {
                    let gap = if pos.before_midpoint(Axis::Vertical) {
                        i as usize
                    } else {
                        i as usize + 1
                    };
                    o2.update(|v| {
                        let mut ids = v.clone();
                        let p = ids.iter().position(|&x| x == id).unwrap();
                        move_item(&mut ids, p, gap);
                        *v = ids;
                    });
                },
            )
            .width(100.0)
            .height(40.0),
        );
    }
    let h = Harness::new(col);
    h.layout(400.0, 400.0);

    h.drag((50.0, 20.0), (50.0, 110.0));
    h.tick();
    assert_eq!(order.get_untracked(), vec![1, 2, 0]);

    h.drag((50.0, 100.0), (50.0, 10.0));
    h.tick();
    assert_eq!(order.get_untracked(), vec![2, 1, 0]);
}
