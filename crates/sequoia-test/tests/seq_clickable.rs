#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_reactive::Signal;
use sequoia_seq::{FieldSignal, HostBindings, SeqLoader, SeqRow, check, parse};
use sequoia_test::Harness;

fn glyphs(h: &Harness) -> usize {
    use sequoia_core::RenderItem;
    h.paint()
        .items()
        .iter()
        .map(|i| match i {
            RenderItem::Text(t) => t.glyphs.len(),
            _ => 0,
        })
        .sum()
}

#[test]
fn container_click_fires() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  on pick()\n  window M {\n    Container(width: fill(1), height: px(40), on_click: { pick() }) {\n      Text(\"karta\")\n    }\n  }\n}\n";
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    let b = HostBindings::new().on("pick", move || f.set(f.get() + 1));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 5);
    h.click(100.0, 20.0);
    h.tick();
    assert_eq!(fired.get(), 1);
}

#[test]
fn row_click_fires_and_keeps_layout() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 100)\n  on pick()\n  window M {\n    Row(width: fill(1), height: px(48), padding_x: 16, gap: 12, align: center, hover: #2B2B2B, on_click: { pick() }) {\n      Text(\"a\")\n      Spacer(flex: 1)\n      Text(\"b\")\n    }\n  }\n}\n";
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    let b = HostBindings::new().on("pick", move || f.set(f.get() + 1));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 100.0);
    assert_eq!(glyphs(&h), 2);

    h.click(390.0, 24.0);
    h.tick();
    assert_eq!(fired.get(), 1);
}

#[test]
fn clickable_row_content_still_stretches() {
    for clickable in [false, true] {
        let click = if clickable {
            ", hover: #2B2B2B, on_click: { pick() }"
        } else {
            ""
        };
        let src = format!(
            "app A {{\n  title: \"A\"\n  size: (400, 100)\n  on pick()\n  window M {{\n    Row(width: fill(1), height: px(48), padding_x: 16, gap: 12, align: center{click}) {{\n      Text(\"a\")\n      Spacer(flex: 1)\n      Text(\"b\")\n    }}\n  }}\n}}\n"
        );
        let b = HostBindings::new().on("pick", || {});
        let loaded = SeqLoader::load_str_with("Main.seq", &src, &b).unwrap();
        let h = Harness::new(loaded.view);
        h.layout(400.0, 100.0);
        let root = h.root().expect("root");
        let wrap = h.children(root).into_iter().next().expect("wiersz");

        let row = if clickable {
            h.children(wrap).into_iter().next().expect("wnetrze")
        } else {
            wrap
        };
        let kids = h.children(row);
        assert_eq!(kids.len(), 3);
        let bx = h.rect(kids[2]).expect("rect").left();
        assert!(
            bx > 340.0,
            "klik={clickable}: trailing tekst przy prawej (x={bx})"
        );
    }
}

#[test]
fn click_through_text_fires_row_action() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 100)\n  on pick()\n  window M {\n    Row(width: fill(1), height: px(48), padding_x: 16, gap: 12, align: center, hover: #2B2B2B, on_click: { pick() }) {\n      Text(\"kliknij-mnie-tutaj\")\n    }\n  }\n}\n";
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    let b = HostBindings::new().on("pick", move || f.set(f.get() + 1));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 100.0);

    h.click(30.0, 24.0);
    h.tick();
    assert_eq!(fired.get(), 1, "klik w tekst odpala on_click wiersza");
}

#[test]
fn list_row_click_through_title_text() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 300)\n  on play(id: Int)\n  window M {\n    ScrollView(axis: vertical) {\n      Column(width: fill(1), gap: 0) {\n        List(items: songs, key: id) {\n          Row(width: fill(1), height: px(48), padding_x: 16, gap: 12, align: center, hover: #2B2B2B, on_click: { play(item.id) }) {\n            If(when: item.selected) {\n              Container(width: px(3), height: px(20), background: #35C2F1)\n            }\n            Container(width: px(32), height: px(32), background: #2B2B2B, radius: 2)\n            Column(gap: 2) {\n              Text(dynamic: item.title, size: 13, truncate: 280)\n            }\n          }\n        }\n      }\n    }\n  }\n}\n";
    let rows = Signal::new(vec![
        Song {
            id: 7,
            title: Signal::new("Seven Songs Long Title".to_string()),
            selected: Signal::new(false),
        },
        Song {
            id: 9,
            title: Signal::new("Nine".to_string()),
            selected: Signal::new(false),
        },
    ]);
    let got = Rc::new(Cell::new(0i64));
    let g = got.clone();
    let b = HostBindings::new()
        .rows("songs", rows)
        .on_int("play", move |id| g.set(id));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);

    h.click(80.0, 24.0);
    h.tick();
    assert_eq!(got.get(), 7, "klik w tytul odpala play z id");
}

#[test]
fn clickable_row_pointer_cursor_over_text_and_gap() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 100)\n  on pick()\n  window M {\n    Row(width: fill(1), height: px(48), padding_x: 16, gap: 12, align: center, hover: #2B2B2B, on_click: { pick() }) {\n      Text(\"abc\")\n    }\n  }\n}\n";
    let b = HostBindings::new().on("pick", || {});
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 100.0);
    use sequoia_input::CursorIcon;
    h.pointer_move(20.0, 24.0);
    h.tick();
    assert_eq!(h.cursor_icon(), CursorIcon::Pointer, "pointer nad tekstem");
    h.pointer_move(390.0, 24.0);
    h.tick();
    assert_eq!(
        h.cursor_icon(),
        CursorIcon::Pointer,
        "pointer nad pustym polem"
    );
}

#[test]
fn nested_button_fires_once_row_stays_quiet() {
    let src = "app A {\n  title: \"A\"\n  size: (400, 100)\n  on pick()\n  on row()\n  window M {\n    Row(width: fill(1), height: px(48), padding_x: 16, gap: 12, align: center, hover: #2B2B2B, on_click: { row() }) {\n      Text(\"a\")\n      Button(\"Go\", style: primary, on_click: { pick() })\n    }\n  }\n}\n";
    let btn = Rc::new(Cell::new(0u32));
    let row = Rc::new(Cell::new(0u32));
    let b = btn.clone();
    let r = row.clone();
    let bindings = HostBindings::new()
        .on("pick", move || b.set(b.get() + 1))
        .on("row", move || r.set(r.get() + 1));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &bindings).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 100.0);

    h.click(70.0, 24.0);
    h.tick();
    assert_eq!(btn.get(), 1, "button odpalil");
    assert_eq!(row.get(), 0, "wiersz nie dubluje");

    h.click(20.0, 24.0);
    h.tick();
    assert_eq!(row.get(), 1, "klik w tekst odpala wiersz");
}

#[test]
fn tmp_touch_pan_debug() {
    for variant in ["plain", "click", "hover", "ifonly"] {
        let row = match variant {
            "plain" => {
                "Row(width: fill(1), height: px(48)) {\n            Text(dynamic: item.title)\n          }"
            }
            "click" => {
                "Row(width: fill(1), height: px(48), on_click: { play(item.id) }) {\n            Text(dynamic: item.title)\n          }"
            }
            "hover" => {
                "Row(width: fill(1), height: px(48), hover: #2B2B2B) {\n            Text(dynamic: item.title)\n          }"
            }
            _ => {
                "Row(width: fill(1), height: px(48)) {\n            If(when: item.selected) {\n              Container(width: px(3), height: px(20), background: #35C2F1)\n            }\n            Text(dynamic: item.title)\n          }"
            }
        };
        let src = format!(
            "app A {{\n  title: \"A\"\n  size: (400, 600)\n  on play(id: Int)\n  window M {{\n    ScrollView(axis: vertical) {{\n      Column(width: fill(1), gap: 0) {{\n        List(items: songs, key: id) {{\n          {row}\n        }}\n      }}\n    }}\n  }}\n}}\n"
        );
        let rows = Signal::new(
            (0..10)
                .map(|i| Song {
                    id: i,
                    title: Signal::new(format!("R{i:02}")),
                    selected: Signal::new(false),
                })
                .collect::<Vec<_>>(),
        );
        let b = HostBindings::new()
            .rows("songs", rows)
            .on_int("play", |_| {});
        let loaded = SeqLoader::load_str_with("Main.seq", &src, &b).unwrap();
        let h = Harness::new(loaded.view);
        h.layout(400.0, 600.0);
        h.touch_down(1, 200.0, 300.0, None);
        h.touch_move(1, 200.0, 200.0, None);
        h.touch_move(1, 200.0, 100.0, None);
        h.touch_up(1, 200.0, 100.0);
        h.tick();
        h.layout(400.0, 600.0);

        fn find_scroll(h: &Harness, id: sequoia_view::ElementId) -> Option<f32> {
            if h.type_name_of(id).is_some_and(|n| n.contains("ScrollView")) {
                let v: Rc<sequoia_view::ScrollView> = h.view(id)?;
                return Some(sequoia_view::ScrollView::offset(&v, &h.runtime(), id));
            }
            h.children(id)
                .into_iter()
                .filter_map(|c| find_scroll(h, c))
                .next()
        }
        let off = find_scroll(&h, h.root().unwrap()).unwrap();
        eprintln!("VARIANT={variant} OFFSET={off}");
        assert!(off > 0.0, "[{variant}] touch pan dziala (offset={off})");
    }
}

#[test]
fn tmp_if_flip_in_list_row() {
    use sequoia_core::{Color, RenderItem};
    let src = "app A {\n  title: \"A\"\n  size: (300, 400)\n  on play(id: Int)\n  on flip()\n  window M {\n    Column {\n      Button(\"flip\", on_click: { flip() })\n      ScrollView(axis: vertical) {\n        Column(width: fill(1), gap: 0) {\n          List(items: songs, key: id) {\n            Row(width: fill(1), height: px(48), hover: #2B2B2B, on_click: { play(item.id) }) {\n              If(when: item.selected) {\n                Container(width: px(10), height: px(10), background: #35C2F1)\n              }\n              Text(dynamic: item.title)\n            }\n          }\n        }\n      }\n    }\n  }\n}\n";
    let s1 = Signal::new(true);
    let s2 = Signal::new(false);
    let a = s1.clone();
    let b2 = s2.clone();
    let rows = Signal::new(vec![
        Song {
            id: 7,
            title: Signal::new("Seven".to_string()),
            selected: s1.clone(),
        },
        Song {
            id: 9,
            title: Signal::new("Nine".to_string()),
            selected: s2.clone(),
        },
    ]);
    let b = HostBindings::new()
        .rows("songs", rows.clone())
        .on_int("play", |_| {})
        .on("flip", move || {
            a.set(false);
            b2.set(true);
        });
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    let cyan = |h: &Harness| {
        h.paint()
            .items()
            .iter()
            .filter(|i| match i {
                RenderItem::Quad(q) => q.color == Color::rgb8(0x35, 0xC2, 0xF1),
                _ => false,
            })
            .count()
    };
    assert_eq!(cyan(&h), 1, "marker na wierszu 1");
    h.click(20.0, 12.0);
    h.tick();
    std::thread::sleep(std::time::Duration::from_millis(500));
    h.tick();
    h.layout(300.0, 200.0);

    let sizes: Vec<(f32, f32)> = h
        .paint()
        .items()
        .iter()
        .filter_map(|i| match i {
            RenderItem::Quad(q) if q.color == Color::rgb8(0x35, 0xC2, 0xF1) => {
                Some((q.rect.size.width, q.rect.size.height))
            }
            _ => None,
        })
        .collect();
    eprintln!("SIZES={sizes:?}");

    rows.update(|v| {
        v.push(Song {
            id: 11,
            title: Signal::new("Eleven".to_string()),
            selected: Signal::new(false),
        });
    });
    h.tick();
    h.layout(300.0, 200.0);
    let sizes2: Vec<(f32, f32)> = h
        .paint()
        .items()
        .iter()
        .filter_map(|i| match i {
            RenderItem::Quad(q) if q.color == Color::rgb8(0x35, 0xC2, 0xF1) => {
                Some((q.rect.size.width, q.rect.size.height))
            }
            _ => None,
        })
        .collect();
    eprintln!("SIZES_AFTER_FORCED={sizes2:?}");
}

#[test]
fn hover_only_needs_no_host() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Row(width: fill(1), height: px(32), hover: #2B2B2B) {\n      Text(\"x\")\n    }\n  }\n}\n";
    let loaded = SeqLoader::load_str("Main.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(200.0, 100.0);
    assert_eq!(glyphs(&h), 1);
}

#[test]
fn click_needs_host() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Container(on_click: { pick() }) {\n      Text(\"x\")\n    }\n  }\n}\n";
    let err = match SeqLoader::load_str("Main.seq", src) {
        Err(e) => e,
        Ok(_) => panic!("oczekiwano bledu"),
    };
    assert!(err.message.contains("hosta"), "{err}");
}

#[test]
fn click_rejected_on_text() {
    let src = "app A {\n  title: \"A\"\n  size: (200, 100)\n  window M {\n    Text(\"x\", on_click: { pick() })\n  }\n}\n";
    let app = parse("Main.seq", src).unwrap();
    let err = check("Main.seq", &app).unwrap_err();
    assert!(err.message.contains("on_click"), "{err}");
}

#[derive(Clone)]
struct Song {
    id: i64,
    title: Signal<String>,
    selected: Signal<bool>,
}

impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl SeqRow for Song {
    fn id(&self) -> i64 {
        self.id
    }

    fn fields(&self) -> Vec<(String, FieldSignal)> {
        vec![
            ("title".to_string(), FieldSignal::Str(self.title.clone())),
            (
                "selected".to_string(),
                FieldSignal::Bool(self.selected.clone()),
            ),
        ]
    }
}

#[test]
fn list_row_click_carries_id() {
    let src = "app A {\n  title: \"A\"\n  size: (300, 200)\n  on play(id: Int)\n  window M {\n    List(items: songs, key: id) {\n      Row(width: fill(1), height: px(48), padding_x: 16, gap: 12, align: center, on_click: { play(item.id) }) {\n        Text(dynamic: item.title)\n      }\n    }\n  }\n}\n";
    let rows = Signal::new(vec![
        Song {
            id: 7,
            title: Signal::new("Seven".to_string()),
            selected: Signal::new(false),
        },
        Song {
            id: 9,
            title: Signal::new("Nine".to_string()),
            selected: Signal::new(false),
        },
    ]);
    let got = Rc::new(Cell::new(0i64));
    let g = got.clone();
    let b = HostBindings::new()
        .rows("songs", rows)
        .on_int("play", move |id| g.set(id));
    let loaded = SeqLoader::load_str_with("Main.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(300.0, 200.0);
    assert_eq!(glyphs(&h), 9, "Seven + Nine");
    h.click(150.0, 24.0);
    h.tick();
    assert_eq!(got.get(), 7, "klik w pierwszy wiersz niesie id");
}
