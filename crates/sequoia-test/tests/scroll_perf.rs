#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::hint::black_box;
use std::time::Instant;

use sequoia_core::{Color, Length, Point, Rect, RenderList, Vec2};
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

const W: f32 = 800.0;
const H: f32 = 600.0;

const STEP: f32 = 5.0;
const FRAMES: usize = 600;

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

struct Demo {
    n: usize,
}

impl View for Demo {
    fn body(&self) -> impl View + 'static {
        let mut col = Column::new();
        for _ in 0..self.n {
            col = col.child(Bar { h: 40.0 });
        }
        ScrollView::new(col)
    }
}

struct FrameStats {
    avg_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    avg_layout_ms: f64,
    avg_paint_ms: f64,
    quads: usize,
}

fn scroll_el_of(h: &Harness) -> ElementId {
    let root = h.root().expect("root");
    h.children(root)[0]
}

fn bench(n: usize, frames: usize, step: f32) -> FrameStats {
    let h = Harness::new(Demo { n });
    let rt = h.runtime();
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, W, H);

    rt.layout(bounds, &mut text);
    list.clear();
    rt.paint(&mut list, &mut text);
    black_box(list.len());

    let scroll_el = scroll_el_of(&h);
    let scroll = h.view::<ScrollView>(scroll_el).expect("ScrollView");
    let max = scroll.max_offset(&rt, scroll_el);

    assert!(
        max > 0.0,
        "n={n} does not require scrolling (content <= viewport), use a larger n"
    );

    for _ in 0..10 {
        scroll.scroll_by(&rt, scroll_el, Vec2::new(0.0, step));
        rt.layout(bounds, &mut text);
        list.clear();
        rt.paint(&mut list, &mut text);
    }
    scroll.scroll_to_top(&rt, scroll_el);
    rt.layout(bounds, &mut text);

    let mut dir = 1.0f32;
    let mut dts = Vec::with_capacity(frames);
    let mut layout_acc = 0.0f64;
    let mut paint_acc = 0.0f64;
    let mut quads = 0usize;

    for _ in 0..frames {
        let off = ScrollView::offset(&scroll, &rt, scroll_el);
        if off >= max {
            dir = -1.0;
        } else if off <= 0.0 {
            dir = 1.0;
        }
        let t0 = Instant::now();
        scroll.scroll_by(&rt, scroll_el, Vec2::new(0.0, dir * step));

        let t_layout = Instant::now();
        rt.layout(bounds, &mut text);
        let dt_layout = t_layout.elapsed();

        let t_paint = Instant::now();
        list.clear();
        rt.paint(&mut list, &mut text);
        let dt_paint = t_paint.elapsed();

        quads = list.len();
        black_box(quads);
        dts.push(t0.elapsed().as_secs_f64() * 1000.0);
        layout_acc += dt_layout.as_secs_f64() * 1000.0;
        paint_acc += dt_paint.as_secs_f64() * 1000.0;
    }

    dts.sort_by(|a, b| a.total_cmp(b));
    let avg_ms = dts.iter().sum::<f64>() / dts.len() as f64;
    FrameStats {
        avg_ms,
        p50_ms: dts[dts.len() / 2],
        p95_ms: dts[(dts.len() as f64 * 0.95) as usize],
        max_ms: dts[dts.len() - 1],
        avg_layout_ms: layout_acc / frames as f64,
        avg_paint_ms: paint_acc / frames as f64,
        quads,
    }
}

fn report(label: &str, n: usize, s: &FrameStats) {
    let cpus = std::thread::available_parallelism()
        .map(|v| v.get() as f64)
        .unwrap_or(1.0);
    let one_core_pct = s.avg_ms / 16.6667 * 100.0;
    let machine_pct = one_core_pct / cpus;
    let fps = 1000.0 / s.avg_ms.max(1e-9);
    println!(
        "[scroll_perf] {}: n={} itemow | quads/klatke={} | avg={:.3}ms (layout {:.3} + paint {:.3}) | p50={:.3}ms p95={:.3}ms max={:.3}ms | ~{:.0} FPS teoretycznie | CPU ok. {:.1}% jednego rdzenia przy 60 FPS ({:.2}% maszyny {:.0} CPU)",
        label,
        n,
        s.quads,
        s.avg_ms,
        s.avg_layout_ms,
        s.avg_paint_ms,
        s.p50_ms,
        s.p95_ms,
        s.max_ms,
        fps,
        one_core_pct,
        machine_pct,
        cpus,
    );
}

#[test]
fn continuous_scroll_perf() {
    let small = bench(20, FRAMES, STEP);
    report("mala tresc (800px)", 20, &small);

    let medium = bench(200, FRAMES, STEP);
    report("srednia tresc (8000px)", 200, &medium);

    let large = bench(1000, FRAMES, STEP);
    report("duza tresc (40000px)", 1000, &large);

    assert!(
        small.avg_ms < 2.0,
        "small scroll too slow: avg={:.3}ms",
        small.avg_ms
    );
    assert!(
        small.p95_ms < 5.0,
        "small scroll has tails: p95={:.3}ms",
        small.p95_ms
    );
}

#[test]
fn continuous_scroll_wheel_event_cost() {
    let h = Harness::new(Demo { n: 50 });
    let rt = h.runtime();
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, W, H);
    let pos = Point::new(W / 2.0, H / 2.0);

    rt.layout(bounds, &mut text);
    list.clear();
    rt.paint(&mut list, &mut text);

    let frames = 300usize;
    let mut dts = Vec::with_capacity(frames);
    let mut down = true;

    for _ in 0..10 {
        rt.wheel(pos, Vec2::new(0.0, -STEP), Modifiers::NONE, &mut text);
        rt.layout(bounds, &mut text);
        list.clear();
        rt.paint(&mut list, &mut text);
    }
    let t_total = Instant::now();
    for i in 0..frames {
        let t0 = Instant::now();
        let dy = if down { -STEP } else { STEP };
        rt.wheel(pos, Vec2::new(0.0, dy), Modifiers::NONE, &mut text);

        rt.layout(bounds, &mut text);
        list.clear();
        rt.paint(&mut list, &mut text);
        black_box(list.len());
        dts.push(t0.elapsed().as_secs_f64() * 1000.0);
        if i % 100 == 99 {
            down = !down;
        }
    }
    let total_ms = t_total.elapsed().as_secs_f64() * 1000.0;
    dts.sort_by(|a, b| a.total_cmp(b));
    let avg = dts.iter().sum::<f64>() / dts.len() as f64;
    let p95 = dts[(dts.len() as f64 * 0.95) as usize];
    let cpus = std::thread::available_parallelism()
        .map(|v| v.get() as f64)
        .unwrap_or(1.0);
    println!(
        "[scroll_perf] wheel(50 wierszy, 300 klatek): avg={:.3}ms p95={:.3}ms total={:.0}ms | CPU ok. {:.1}% rdzenia @60FPS ({:.2}% maszyny {:.0} CPU)",
        avg,
        p95,
        total_ms,
        avg / 16.6667 * 100.0,
        avg / 16.6667 * 100.0 / cpus,
        cpus,
    );
    assert!(avg < 3.0, "wheel+layout+paint za wolne: avg={avg:.3}ms");
}
