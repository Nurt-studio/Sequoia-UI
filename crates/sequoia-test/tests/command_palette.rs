#![allow(clippy::unwrap_used)]

use std::cell::Cell;
use std::rc::Rc;

use sequoia_test::Harness;
use sequoia_ui::prelude::*;
use sequoia_ui::view::ElementId;

fn all_texts(h: &Harness) -> Vec<String> {
    let mut out = Vec::new();
    fn rec(h: &Harness, id: ElementId, out: &mut Vec<String>) {
        if h.type_name_of(id).is_some_and(|t| t.contains("Text"))
            && let Some(t) = h.text(id)
        {
            out.push(t);
        }
        for c in h.children(id) {
            rec(h, c, out);
        }
    }
    rec(h, h.root().unwrap(), &mut out);

    for oid in h.overlay_ids() {
        if let Some(el) = h.overlay_element(oid) {
            rec(h, el, &mut out);
        }
    }
    out
}

fn sync_twice(h: &Harness, w: f32, hh: f32) {
    h.sync(w, hh);
    h.sync(w, hh);
}

fn find_text_field(h: &Harness) -> Option<ElementId> {
    fn rec(h: &Harness, id: ElementId) -> Option<ElementId> {
        if h.type_name_of(id).is_some_and(|t| t.contains("TextField")) {
            return Some(id);
        }
        for c in h.children(id) {
            if let Some(hit) = rec(h, c) {
                return Some(hit);
            }
        }
        None
    }
    if let Some(hit) = rec(h, h.root().unwrap()) {
        return Some(hit);
    }
    for oid in h.overlay_ids() {
        if let Some(el) = h.overlay_element(oid)
            && let Some(hit) = rec(h, el)
        {
            return Some(hit);
        }
    }
    None
}

fn registry() -> (CommandRegistry, Rc<Cell<u32>>) {
    let r = CommandRegistry::new();
    let n = Rc::new(Cell::new(0u32));
    let n2 = n.clone();
    r.register("file.save", "Zapisz", "Plik", Some("Ctrl+S"), move || {
        n2.set(n2.get() + 1)
    });
    r.register("file.open", "Open", "Plik", None::<String>, || {});
    r.register("edit.undo", "Cofnij", "Edycja", None::<String>, || {});
    (r, n)
}

#[test]
fn menubar_shows_categories() {
    let (r, _) = registry();
    let h = Harness::new(MenuBar::new(r));
    h.sync(800.0, 40.0);
    let texts = all_texts(&h);
    assert!(texts.iter().any(|t| t == "Plik"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Edycja"), "{texts:?}");
}

#[test]
fn palette_filters_and_enter_runs_first_match() {
    let (r, n) = registry();
    let open = Signal::new(true);
    let pal = CommandPalette::new(r, open.clone());
    let h = Harness::new(Column::new().fill_size().child(pal.clone()));
    sync_twice(&h, 600.0, 400.0);

    assert!(all_texts(&h).iter().any(|t| t.contains("Zapisz")));
    assert!(all_texts(&h).iter().any(|t| t.contains("Cofnij")));

    pal.query_signal().set("zap".to_string());
    sync_twice(&h, 600.0, 400.0);
    let texts = all_texts(&h);
    assert!(texts.iter().any(|t| t.contains("Zapisz")), "{texts:?}");
    assert!(!texts.iter().any(|t| t.contains("Cofnij")), "{texts:?}");

    let field = find_text_field(&h).expect("pole palety");
    let r = h.rect(field).unwrap();
    h.click(r.left() + 10.0, r.top() + r.size.height * 0.5);
    h.tick();
    h.key(Key::ENTER);
    h.tick();
    assert_eq!(n.get(), 1);
    assert!(!pal.is_open());
}
