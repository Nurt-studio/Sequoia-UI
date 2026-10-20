use std::time::{Duration, Instant};

use sequoia_core::{Color, Length, Rect, RenderList, Vec2};
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

const W: f32 = 800.0;
const H: f32 = 600.0;
const STEP: f32 = 5.0;
const FRAME: Duration = Duration::from_nanos(16_666_667);
const SECS: u64 = 20;

struct Bar {
    h: f32,
}

impl View for Bar {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(self.h),
            ..LayoutStyle::default()
        }
    }
    fn paint_self(&self, ctx: &mut PaintCtx) {
        ctx.quad(ctx.rect, Color::rgb8(0x33, 0x99, 0xcc), 0.0);
    }
}

struct Demo;

impl View for Demo {
    fn body(&self) -> impl View + 'static {
        let mut col = Column::new();
        for _ in 0..1000 {
            col = col.child(Bar { h: 40.0 });
        }
        ScrollView::new(col)
    }
}

fn main() {
    let h = Harness::new(Demo);
    let rt = h.runtime();
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, W, H);

    rt.layout(bounds, &mut text);
    let root = h.root().expect("root");
    let scroll_el = h.children(root)[0];
    let scroll = h.view::<ScrollView>(scroll_el).expect("ScrollView");
    let max = scroll.max_offset(&rt, scroll_el);

    let end = Instant::now() + Duration::from_secs(SECS);
    let mut dir = 1.0f32;
    let mut frames = 0u64;
    let mut busy_acc = Duration::ZERO;
    let mut next = Instant::now();

    println!(
        "[scroll_stress] start: 1000 wierszy, {}s przy ~60 FPS (pid {})",
        SECS,
        std::process::id()
    );
    while Instant::now() < end {
        let frame_start = Instant::now();
        let off = ScrollView::offset(&scroll, &rt, scroll_el);
        if off >= max {
            dir = -1.0;
        } else if off <= 0.0 {
            dir = 1.0;
        }
        scroll.scroll_by(&rt, scroll_el, Vec2::new(0.0, dir * STEP));
        rt.layout(bounds, &mut text);
        list.clear();
        rt.paint(&mut list, &mut text);
        std::hint::black_box(list.len());

        busy_acc += frame_start.elapsed();
        frames += 1;
        next += FRAME;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else {
            next = now;
        }
    }
    let busy_ms = busy_acc.as_secs_f64() * 1000.0 / frames as f64;
    println!(
        "[scroll_stress] stop: {frames} klatek, busy avg={busy_ms:.3}ms/klatke (~{:.1}% rdzenia)",
        busy_ms / 16.6667 * 100.0
    );
}
