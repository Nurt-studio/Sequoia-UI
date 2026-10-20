#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_core::Length;
use sequoia_elements::Container;
use sequoia_reactive::Signal;
use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[derive(Clone, PartialEq, Debug)]
struct Item {
    id: u64,
    name: &'static str,
}

fn items(v: &[(u64, &'static str)]) -> Vec<Item> {
    v.iter().map(|(id, name)| Item { id: *id, name }).collect()
}

struct Row {
    name: &'static str,
}

impl View for Row {
    fn body(&self) -> impl View + 'static {
        Text::new(self.name)
    }
}

struct TrackedRow {
    name: &'static str,
    mounts: Rc<Cell<u32>>,
    unmounts: Rc<Cell<u32>>,
}

impl View for TrackedRow {
    fn body(&self) -> impl View + 'static {
        Text::new(self.name)
    }

    fn on_mount(&self) {
        self.mounts.set(self.mounts.get() + 1);
    }

    fn on_unmount(&self) {
        self.unmounts.set(self.unmounts.get() + 1);
    }
}

struct ListView {
    items: Signal<Vec<Item>>,
}

impl View for ListView {
    fn body(&self) -> impl View + 'static {
        List::new({
            let s = self.items.clone();
            move || s.get()
        })
        .key(|it: &Item| it.id)
        .item(|it: &Item| Row { name: it.name })
    }
}

struct TrackedView {
    items: Signal<Vec<Item>>,
    mounts: Rc<Cell<u32>>,
    unmounts: Rc<Cell<u32>>,
}

impl View for TrackedView {
    fn body(&self) -> impl View + 'static {
        List::new({
            let s = self.items.clone();
            move || s.get()
        })
        .key(|it: &Item| it.id)
        .item({
            let m = self.mounts.clone();
            let u = self.unmounts.clone();
            move |it: &Item| TrackedRow {
                name: it.name,
                mounts: m.clone(),
                unmounts: u.clone(),
            }
        })
    }
}

fn list_el(h: &Harness) -> ElementId {
    let root = h.root().unwrap();
    let children = h.children(root);
    assert_eq!(children.len(), 1, "root has one child: List");
    children[0]
}

fn row_ids(h: &Harness) -> Vec<ElementId> {
    h.children(list_el(h))
}

fn row_names(h: &Harness) -> Vec<String> {
    row_ids(h)
        .iter()
        .map(|&row| {
            let text = h.children(row);
            assert_eq!(text.len(), 1, "row has one child: Text");
            h.text(text[0]).unwrap()
        })
        .collect()
}

#[test]
fn reorder_preserves_identity_and_order() {
    let s = Signal::new(items(&[(1, "a"), (2, "b"), (3, "c")]));
    let h = Harness::new(ListView { items: s.clone() });
    let before = row_ids(&h);
    assert_eq!(row_names(&h), ["a", "b", "c"]);

    assert_eq!(h.stats().mounts, 8);

    h.set(&s, items(&[(3, "c"), (1, "a"), (2, "b")]));

    let after = row_ids(&h);
    assert_eq!(row_names(&h), ["c", "a", "b"]);

    let mut b = before.clone();
    let mut a = after.clone();
    b.sort();
    a.sort();
    assert_eq!(b, a, "identity must survive reorder");

    assert_eq!(after[0], before[2], "'c' moved to front, same ElementId");
    assert_eq!(after[1], before[0]);
    assert_eq!(after[2], before[1]);
    assert_eq!(h.stats().unmounts, 0, "reorder must not unmount");

    assert_eq!(h.stats().mounts, 8, "zero remounts after reorder");
}

#[test]
fn reorder_does_not_remount_rows() {
    let s = Signal::new(items(&[(1, "a"), (2, "b"), (3, "c")]));
    let mounts = Rc::new(Cell::new(0u32));
    let unmounts = Rc::new(Cell::new(0u32));
    let h = Harness::new(TrackedView {
        items: s.clone(),
        mounts: mounts.clone(),
        unmounts: unmounts.clone(),
    });
    assert_eq!(
        mounts.get(),
        3,
        "rows only; List and root excluded from TrackedRow"
    );

    h.set(&s, items(&[(3, "c"), (1, "a"), (2, "b")]));
    assert_eq!(mounts.get(), 3, "no row was remounted");
    assert_eq!(unmounts.get(), 0);

    h.set(&s, items(&[(2, "b"), (3, "c"), (1, "a")]));
    assert_eq!(mounts.get(), 3);
    assert_eq!(unmounts.get(), 0);
}

#[test]
fn removal_unmounts_only_removed_keys() {
    let s = Signal::new(items(&[(1, "a"), (2, "b"), (3, "c")]));
    let h = Harness::new(ListView { items: s.clone() });
    let before = row_ids(&h);

    h.set(&s, items(&[(2, "b")]));

    assert!(!h.is_alive(before[0]), "row 'a' unmounted");
    assert!(h.is_alive(before[1]), "row 'b' survived");
    assert!(!h.is_alive(before[2]), "row 'c' unmounted");
    assert_eq!(row_names(&h), ["b"]);

    assert_eq!(
        h.stats().unmounts,
        4,
        "removed 'a' and 'c' with their Texts"
    );
}

#[test]
fn insert_keeps_existing_ids() {
    let s = Signal::new(items(&[(2, "b")]));
    let h = Harness::new(ListView { items: s.clone() });
    let old = row_ids(&h)[0];

    h.set(&s, items(&[(1, "a"), (2, "b"), (3, "c")]));

    let now = row_ids(&h);
    assert_eq!(now.len(), 3);
    assert_eq!(now[1], old, "'b' moved to index 1 with preserved id");
    assert_eq!(row_names(&h), ["a", "b", "c"]);
    assert_eq!(h.stats().unmounts, 0);
}

#[derive(Clone, PartialEq)]
struct KItem {
    id: u64,
    wide: bool,
}

struct NarrowRow {
    id: u64,
}

impl View for NarrowRow {
    fn body(&self) -> impl View + 'static {
        Text::new(format!("n{}", self.id))
    }
}

struct WideRow {
    id: u64,
}

impl View for WideRow {
    fn body(&self) -> impl View + 'static {
        Text::new(format!("w{}", self.id))
    }
}

struct KindView {
    items: Signal<Vec<KItem>>,
}

impl View for KindView {
    fn body(&self) -> impl View + 'static {
        List::new({
            let s = self.items.clone();
            move || s.get()
        })
        .key(|it: &KItem| it.id)
        .item(|it: &KItem| {
            if it.wide {
                AnyView::new(WideRow { id: it.id })
            } else {
                AnyView::new(NarrowRow { id: it.id })
            }
        })
    }
}

#[test]
fn type_change_under_same_key_remounts() {
    let s = Signal::new(vec![KItem { id: 7, wide: false }]);
    let h = Harness::new(KindView { items: s.clone() });
    let row = row_ids(&h)[0];
    let text = h.children(row)[0];
    assert_eq!(h.text(text).as_deref(), Some("n7"));

    h.set(&s, vec![KItem { id: 7, wide: true }]);

    let row2 = row_ids(&h)[0];
    assert_ne!(row, row2, "type change under the same key = remount");
    assert!(!h.is_alive(row));
    let text2 = h.children(row2)[0];
    assert_eq!(h.text(text2).as_deref(), Some("w7"));
    assert!(h.stats().unmounts >= 1);
}

#[test]
fn duplicate_keys_do_not_panic() {
    let s = Signal::new(items(&[(5, "x"), (5, "y")]));
    let h = Harness::new(ListView { items: s.clone() });
    assert_eq!(row_ids(&h).len(), 2, "duplicate key = insert, not panic");
    assert_eq!(row_names(&h), ["x", "y"]);

    h.set(&s, items(&[(5, "z")]));
    assert_eq!(row_names(&h), ["z"]);

    assert_eq!(h.stats().unmounts, 2, "duplicate (Row + Text) unmounted");
}

#[test]
fn unkeyed_list_defaults_to_positional() {
    struct PlainView {
        items: Signal<Vec<Item>>,
    }
    impl View for PlainView {
        fn body(&self) -> impl View + 'static {
            List::new({
                let s = self.items.clone();
                move || s.get()
            })
            .item(|it: &Item| Row { name: it.name })
        }
    }

    let s = Signal::new(items(&[(1, "a")]));
    let h = Harness::new(PlainView { items: s.clone() });
    let first = row_ids(&h)[0];

    h.set(&s, items(&[(9, "zz"), (8, "yy")]));
    let now = row_ids(&h);
    assert_eq!(now[0], first, "no keys: index 0 reused positionally");
    assert_eq!(row_names(&h), ["zz", "yy"]);
    assert_eq!(h.key_of(now[0]), Some(0));
    assert_eq!(h.key_of(now[1]), Some(1));
}

struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

#[test]
fn stress_shuffles_keep_identity_consistent() {
    let n: u64 = 40;
    let mut expected: Vec<Item> = (0..n)
        .map(|i| Item {
            id: i + 1,
            name: "row",
        })
        .collect();

    let s = Signal::new(expected.clone());
    let h = Harness::new(ListView { items: s.clone() });

    let mut key_to_el: HashMap<u64, ElementId> = HashMap::new();
    for &el in &row_ids(&h) {
        key_to_el.insert(h.key_of(el).unwrap(), el);
    }

    let mut rng = XorShift::new(0xC0FF_EE12_3456_789A);
    for _ in 0..15 {
        for i in (1..expected.len()).rev() {
            let j = (rng.next() % (i as u64 + 1)) as usize;
            expected.swap(i, j);
        }
        h.set(&s, expected.clone());

        let rows = row_ids(&h);
        assert_eq!(rows.len() as u64, n);
        for (pos, &el) in rows.iter().enumerate() {
            let key = h.key_of(el).unwrap();
            assert_eq!(key, expected[pos].id, "id order matches data");
            assert_eq!(
                key_to_el.get(&key),
                Some(&el),
                "key ElementId {key} stable across shuffles"
            );
        }

        assert_eq!(h.element_count(), 82, "no slot leaks");
        assert_eq!(h.stats().unmounts, 0, "shuffle must not unmount rows");
    }
}

#[test]
fn large_reorder_keeps_identity() {
    let n: u64 = 500;
    let base: Vec<Item> = (1..=n).map(|id| Item { id, name: "row" }).collect();
    let s = Signal::new(base.clone());
    let h = Harness::new(ListView { items: s.clone() });
    let before = row_ids(&h);

    let mut rev = base;
    rev.reverse();
    h.set(&s, rev);

    let after = row_ids(&h);
    assert_eq!(after.len() as u64, n);
    assert_eq!(h.stats().unmounts, 0, "reorder does not unmount");
    assert_eq!(h.stats().mounts, 2 * n + 2, "no remounts");
    for i in 0..n as usize {
        assert_eq!(
            after[i],
            before[n as usize - 1 - i],
            "row {i} identity after reverse"
        );
    }
}

struct FixedRow;

impl View for FixedRow {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(24.0),
            ..LayoutStyle::default()
        }
    }
}

#[test]
fn many_rows_keep_height_in_bounded_container() {
    let rows = 50u64;
    let h = Harness::new(
        Container::new().width(240.0).height(200.0).child(
            List::new(move || (0..rows).collect::<Vec<_>>())
                .key(|id: &u64| *id)
                .item(|_| FixedRow),
        ),
    );
    h.layout(240.0, 200.0);

    let list = h.children(h.root().unwrap())[0];
    let kids = h.children(list);
    assert_eq!(kids.len() as u64, rows);

    for (i, &child) in kids.iter().enumerate() {
        let r = h.rect(child).unwrap();
        assert!(
            (r.size.height - 24.0).abs() < 0.5,
            "row {i} squeezed to {}",
            r.size.height
        );
        assert!(
            (r.top() - i as f32 * 24.0).abs() < 0.5,
            "row {i} at wrong position: {r:?}"
        );
    }

    let last = h.rect(*kids.last().unwrap()).unwrap();
    assert!(last.bottom() > 200.0 + 0.5, "list must overflow: {last:?}");
}
