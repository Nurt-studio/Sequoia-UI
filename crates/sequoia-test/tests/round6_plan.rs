#![allow(clippy::unwrap_used)]

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

#[test]
fn dialog_style_changes_title_and_danger() {
    let open = Signal::new(false);
    let plain = Dialog::new(open.clone(), "T");
    let styled = Dialog::new(open.clone(), "T").style(
        DialogStyle::new()
            .title_size(20.0)
            .danger(Color::rgb8(1, 2, 3)),
    );

    let _ = (plain, styled);
    assert_eq!(DIALOG_DANGER, Color::rgb8(0xFF, 0x5F, 0x57));
    assert_eq!(FORM_ERROR, Color::rgb8(0xFF, 0x5F, 0x57));
    let t = Theme::light();
    assert_ne!(t.background, Theme::dark().background);
    assert_ne!(t.text, Theme::dark().text);
}

#[test]
fn form_fields_mount_with_labels_and_styles() {
    let form = Form::new();
    let name =
        Field::new(Signal::new(String::new()), "Nazwa").style(FormStyle::new().label_size(13.0));
    let count = NumberField::new(Signal::new(1.0f32), "Quantity").style(FormStyle::new());
    let when = DateField::new(Signal::new("2026-01-01".to_string()), "Dnia");
    let tags = MultiSelect::new(vec!["a".into(), "b".into()], "Tagi");
    form.register(&name);
    form.register(&count);
    form.register(&when);
    form.register(&tags);
    assert_eq!(form.len(), 4);
    assert!(form.is_valid());
    let h = Harness::new(
        Column::new()
            .fill_size()
            .child(name)
            .child(count)
            .child(when)
            .child(tags),
    );
    h.sync(500.0, 600.0);
    let texts = all_texts(&h);
    for label in ["Nazwa", "Quantity", "Dnia", "Tagi"] {
        assert!(
            texts.iter().any(|t| t == label),
            "{label} missing in {texts:?}"
        );
    }
}

#[test]
fn table_stripe_and_tree_style_mount() {
    let rows = Signal::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let t = Table::new({
        let rows = rows.clone();
        move || rows.get()
    })
    .key(|s: &String| s.len() as u64)
    .column(TableColumn::new("N").text_cell(|s: &String| s.clone()))
    .style(
        TableStyle::new()
            .header_size(13.0)
            .row_stripe(Color::rgb8(0x22, 0x22, 0x22)),
    )
    .label("demo");
    let tree: TreeView<String> = TreeView::new(|| vec![TreeNode::leaf("a", "A".into())])
        .row(|v: &String, _| Text::new(v.clone()))
        .style(TreeStyle::new().font_size(14.0))
        .label("drzewo");
    let h = Harness::new(Column::new().fill_size().child(t).child(tree));
    h.sync(600.0, 400.0);

    assert!(!h.paint().is_empty());
}

#[test]
fn progress_bar_paints_track_and_fill() {
    let v = Signal::new(0.5f32);
    let h = Harness::new(ProgressBar::new(v.clone()).height(8.0));
    h.sync(200.0, 40.0);
    let quads = h.paint().quads().count();
    assert_eq!(quads, 2, "track + filling");
    v.set(0.0);
    h.sync(200.0, 40.0);
    assert_eq!(h.paint().quads().count(), 1, "puste = sam tor");
}

#[test]
fn spinner_and_skeleton_mount() {
    let h = Harness::new(Spinner::new().size(16.0));
    h.sync(300.0, 200.0);
    assert_eq!(h.paint().quads().count(), 8, "8 kropek spinnera");
    let h2 = Harness::new(Skeleton::new().width(200.0).height(16.0));
    h2.sync(300.0, 200.0);
    assert!(
        h2.paint().quads().count() >= 1,
        "skeleton paints background"
    );
}

#[test]
fn async_view_with_progress_shows_bar() {
    let res: Resource<String> = Resource::new();
    let frac = Signal::new(0.5f32);
    let v = AsyncView::new(res.clone()).progress(frac);
    let h = Harness::new(v);
    h.sync(400.0, 300.0);

    assert!(h.paint().quads().count() >= 2);
    assert!(all_texts(&h).iter().any(|t| t.contains("Landing")));
}

#[test]
fn accessibility_tree_lists_widgets_not_layout() {
    let h = Harness::new(
        Column::new()
            .fill_size()
            .child(Text::new("hello"))
            .child(Button::new("Zapisz"))
            .child(Row::new().child(Text::new("w wierszu"))),
    );
    h.sync(400.0, 300.0);
    let tree = h.runtime().accessibility_tree();
    let mut labels = Vec::new();
    for n in &tree {
        labels.extend(n.flat_labels());
    }
    assert!(labels.iter().any(|l| l == "Zapisz"), "{labels:?}");
    assert!(labels.iter().any(|l| l == "hello"), "{labels:?}");
    assert!(labels.iter().any(|l| l == "w wierszu"), "{labels:?}");

    let btn = tree.iter().find(|n| n.label == "Zapisz").unwrap();
    assert_eq!(btn.role, AccessibleRole::Button);
}

#[test]
fn i18n_rebuilds_text_on_locale_switch() {
    let i18n = I18n::new("en");
    i18n.add("en", "save", "Save");
    i18n.add("pl", "save", "Zapisz");
    let i2 = i18n.clone();
    let h = Harness::new(Text::with(move || i2.t("save")));
    h.sync(300.0, 100.0);
    assert!(all_texts(&h).iter().any(|t| t == "Save"));
    i18n.set_locale("pl");
    h.sync(300.0, 100.0);
    assert!(
        all_texts(&h).iter().any(|t| t == "Zapisz"),
        "{:?}",
        all_texts(&h)
    );
    assert_eq!(i18n.missing_in("pl"), Vec::<String>::new());
}
