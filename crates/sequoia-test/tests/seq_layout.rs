#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_seq::{SeqLoader, check, parse};
use sequoia_test::Harness;

fn app_with(body: &str) -> String {
    format!("app A {{\n  title: \"A\"\n  size: (400, 300)\n  window M {{\n{body}\n  }}\n}}\n")
}

fn paints(src: &str) {
    let loaded = SeqLoader::load_str("Main.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    assert!(!h.paint().is_empty(), "drzewo maluje: {src}");
}

#[test]
fn stack_layers_paint() {
    let src = app_with(
        "    Stack(padding: 8, background: theme.bg) {\n      Text(\"a\", at: top_left)\n      Text(\"b\", at: bottom_right, offset: (4, 6), z: 2)\n    }",
    );
    paints(&src);
}

#[test]
fn stack_fill_and_center_positions() {
    let src = app_with(
        "    Stack {\n      Text(\"a\", at: fill)\n      Text(\"b\", at: center, z: 1)\n    }",
    );
    paints(&src);
}

#[test]
fn bad_stack_pos_reports_line() {
    let src = app_with("    Stack {\n      Text(\"a\", at: nowhere)\n    }");
    let app = parse("Main.seq", &src).unwrap();
    check("Main.seq", &app).unwrap();
    let err = match SeqLoader::load_str("Main.seq", &src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("at to"), "{err}");
}

#[test]
fn grid_tracks_and_cells_paint() {
    let src = app_with(
        "    Grid(columns: [px(100), fill(1)], rows: [px(40), fill(1)], gap: 8) {\n      Text(\"a\", col: 0, row: 0)\n      Text(\"b\", col: 1, row: 1, col_span: 1, row_span: 1)\n    }",
    );
    paints(&src);
}

#[test]
fn grid_gaps_and_flow() {
    let src = app_with(
        "    Grid(columns: [fill(1), fill(2)], column_gap: 4, row_gap: 6, auto_flow: column, align: center, justify: center) {\n      Text(\"a\")\n      Text(\"b\")\n    }",
    );
    paints(&src);
}

#[test]
fn wrap_axis_and_gaps_paint() {
    let src = app_with(
        "    Wrap(axis: horizontal, gap: 4, column_gap: 4, row_gap: 6, align: center) {\n      Text(\"a\")\n      Text(\"b\")\n    }",
    );
    paints(&src);
}

#[test]
fn masonry_columns_and_gaps_paint() {
    let src = app_with(
        "    Masonry(columns: 3, gap: 10, width: px(320)) {\n      Panel(height: px(40))\n      Panel(height: px(60))\n      Panel(height: px(20))\n      Panel(height: px(30))\n    }",
    );
    let loaded = SeqLoader::load_str("Main.seq", &src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);

    let masonry = h.children(h.root().unwrap())[0];
    let kids = h.children(masonry);
    assert_eq!(kids.len(), 4);
    let rects: Vec<_> = kids.iter().map(|k| h.rect(*k).unwrap()).collect();
    for (i, x) in [0.0, 110.0, 220.0].into_iter().enumerate() {
        assert!((rects[i].left() - x).abs() < 0.01, "{:?}", rects[i]);
        assert!((rects[i].size.width - 100.0).abs() < 0.01, "{:?}", rects[i]);
        assert!((rects[i].top() - 0.0).abs() < 0.01, "{:?}", rects[i]);
    }
    assert!((rects[3].left() - 220.0).abs() < 0.01, "{:?}", rects[3]);
    assert!((rects[3].top() - 30.0).abs() < 0.01, "{:?}", rects[3]);
    assert!((h.rect(masonry).unwrap().size.height - 60.0).abs() < 0.01);
}

#[test]
fn box_props_on_flex_containers() {
    let src = app_with(
        "    Column(margin: 4, margin_x: 2, min_width: 10, max_width: 380, min_height: 10, max_height: 280, overflow: hidden, main_gap: 2, cross_gap: 2, wrap: true) {\n      Text(\"a\")\n    }",
    );
    paints(&src);
}

#[test]
fn box_props_on_row_align_content() {
    let src = app_with(
        "    Row(margin_left: 1, margin_right: 1, margin_top: 1, margin_bottom: 1, overflow: auto, align_content: center) {\n      Text(\"a\")\n    }",
    );
    paints(&src);
}

#[test]
fn box_props_on_container_and_panel() {
    let src = app_with(
        "    Column {\n      Container(margin: [6, 3], min_width: 10, overflow: scroll, radius: 4) {\n        Text(\"a\")\n      }\n      Panel(margin_y: 5, max_width: 300, overflow: hidden)\n    }",
    );
    paints(&src);
}

#[test]
fn box_props_on_text_button_icon_spacer() {
    let src = app_with(
        "    Column {\n      Text(\"a\", width: px(100), min_width: 10, max_width: 200, margin: 4)\n      Button(\"ok\", width: px(120), margin_x: 4, min_height: 10)\n      Icon(name: \"x\", width: px(16), margin: 2)\n      Spacer(flex: 1, width: px(4), min_height: 2)\n    }",
    );
    paints(&src);
}

#[test]
fn scrollview_both_axis() {
    let src = app_with("    ScrollView(axis: both) {\n      Text(\"x\")\n    }");
    paints(&src);
}

#[test]
fn canvas_min_max() {
    let src = app_with(
        "    Canvas(width: px(200), height: px(100), min_width: 10, max_width: 300, min_height: 10, max_height: 200) {\n      Rect(x: 0, y: 0, w: 10, h: 10, fill: #FF0000)\n    }",
    );
    paints(&src);
}

#[test]
fn placement_outside_parent_is_rejected() {
    let src = app_with("    Column {\n      Text(\"a\", at: top)\n    }");
    let app = parse("Main.seq", &src).unwrap();
    let err = check("Main.seq", &app).unwrap_err();
    assert!(err.message.contains("at"), "{err}");
}

#[test]
fn grid_shape_still_works_in_canvas() {
    let src = app_with(
        "    Canvas(width: px(200), height: px(100)) {\n      Grid(step: 10, color: #333333)\n    }",
    );
    paints(&src);
}

#[test]
fn text_flex_takes_remaining_row_width() {
    let src = app_with(
        "    Row(width: px(400), height: px(40)) {\n      Panel(width: px(100), height: px(40))\n      Text(\"a\", flex: 1)\n    }",
    );
    let loaded = SeqLoader::load_str("Main.seq", &src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    let row = h.children(h.root().unwrap())[0];
    let kids = h.children(row);
    assert_eq!(kids.len(), 2);
    let text_rect = h.rect(kids[1]).unwrap();
    assert!(
        (text_rect.size.width - 300.0).abs() < 1.0,
        "flex: 1 takes the rest of the line, it is {}",
        text_rect.size.width
    );
}

#[test]
fn text_flex_weights_split_proportionally() {
    let src = app_with(
        "    Row(width: px(400), height: px(40)) {\n      Text(\"a\", flex: 1)\n      Text(\"b\", flex: 3)\n    }",
    );
    let loaded = SeqLoader::load_str("Main.seq", &src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    let row = h.children(h.root().unwrap())[0];
    let kids = h.children(row);
    assert_eq!(kids.len(), 2);
    let a = h.rect(kids[0]).unwrap();
    let b = h.rect(kids[1]).unwrap();

    assert!(
        ((a.size.width + b.size.width) - 400.0).abs() < 1.0,
        "flex fills the line: {} + {}",
        a.size.width,
        b.size.width
    );
    let ratio = b.size.width / a.size.width;
    assert!((ratio - 3.0).abs() < 0.3, "wagi 1:3, stosunek {ratio}",);
}
