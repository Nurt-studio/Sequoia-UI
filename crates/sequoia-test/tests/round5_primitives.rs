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
    out
}

#[test]
fn accelerator_fires_only_when_allowed() {
    let fired = Rc::new(std::cell::Cell::new(0));
    let f2 = fired.clone();
    let acc = Accelerator::simple(Key::from_char('s'), Modifiers::CONTROL, move || {
        f2.set(f2.get() + 1);
    });
    let ctx = AcceleratorCtx {
        window: WindowId::PRIMARY,
        size: Size::new(800.0, 600.0),
        scale: 1.0,
        environment: Environment::new(),
    };
    assert!(acc.should_fire());
    acc.fire(&ctx);
    assert_eq!(fired.get(), 1);

    let blocked = Accelerator::simple(Key::from_char('s'), Modifiers::CONTROL, || {})
        .with_when(Rc::new(|| false));
    assert!(!blocked.should_fire());
}

#[test]
fn not_editing_follows_signal() {
    let editing = Signal::new(false);
    let guard = not_editing(editing.clone());
    assert!(guard());
    editing.set(true);
    assert!(!guard());
}

#[test]
fn dialog_opens_closes_and_fires_on_close() {
    let open = Signal::new(false);
    let closed = Rc::new(std::cell::Cell::new(0));
    let c2 = closed.clone();
    let dlg = Dialog::new(open.clone(), "To remove?")
        .content(Text::new("Nieodwracalne."))
        .danger("Delete", || {})
        .cancel("Anuluj")
        .on_close(move || c2.set(c2.get() + 1));
    assert!(!dlg.is_open());
    let h = Harness::new(Column::new().fill_size().child(dlg.clone()));
    h.sync(800.0, 600.0);
    open.set(true);
    h.sync(800.0, 600.0);
    assert!(dlg.is_open());
    dlg.close();
    assert!(!dlg.is_open());
    assert_eq!(closed.get(), 1);
    h.sync(800.0, 600.0);
}

#[test]
fn field_hides_error_until_touched() {
    let form = Form::new();
    let v = Signal::new(String::new());
    let f = Field::new(v.clone(), "Nazwa").rule("cannot be empty", |s: &str| !s.trim().is_empty());
    form.register(&f);
    let h = Harness::new(Column::new().fill_size().child(f.clone()));
    h.sync(400.0, 300.0);
    assert!(
        !all_texts(&h).iter().any(|t| t.contains("cannot be empty")),
        "przed touch brak komunikatu: {:?}",
        all_texts(&h)
    );
    form.touch_all();
    h.sync(400.0, 300.0);
    assert!(
        all_texts(&h).iter().any(|t| t.contains("cannot be empty")),
        "after touch_all error visible: {:?}",
        all_texts(&h)
    );
    v.set("stal".to_string());
    h.sync(400.0, 300.0);
    assert!(form.is_valid());
    assert!(!all_texts(&h).iter().any(|t| t.contains("cannot be empty")));
}

#[derive(Clone, PartialEq)]
struct Item {
    name: String,
    size: u64,
}

fn sample_table(rows: Signal<Vec<Item>>) -> Table<Item> {
    Table::new({
        let rows = rows.clone();
        move || rows.get()
    })
    .key(|r: &Item| {
        r.size
            .wrapping_mul(1_000_003)
            .wrapping_add(r.name.len() as u64)
    })
    .column(
        TableColumn::new("Nazwa")
            .sort_key(|a: &Item, b: &Item| a.name.cmp(&b.name))
            .matches(|r: &Item, q: &str| r.name.contains(q))
            .text_cell(|r: &Item| r.name.clone()),
    )
    .column(
        TableColumn::new("Rozmiar")
            .sort_key(|a: &Item, b: &Item| a.size.cmp(&b.size))
            .text_cell(|r: &Item| r.size.to_string()),
    )
    .row_height(28.0)
}

#[test]
fn table_sorts_and_filters() {
    let rows = Signal::new(vec![
        Item {
            name: "beta".into(),
            size: 2,
        },
        Item {
            name: "alfa".into(),
            size: 10,
        },
        Item {
            name: "gamma".into(),
            size: 1,
        },
    ]);
    let t = sample_table(rows.clone());
    let names: Vec<_> = t.filtered_sorted().iter().map(|r| r.name.clone()).collect();
    assert_eq!(names, vec!["beta", "alfa", "gamma"]);

    t.sort_signal().set(Some((0, SortDir::Asc)));
    let names: Vec<_> = t.filtered_sorted().iter().map(|r| r.name.clone()).collect();
    assert_eq!(names, vec!["alfa", "beta", "gamma"]);

    t.sort_signal().set(Some((1, SortDir::Desc)));
    let sizes: Vec<_> = t.filtered_sorted().iter().map(|r| r.size).collect();
    assert_eq!(sizes, vec![10, 2, 1]);

    t.sort_signal().set(None);
    let cols = t.columns_for_test();
    cols[0].filter.set("alf".to_string());
    let names: Vec<_> = t.filtered_sorted().iter().map(|r| r.name.clone()).collect();
    assert_eq!(names, vec!["alfa"]);
}

#[test]
fn table_mounts_without_panic() {
    let rows = Signal::new(vec![
        Item {
            name: "a".into(),
            size: 1,
        },
        Item {
            name: "b".into(),
            size: 2,
        },
    ]);
    let h = Harness::new(sample_table(rows));
    h.sync(600.0, 400.0);
    let texts = all_texts(&h);
    assert!(texts.iter().any(|t| t.contains("Nazwa")), "{texts:?}");
}

#[test]
fn tree_flattens_only_expanded_branches() {
    let open: TreeView<String> = TreeView::new(|| {
        vec![
            TreeNode::leaf("a", "A".to_string()),
            TreeNode::branch(
                "b",
                "B".to_string(),
                vec![
                    TreeNode::leaf("b1", "B1".to_string()),
                    TreeNode::leaf("b2", "B2".to_string()),
                ],
            ),
        ]
    })
    .row(|v: &String, _| Text::new(v.clone()));
    let ids: Vec<_> = open.flatten().iter().map(|r| r.id.clone()).collect();

    assert_eq!(ids, vec!["a", "b", "b1", "b2"]);

    let shut: TreeView<String> = TreeView::new(|| {
        vec![
            TreeNode::branch(
                "b",
                "B".to_string(),
                vec![TreeNode::leaf("b1", "B1".to_string())],
            )
            .expanded(false),
        ]
    })
    .row(|v: &String, _| Text::new(v.clone()));
    let ids: Vec<_> = shut.flatten().iter().map(|r| r.id.clone()).collect();
    assert_eq!(ids, vec!["b"]);
}

#[test]
fn tree_mounts_and_toggles() {
    let expanded = Signal::new(true);
    let ex = expanded.clone();
    let t: TreeView<String> = TreeView::new(move || {
        vec![TreeNode {
            id: "b".into(),
            value: "B".into(),
            children: vec![TreeNode::leaf("b1", "B1".into())],
            expanded: ex.clone(),
        }]
    })
    .row(|v: &String, _| Text::new(v.clone()));
    let h = Harness::new(t);
    h.sync(400.0, 300.0);
    assert!(
        all_texts(&h).iter().any(|t| t == "B1"),
        "{:?}",
        all_texts(&h)
    );
    expanded.set(false);
    h.sync(400.0, 300.0);
    assert!(
        !all_texts(&h).iter().any(|t| t == "B1"),
        "{:?}",
        all_texts(&h)
    );
}

#[derive(Clone)]
struct Boom;

impl View for Boom {
    #[allow(refining_impl_trait)]
    fn body(&self) -> Unit {
        panic!("boom-test");
    }
}

#[test]
fn boundary_shows_fallback_after_child_panic() {
    let seen = Rc::new(std::cell::Cell::new(0));
    let s2 = seen.clone();
    let b = ErrorBoundary::new(|| AnyView::new(Boom))
        .fallback(|e| Text::new(format!("it fell: {}", e.message)))
        .on_error(move |_| s2.set(s2.get() + 1));
    assert!(!b.is_failed());
    let h = Harness::new(Column::new().fill_size().child(b.clone()));
    h.sync(400.0, 300.0);
    assert!(b.is_failed(), "panika dziecka stawia fallback");
    assert_eq!(seen.get(), 1);
    assert!(
        all_texts(&h).iter().any(|t| t.contains("it fell")),
        "{:?}",
        all_texts(&h)
    );
    assert!(!h.runtime().errors().is_empty());
    b.reset();
    assert!(!b.is_failed());
}

#[test]
fn async_view_walks_idle_loading_ready_failed() {
    let res: Resource<String> = Resource::new();
    let r2 = res.clone();
    let v = AsyncView::new(res.clone())
        .loading(|| Text::new("czekaj…"))
        .data(|s: &String| Text::new(format!("got:{s}")))
        .error(|e: &str| Text::new(format!("err:{e}")))
        .retry("Jeszcze raz", move || {
            r2.begin_loading();
        });
    let h = Harness::new(v);
    h.sync(400.0, 300.0);
    assert!(
        all_texts(&h).iter().any(|t| t.contains("czekaj")),
        "{:?}",
        all_texts(&h)
    );

    let generation = res.begin_loading();
    h.sync(400.0, 300.0);
    assert!(all_texts(&h).iter().any(|t| t.contains("czekaj")));

    res.complete(generation, "dane".to_string());
    h.sync(400.0, 300.0);
    assert!(
        all_texts(&h).iter().any(|t| t.contains("got:dane")),
        "{:?}",
        all_texts(&h)
    );

    let generation2 = res.begin_loading();
    res.fail(generation2, "boom".to_string());
    h.sync(400.0, 300.0);
    let texts = all_texts(&h);
    assert!(texts.iter().any(|t| t.contains("err:boom")), "{texts:?}");
    assert!(texts.iter().any(|t| t.contains("Jeszcze raz")), "{texts:?}");
}

#[test]
fn dev_overlay_summarizes_runtime() {
    let h = Harness::new(Text::new("hello"));
    h.sync(400.0, 300.0);
    let dev = DevOverlay::new(h.runtime());
    let summary = dev.summary_untracked();
    assert!(
        summary.contains("el=") && summary.contains("errors="),
        "{summary}"
    );
    let h2 = Harness::new(Column::new().fill_size().child(dev));
    h2.sync(400.0, 300.0);
    assert!(
        all_texts(&h2).iter().any(|t| t.contains("el=")),
        "{:?}",
        all_texts(&h2)
    );
}
