#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_assets::AssetRegistry;
use sequoia_core::prelude::*;
use sequoia_reactive::Environment;
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;

const DOT: &str = r#"<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"/></svg>"#;

fn runtime_with_registry(icon: &str, svg: &str) -> Runtime {
    let assets = AssetRegistry::new();
    assets.register_icon(icon, svg).unwrap();
    let env = Environment::new();
    env.provide(assets);
    let rt = Runtime::new();
    rt.set_environment(env);
    rt
}

#[test]
fn icon_without_registry_paints_nothing() {
    let rt = Runtime::new();
    rt.mount(Icon::new("dot").size(10.0));
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert!(list.is_empty());
}

#[test]
fn icon_without_registered_name_paints_nothing() {
    let rt = runtime_with_registry("dot", DOT);
    rt.mount(Icon::new("missing").size(10.0));
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert!(list.is_empty());
}

#[test]
fn icon_paints_one_sprite_with_size_and_color() {
    let rt = runtime_with_registry("dot", DOT);
    rt.mount(
        Column::new()
            .child(Icon::new("dot").size(12.0).color(Color::RED))
            .child(Icon::new("dot").size(12.0).color(Color::RED)),
    );
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);

    assert_eq!(list.len(), 2, "two icons = two sprites");
    for item in list.items() {
        let RenderItem::Text(t) = item else {
            panic!("expected a sprite")
        };
        assert_eq!(t.color, Color::RED);
        assert_eq!(t.glyphs.len(), 1);
        let g = &t.glyphs[0];
        assert_eq!(
            g.rect.size,
            Size::new(12.0, 12.0),
            "target rect = icon size"
        );
        assert!(g.uv[2] > g.uv[0] && g.uv[3] > g.uv[1], "UV correct");
    }
}

#[test]
fn icon_is_centered_in_its_rect() {
    let assets = AssetRegistry::new();
    assets.register_icon("dot", DOT).unwrap();
    let env = Environment::new();
    env.provide(assets);
    let rt = Runtime::new();
    rt.set_environment(env);
    rt.mount(
        Stack::new()
            .width(40.0)
            .height(40.0)
            .child(Icon::new("dot").size(10.0)),
    );
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    let item = list
        .items()
        .iter()
        .find(|i| matches!(i, RenderItem::Text(_)))
        .unwrap();
    let RenderItem::Text(t) = item else {
        unreachable!()
    };
    let left = t.glyphs[0].rect.left();
    let top = t.glyphs[0].rect.top();
    assert!(
        ((left - 15.0).abs() < 0.6),
        "icon centered horizontally, left={left}"
    );
    assert!(
        ((top - 15.0).abs() < 0.6),
        "icon centered vertically, top={top}"
    );
}

#[test]
fn icon_of_missing_viewbox_data_is_ignored() {
    let assets = AssetRegistry::new();
    assert!(
        assets
            .register_icon("zepsuta", "<svg><path d='M0 0'/></svg>")
            .is_err()
    );
    let env = Environment::new();
    env.provide(assets);
    let rt = Runtime::new();
    rt.set_environment(env);
    rt.mount(Icon::new("zepsuta"));
    let mut text = TextSystem::new();
    rt.layout(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &mut text);
    let mut list = RenderList::new();
    rt.paint(&mut list, &mut text);
    assert!(list.is_empty());
}
