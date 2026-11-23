#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::any::Any;
use std::rc::Rc;

use sequoia_core::Length;
use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;
use sequoia_view::{AnyView, LayoutStyle, Unit, View, when};

#[derive(Clone)]
struct SurfaceStub;

impl View for SurfaceStub {
    fn body(&self) -> impl View + 'static {
        Unit
    }
    fn is_leaf(&self) -> bool {
        true
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }
    fn surface_target(&self) -> Option<Rc<dyn Any>> {
        Some(Rc::new(()))
    }
}

fn app(body: &str) -> String {
    format!(
        "app R {{\n  title: \"R\"\n  size: (800, 600)\n  prop flag: Bool\n  window Main {{\n{body}\n  }}\n}}\n"
    )
}

fn load(body: &str, flag: &Signal<bool>) -> Harness {
    let bindings = HostBindings::new()
        .prop_bool("flag", flag.clone())
        .prop_view("inner", AnyView::new(SurfaceStub))
        .prop_view("overlays", AnyView::new(SurfaceStub))
        .prop_view("left", AnyView::new(SurfaceStub))
        .prop_view("right", AnyView::new(SurfaceStub));
    let loaded = SeqLoader::load_str_with("Repro.seq", &app(body), &bindings).unwrap();
    Harness::new(loaded.view)
}

#[test]
fn surface_inside_if_in_row_keeps_its_layout_rect() {
    let body = "    Stack(flex: 1) {\n      Row(width: px(600), height: px(200), at: bottom_right) {\n        Panel(width: px(100), height: px(200))\n        If(when: flag) {\n          GpuSurface(name: \"inner\", flex: 1)\n        }\n      }\n      GpuSurface(name: \"overlays\", at: fill)\n    }";
    let flag = Signal::new(false);
    let h = load(body, &flag);
    h.layout(800.0, 600.0);
    let _ = h.paint();
    flag.set(true);
    h.tick();
    h.layout(800.0, 600.0);
    let _ = h.paint();

    let draws = h.surface_draws();
    assert_eq!(draws.len(), 2, "both surfaces must be recorded");
    let inner = draws
        .iter()
        .min_by_key(|d| d.rect.size.width as u32)
        .expect("inner surface");
    assert_eq!(inner.rect.left(), 300.0);
    assert_eq!(inner.rect.top(), 400.0);
    assert_eq!(inner.rect.size.width, 500.0);
}

#[test]
fn two_fixed_panels_and_an_if_in_row_all_remain_visible() {
    let body = "    Column(width: px(800), height: px(600)) {\n      Row(width: px(800), height: px(200)) {\n        GpuSurface(name: \"left\", width: px(200))\n        Panel(width: px(200), height: px(200), background: #FF0000)\n        If(when: flag) {\n          GpuSurface(name: \"right\", width: px(200))\n        }\n      }\n    }";
    let flag = Signal::new(true);
    let h = load(body, &flag);
    h.layout(800.0, 600.0);
    let _ = h.paint();

    let draws = h.surface_draws();
    for (i, d) in draws.iter().enumerate() {
        println!("panel draw {i}: {:?}", d.rect);
    }
    assert_eq!(
        draws.len(),
        2,
        "left and right surfaces must both be painted"
    );
    assert_eq!(draws[0].rect.left(), 0.0);
    assert!(
        draws[1].rect.left() >= 200.0,
        "right surface must clear the left panel"
    );
}

#[test]
fn when_preserves_the_host_layout_style() {
    let flag = Signal::new(true);
    let wrapped = when(flag, SurfaceStub);
    let style = wrapped.layout_style();
    assert_eq!(style.width, Length::Fill(1.0));
    assert_eq!(style.height, Length::Fill(1.0));

    let hidden = Signal::new(false);
    let empty = when(hidden, SurfaceStub);
    let style = empty.layout_style();
    assert_eq!(style.width, Length::Fit);
    assert_eq!(style.height, Length::Fit);
}
