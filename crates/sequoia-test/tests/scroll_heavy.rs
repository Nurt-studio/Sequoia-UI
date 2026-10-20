#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::hint::black_box;
use std::time::Instant;

use sequoia_core::{Color, Length, Rect, RenderItem, RenderList, Vec2};
use sequoia_elements::{Avatar, Badge, Button, Container, Slider, Switch};
use sequoia_reactive::Signal;
use sequoia_style::{BoxStyle, ShadowSpec};
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

const W: f32 = 800.0;
const H: f32 = 600.0;
const STEP: f32 = 9.0;
const N: usize = 300;
const FRAMES: usize = 200;

const LOREM: &str = "Sequoia renders this paragraph in every list row: wrapping, shaping \
    and measure must run each frame, and the text cache takes a hit from live counters \
    nearby. The longer the text, the more expensive the layout — this is a deliberate worst case for the dashboard.";

struct HeavyRow {
    i: usize,
    tick: Signal<u64>,
    level: Signal<f32>,
    on: Signal<bool>,
}

impl View for HeavyRow {
    fn body(&self) -> impl View + 'static {
        let i = self.i;
        let tick = self.tick.clone();
        let live =
            Text::with(move || format!("wiersz {i:04} • tick {} • {:.1} fps", tick.get(), 60.0))
                .size(12.0)
                .color(Color::rgb8(0x99, 0x99, 0x99));

        let title = Text::new(format!("Item {i:04} - composite dashboard tab")).size(15.0);
        let sub = Text::new("avatar • badge • button • slider • blur in header")
            .size(12.0)
            .color(Color::rgb8(0x77, 0x77, 0x77));
        let body_text = Text::new(LOREM).size(12.0);

        Container::new()
            .width(Length::Fill(1.0))
            .style(
                BoxStyle::new()
                    .background(Color::rgb8(0x24, 0x24, 0x26))
                    .radius(12.0)
                    .border(1.0, Color::rgb8(0x44, 0x44, 0x44))
                    .shadow(ShadowSpec::new(Color::BLACK, 12.0)),
            )
            .padding_xy(12.0, 10.0)
            .child(
                Column::new()
                    .gap(8.0)
                    .child(
                        Row::new()
                            .gap(10.0)
                            .child(Avatar::named(format!("Osoba {i}")).size(40.0))
                            .child(Column::new().gap(2.0).child(title).child(sub))
                            .child(Badge::count(i % 100))
                            .child(Button::labeled("Open")),
                    )
                    .child(live)
                    .child(body_text)
                    .child(
                        Row::new()
                            .gap(10.0)
                            .child(Switch::new(self.on.clone()))
                            .child(Slider::new(self.level.clone()).min(0.0).max(100.0)),
                    ),
            )
    }
}

struct Heavy {
    n: usize,
    tick: Signal<u64>,
    level: Signal<f32>,
    on: Signal<bool>,
}

impl View for Heavy {
    fn body(&self) -> impl View + 'static {
        let tick = self.tick.clone();
        let n = self.n;
        let stats =
            Text::with(move || format!("DASHBOARD • {n} kart • tick {}", tick.get())).size(16.0);
        let header = BackdropBlur::blurred(
            Container::new()
                .background(Color::rgb8(0x1c, 0x1c, 0x1e))
                .padding_xy(16.0, 10.0)
                .child(stats),
            12.0,
        );

        let mut col = Column::new().gap(10.0).padding_xy(12.0, 10.0);
        for i in 0..self.n {
            col = col.child(HeavyRow {
                i,
                tick: self.tick.clone(),
                level: self.level.clone(),
                on: self.on.clone(),
            });
        }
        Column::new()
            .child(header)
            .child_flex(ScrollView::new(col), 1.0)
    }
}

#[test]
fn heavy_dashboard_virtualization_speedup() {
    let full_avg = bench_full();
    let virt_avg = bench_virt();
    let speedup = full_avg / virt_avg.max(1e-9);
    println!(
        "[scroll_heavy] SPEEDUP: full {full_avg:.1}ms vs virt {virt_avg:.1}ms = {speedup:.1}x"
    );
    assert!(
        speedup >= 5.0,
        "virtualization should give at least 5x (there is {speedup:.1}x)"
    );
    assert!(
        virt_avg < 25.0,
        "virtualization too slow even absolutely: avg={virt_avg:.3}ms"
    );
}

fn bench_full() -> f64 {
    let tick = Signal::new(0u64);
    let level = Signal::new(50.0f32);
    let on = Signal::new(true);

    let h = Harness::new(Heavy {
        n: N,
        tick: tick.clone(),
        level: level.clone(),
        on: on.clone(),
    });
    let rt = h.runtime();
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, W, H);

    rt.layout(bounds, &mut text);
    list.clear();
    rt.paint(&mut list, &mut text);
    black_box(list.len());

    let root = h.root().expect("root");

    let body = h.children(root)[0];
    let kids = h.children(body);
    let scroll_el = kids[1];
    let scroll = h.view::<ScrollView>(scroll_el).expect("ScrollView");
    let max = scroll.max_offset(&rt, scroll_el);
    assert!(max > 0.0, "content must require scrolling");

    let elements = h.element_count();
    println!("[scroll_heavy] elementy w drzewie: {elements} ({N} kart)");

    for _ in 0..5 {
        tick.set(tick.get_untracked() + 1);
        rt.tick();
        rt.layout(bounds, &mut text);
        list.clear();
        rt.paint(&mut list, &mut text);
    }
    scroll.scroll_to_top(&rt, scroll_el);

    let mut dir = 1.0f32;
    let mut dts = Vec::with_capacity(FRAMES);
    let mut layout_acc = 0.0f64;
    let mut paint_acc = 0.0f64;
    let mut rebuild_acc = 0.0f64;
    let (mut quads, mut texts, mut glyphs, mut shadows, mut backdrops) = (0, 0, 0, 0, 0);

    for f in 0..FRAMES {
        let off = sequoia_view::ScrollView::offset(&scroll, &rt, scroll_el);
        if off >= max {
            dir = -1.0;
        } else if off <= 0.0 {
            dir = 1.0;
        }
        let t0 = Instant::now();

        tick.set(f as u64);
        level.set(50.0 + 50.0 * ((f as f32 * 0.2).sin()));
        if f % 60 == 59 {
            on.set(!on.get_untracked());
        }
        scroll.scroll_by(&rt, scroll_el, Vec2::new(0.0, dir * STEP));

        let t_rb = Instant::now();
        rt.tick();
        let dt_rb = t_rb.elapsed();

        let t_lo = Instant::now();
        rt.layout(bounds, &mut text);
        let dt_lo = t_lo.elapsed();

        let t_pa = Instant::now();
        list.clear();
        rt.paint(&mut list, &mut text);
        let dt_pa = t_pa.elapsed();

        quads = 0;
        texts = 0;
        glyphs = 0;
        shadows = 0;
        for item in list.items() {
            match item {
                RenderItem::Quad(_) => quads += 1,
                RenderItem::Text(t) => {
                    texts += 1;
                    glyphs += t.glyphs.len();
                }
                RenderItem::Shadow(_) => shadows += 1,
                _ => {}
            }
        }
        backdrops = list.backdrops().len();
        black_box((quads, glyphs));

        dts.push(t0.elapsed().as_secs_f64() * 1000.0);
        rebuild_acc += dt_rb.as_secs_f64() * 1000.0;
        layout_acc += dt_lo.as_secs_f64() * 1000.0;
        paint_acc += dt_pa.as_secs_f64() * 1000.0;
    }

    dts.sort_by(|a, b| a.total_cmp(b));
    let avg = dts.iter().sum::<f64>() / dts.len() as f64;
    let p50 = dts[dts.len() / 2];
    let p95 = dts[(dts.len() as f64 * 0.95) as usize];
    let mx = dts[dts.len() - 1];
    let cpus = std::thread::available_parallelism()
        .map(|v| v.get() as f64)
        .unwrap_or(1.0);
    println!(
        "[scroll_heavy] {N} kart, {FRAMES} klatek: quads={quads} texts={texts} glyphs={glyphs} shadows={shadows} backdrops={backdrops}"
    );
    println!(
        "[scroll_heavy] avg={avg:.3}ms (rebuild {:.3} + layout {:.3} + paint {:.3}) | p50={p50:.3}ms p95={p95:.3}ms max={mx:.3}ms | ~{:.0} FPS | CPU ok. {:.1}% rdzenia @60FPS ({:.2}% maszyny {cpus:.0} CPU)",
        rebuild_acc / FRAMES as f64,
        layout_acc / FRAMES as f64,
        paint_acc / FRAMES as f64,
        1000.0 / avg.max(1e-9),
        avg / 16.6667 * 100.0,
        avg / 16.6667 * 100.0 / cpus,
    );

    assert!(
        avg < 250.0,
        "pathological max-load regression: avg={avg:.3}ms (was ~60-100ms)"
    );
    avg
}

struct HeavyVirtual {
    tick: Signal<u64>,
    level: Signal<f32>,
    on: Signal<bool>,
    item_h: f32,
}

impl View for HeavyVirtual {
    fn body(&self) -> impl View + 'static {
        let tick = self.tick.clone();
        let level = self.level.clone();
        let on = self.on.clone();
        let header_text = Text::new("DASHBOARD wirtualizowany — to samo 300 kart").size(16.0);
        let list = VirtualList::new(move || (0..N as u32).collect::<Vec<u32>>())
            .key(|id: &u32| *id as u64)
            .item(move |id: &u32| HeavyRow {
                i: *id as usize,
                tick: tick.clone(),
                level: level.clone(),
                on: on.clone(),
            })
            .item_height(self.item_h)
            .overscan(4);
        Column::new()
            .child(
                Container::new()
                    .width(Length::Fill(1.0))
                    .background(Color::rgb8(0x1c, 0x1c, 0x1e))
                    .padding_xy(16.0, 10.0)
                    .child(header_text),
            )
            .child_flex(list, 1.0)
    }
}

fn probe_card_height() -> f32 {
    let h = Harness::new(HeavyRow {
        i: 0,
        tick: Signal::new(0u64),
        level: Signal::new(50.0f32),
        on: Signal::new(true),
    });
    h.layout(W, H);
    h.rect(h.root().unwrap())
        .map(|r| r.size.height)
        .unwrap_or(0.0)
}

fn bench_virt() -> f64 {
    let item_h = probe_card_height();
    assert!(
        item_h > 50.0 && item_h < 2000.0,
        "nonsense card height: {item_h}"
    );
    println!("[scroll_heavy] VIRT: slot karty = {item_h:.1}px");

    let tick = Signal::new(0u64);
    let level = Signal::new(50.0f32);
    let on = Signal::new(true);

    let h = Harness::new(HeavyVirtual {
        tick: tick.clone(),
        level: level.clone(),
        on: on.clone(),
        item_h,
    });
    let rt = h.runtime();
    let mut text = TextSystem::new();
    let mut list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, W, H);

    rt.layout(bounds, &mut text);
    list.clear();
    rt.paint(&mut list, &mut text);
    black_box(list.len());

    let body = h.children(h.root().unwrap())[0];
    let vl_el = h.children(body)[1];
    let vl = h.view::<VirtualList<u32>>(vl_el).expect("VirtualList");
    let max = vl.max_offset();
    assert!(max > 0.0, "content must require scrolling");

    vl.scroll_to_bottom();
    h.tick();
    h.layout(W, H);
    let first_key = h
        .children(vl_el)
        .into_iter()
        .filter_map(|id| h.key_of(id))
        .min()
        .expect("window keys");
    assert!(first_key > 200, "bottom of list: first_key={first_key}");
    vl.scroll_to_top();
    h.tick();
    h.layout(W, H);

    println!(
        "[scroll_heavy] VIRT: elementy w drzewie: {}",
        h.element_count()
    );

    for _ in 0..5 {
        tick.set(tick.get_untracked() + 1);
        rt.tick();
        rt.layout(bounds, &mut text);
        list.clear();
        rt.paint(&mut list, &mut text);
    }
    vl.scroll_to_top();
    rt.tick();
    rt.layout(bounds, &mut text);

    let mut dts = Vec::with_capacity(FRAMES);
    let mut layout_acc = 0.0f64;
    let mut paint_acc = 0.0f64;
    let mut rebuild_acc = 0.0f64;
    let (mut quads, mut texts, mut glyphs, mut shadows, mut backdrops) = (0, 0, 0, 0, 0);

    for f in 0..FRAMES {
        if VirtualList::offset(&vl) >= max - 0.5 {
            vl.scroll_to_top();
        } else {
            vl.scroll_by(STEP);
        }
        let t0 = Instant::now();
        tick.set(f as u64);
        level.set(50.0 + 50.0 * ((f as f32 * 0.2).sin()));
        if f % 60 == 59 {
            on.set(!on.get_untracked());
        }

        let t_rb = Instant::now();
        rt.tick();
        let dt_rb = t_rb.elapsed();

        let t_lo = Instant::now();
        rt.layout(bounds, &mut text);
        let dt_lo = t_lo.elapsed();

        let t_pa = Instant::now();
        list.clear();
        rt.paint(&mut list, &mut text);
        let dt_pa = t_pa.elapsed();

        quads = 0;
        texts = 0;
        glyphs = 0;
        shadows = 0;
        for item in list.items() {
            match item {
                RenderItem::Quad(_) => quads += 1,
                RenderItem::Text(t) => {
                    texts += 1;
                    glyphs += t.glyphs.len();
                }
                RenderItem::Shadow(_) => shadows += 1,
                _ => {}
            }
        }
        backdrops = list.backdrops().len();
        black_box((quads, glyphs));

        dts.push(t0.elapsed().as_secs_f64() * 1000.0);
        rebuild_acc += dt_rb.as_secs_f64() * 1000.0;
        layout_acc += dt_lo.as_secs_f64() * 1000.0;
        paint_acc += dt_pa.as_secs_f64() * 1000.0;
    }

    dts.sort_by(|a, b| a.total_cmp(b));
    let avg = dts.iter().sum::<f64>() / dts.len() as f64;
    let p50 = dts[dts.len() / 2];
    let p95 = dts[(dts.len() as f64 * 0.95) as usize];
    let mx = dts[dts.len() - 1];
    let cpus = std::thread::available_parallelism()
        .map(|v| v.get() as f64)
        .unwrap_or(1.0);
    println!(
        "[scroll_heavy] VIRT {N} kart, {FRAMES} klatek: quads={quads} texts={texts} glyphs={glyphs} shadows={shadows} backdrops={backdrops}"
    );
    println!(
        "[scroll_heavy] VIRT avg={avg:.3}ms (rebuild {:.3} + layout {:.3} + paint {:.3}) | p50={p50:.3}ms p95={p95:.3}ms max={mx:.3}ms | ~{:.0} FPS | CPU ok. {:.1}% rdzenia @60FPS ({:.2}% maszyny {cpus:.0} CPU)",
        rebuild_acc / FRAMES as f64,
        layout_acc / FRAMES as f64,
        paint_acc / FRAMES as f64,
        1000.0 / avg.max(1e-9),
        avg / 16.6667 * 100.0,
        avg / 16.6667 * 100.0 / cpus,
    );

    avg
}
