//! Headless Framework Performance & Stress Test for SequoiaUI
//!
//! Tests core GUI framework performance without rendering window surfaces:
//! - Cold-start / First-frame layout & paint latency
//! - Reactivity tick and view reconciliation speed
//! - Heavy element tree layout and paint list generation throughput
//! - Node count & memory metrics

use std::hint::black_box;
use std::time::Instant;

use sequoia_core::{Color, Length, Rect, RenderList};
use sequoia_elements::{Button, Container};
use sequoia_reactive::Signal;
use sequoia_style::BoxStyle;
use sequoia_test::Harness;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

struct BenchmarkWidget {
    id: usize,
    counter: Signal<i32>,
    _toggle: Signal<bool>,
}

impl View for BenchmarkWidget {
    fn body(&self) -> impl View + 'static {
        let id = self.id;
        let counter = self.counter.clone();
        let _toggle = self._toggle.clone();

        Container::new()
            .width(Length::Fill(1.0))
            .padding(8.0)
            .style(
                BoxStyle::new()
                    .background(Color::rgb8(0x20, 0x22, 0x2A))
                    .radius(6.0),
            )
            .child(
                Row::new()
                    .gap(8.0)
                    .child(
                        Text::new(format!("Widget Item #{id:03}"))
                            .size(14.0)
                            .color(Color::WHITE),
                    )
                    .child(Spacer::flex(1.0))
                    .child(Text::with(move || format!("Val: {}", counter.get())).size(12.0))
                    .child(Button::labeled("Inc")),
            )
    }
}

struct HeavyApp {
    count: usize,
    counter: Signal<i32>,
    toggle: Signal<bool>,
}

impl View for HeavyApp {
    fn body(&self) -> impl View + 'static {
        let mut col = Column::new().gap(6.0).padding(12.0);
        for i in 0..self.count {
            col = col.child(BenchmarkWidget {
                id: i,
                counter: self.counter.clone(),
                _toggle: self.toggle.clone(),
            });
        }
        ScrollView::new(col)
    }
}

#[test]
fn test_framework_core_performance() {
    let count = 100; // 100 complex widgets in a scroll view
    let counter = Signal::new(0);
    let toggle = Signal::new(false);

    // 1. Cold-Start / First-Frame Latency Test
    let t_start = Instant::now();
    let harness = Harness::new(HeavyApp {
        count,
        counter: counter.clone(),
        toggle: toggle.clone(),
    });
    let rt = harness.runtime();
    let mut text_system = TextSystem::new();
    let mut render_list = RenderList::new();
    let bounds = Rect::from_xywh(0.0, 0.0, 1024.0, 768.0);

    rt.layout(bounds, &mut text_system);
    rt.paint(&mut render_list, &mut text_system);
    let cold_start_duration = t_start.elapsed();

    let element_count = harness.element_count();
    println!("[FRAMEWORK PERF] Cold-start & First-Frame latency: {cold_start_duration:?}");
    println!("[FRAMEWORK PERF] Total framework elements in tree: {element_count}");
    println!(
        "[FRAMEWORK PERF] Render items generated: {}",
        render_list.len()
    );

    assert!(
        cold_start_duration.as_secs_f64() < 2.0,
        "Cold-start layout and paint took too long: {cold_start_duration:?}"
    );
    assert!(
        element_count > count,
        "Element tree should contain nested components"
    );

    // 2. Reactivity & Tick Performance Test (100 ticks with state updates)
    let t_ticks = Instant::now();
    let iterations = 100;
    for i in 0..iterations {
        counter.set(i);
        if i % 10 == 0 {
            toggle.set(i % 20 == 0);
        }
        rt.tick();
        rt.layout(bounds, &mut text_system);
        render_list.clear();
        rt.paint(&mut render_list, &mut text_system);
        black_box(render_list.len());
    }
    let ticks_duration = t_ticks.elapsed();
    let avg_tick_ms = ticks_duration.as_secs_f64() * 1000.0 / iterations as f64;

    println!("[FRAMEWORK PERF] Completed {iterations} reactive ticks in {ticks_duration:?}");
    println!(
        "[FRAMEWORK PERF] Average time per tick (layout + reconcile + paint): {avg_tick_ms:.3} ms"
    );

    assert!(
        avg_tick_ms < 50.0,
        "Reactive tick average time ({avg_tick_ms:.3} ms) exceeded 50ms threshold"
    );
}
