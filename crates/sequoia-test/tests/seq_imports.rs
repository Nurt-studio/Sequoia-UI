#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
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

fn main_seq_import(import: &str, body: &str) -> String {
    format!(
        "import \"{import}\"\napp Demo {{\n  title: \"Demo\"\n  size: (400, 300)\n  window M {{\n{body}\n  }}\n}}\n"
    )
}

fn main_seq(body: &str) -> String {
    main_seq_import("./ui/card.seq", body)
}

const LIB: &str =
    "pub component Card(title: String) {\n  Column(padding: 4) { Text(text: title) }\n}\n";

fn err_of(file: &str, src: &str, files: &[(&str, &str)]) -> sequoia_seq::SeqError {
    match SeqLoader::load_str_map(file, src, files) {
        Ok(_) => panic!("oczekiwano bledu"),
        Err(e) => e,
    }
}

#[test]
fn pub_component_renders_across_files() {
    let loaded = SeqLoader::load_str_map(
        "Main.seq",
        &main_seq("    Card(title: \"AB\")"),
        &[("ui/card.seq", LIB)],
    )
    .unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    assert_eq!(glyphs(&h), 2);
}

#[test]
fn slot_and_nested_calls_work_across_files() {
    let lib = "pub component Card(title: String) {\n  Column {\n    Text(text: title)\n    children\n  }\n}\npub component Pair(a: String, b: String) {\n  Card(title: a) {\n    Card(title: b)\n  }\n}\n";
    let loaded = SeqLoader::load_str_map(
        "Main.seq",
        &main_seq("    Pair(a: \"A\", b: \"B\")"),
        &[("ui/card.seq", lib)],
    )
    .unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    assert_eq!(glyphs(&h), 2);
}

#[test]
fn transitive_pub_component_is_visible() {
    let a = "import \"./b.seq\"\npub component A(t: String) {\n  B(t: t)\n}\n";
    let b = "pub component B(t: String) {\n  Text(text: t)\n}\n";
    let loaded = SeqLoader::load_str_map(
        "Main.seq",
        &main_seq_import("./ui/a.seq", "    A(t: \"AB\")"),
        &[("ui/a.seq", a), ("ui/b.seq", b)],
    )
    .unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    assert_eq!(glyphs(&h), 2);
}

#[test]
fn diamond_import_loads_once() {
    let a = "import \"./c.seq\"\nimport \"./d.seq\"\npub component A(t: String) {\n  C(t: t)\n}\n";
    let c = "import \"./d.seq\"\npub component C(t: String) {\n  D(t: t)\n}\n";
    let d = "pub component D(t: String) {\n  Text(text: t)\n}\n";
    let loaded = SeqLoader::load_str_map(
        "Main.seq",
        &main_seq_import("./ui/a.seq", "    A(t: \"AB\")"),
        &[("ui/a.seq", a), ("ui/c.seq", c), ("ui/d.seq", d)],
    )
    .unwrap();
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    assert_eq!(glyphs(&h), 2, "D once, no duplicates");
}

#[test]
fn dynamic_prop_inside_imported_component() {
    let lib = "pub component Count(label: String) {\n  Text(text: label)\n}\n";
    let src = main_seq("    Count(label: dynamic(count))");
    let count = Signal::new("AB".to_string());
    let b = HostBindings::new().prop_str("count", count.clone());
    let loaded = SeqLoader::load_str_with_map("Main.seq", &src, &[("ui/card.seq", lib)], &b)
        .expect("landing with host");
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    assert_eq!(glyphs(&h), 2);
    count.set("ABCD".to_string());
    h.layout(400.0, 300.0);
    h.tick();
    assert_eq!(glyphs(&h), 4, "file subscription works");
}

#[test]
fn private_component_from_other_file_rejected() {
    let lib = "component Helper(t: String) {\n  Text(text: t)\n}\n";
    let src = main_seq("    Helper(t: \"x\")");
    let e = err_of("Main.seq", &src, &[("ui/card.seq", lib)]);
    assert!(e.message.contains("jest prywatny"), "{e}");
    assert!(e.message.contains("ui/card.seq"), "{e}");
    assert_eq!(e.file, "Main.seq");
    assert_eq!(e.line, 6);
}

#[test]
fn private_component_within_own_file_ok() {
    let lib = "component Helper(t: String) {\n  Text(text: t)\n}\npub component Card(t: String) {\n  Helper(t: t)\n}\n";
    SeqLoader::load_str_map(
        "Main.seq",
        &main_seq("    Card(t: \"AB\")"),
        &[("ui/card.seq", lib)],
    )
    .expect("private in your own file is ok");
}

#[test]
fn transitive_private_rejected() {
    let a = "import \"./b.seq\"\npub component A(t: String) {\n  Hidden(t: t)\n}\n";
    let b = "component Hidden(t: String) {\n  Text(text: t)\n}\n";
    let e = err_of(
        "Main.seq",
        &main_seq_import("./ui/a.seq", "    A(t: \"x\")"),
        &[("ui/a.seq", a), ("ui/b.seq", b)],
    );
    assert!(e.message.contains("jest prywatny"), "{e}");
    assert_eq!(e.file, "ui/a.seq");
    assert!(e.message.contains("ui/b.seq"), "{e}");
}

#[test]
fn duplicate_across_files_has_both_locations() {
    let src = "import \"./ui/a.seq\"\nimport \"./ui/b.seq\"\napp Demo {\n  title: \"Demo\"\n  size: (400, 300)\n  window M {\n    Card(t: \"x\")\n  }\n}\n";
    let e = err_of(
        "Main.seq",
        src,
        &[
            (
                "ui/a.seq",
                "pub component Card(t: String) {\n  Text(text: t)\n}\n",
            ),
            (
                "ui/b.seq",
                "pub component Card(t: String) {\n  Text(text: t)\n}\n",
            ),
        ],
    );
    assert!(e.message.contains("dwa razy"), "{e}");
    assert!(e.message.contains("ui/a.seq:1"), "{e}");
    assert_eq!(e.file, "ui/b.seq");
    assert_eq!(e.line, 1);
}

#[test]
fn missing_import_reports_import_line() {
    let src = main_seq("    Text(\"x\")");
    let e = err_of("Main.seq", &src, &[]);
    assert!(e.message.contains("import \"./ui/card.seq\""), "{e}");
    assert!(e.message.contains("brak pliku"), "{e}");
    assert_eq!(e.file, "Main.seq");
    assert_eq!(e.line, 1);
}

#[test]
fn import_cycle_rejected() {
    let a = "import \"./b.seq\"\npub component A(t: String) {\n  Text(text: t)\n}\n";
    let b = "import \"./a.seq\"\npub component B(t: String) {\n  Text(text: t)\n}\n";
    let e = err_of(
        "Main.seq",
        &main_seq_import("./ui/a.seq", "    A(t: \"x\")"),
        &[("ui/a.seq", a), ("ui/b.seq", b)],
    );
    assert!(e.message.contains("cykliczny import"), "{e}");
    assert!(
        e.message.contains("ui/a.seq -> ui/b.seq -> ui/a.seq"),
        "{e}"
    );
}

#[test]
fn self_import_rejected() {
    let src =
        main_seq("    Text(\"x\")").replace("import \"./ui/card.seq\"", "import \"./Main.seq\"");
    let e = err_of("Main.seq", &src, &[("Main.seq", &src)]);
    assert!(e.message.contains("cykliczny import"), "{e}");
}

#[test]
fn imported_file_with_app_rejected() {
    let lib =
        "app Other {\n  title: \"O\"\n  size: (10, 10)\n  window M {\n    Text(\"x\")\n  }\n}\n";
    let e = err_of(
        "Main.seq",
        &main_seq("    Text(\"x\")"),
        &[("ui/card.seq", lib)],
    );
    assert!(e.message.contains("cannot contain `app`"), "{e}");
    assert_eq!(e.line, 1);
}

#[test]
fn parse_error_inside_imported_file_cites_that_file() {
    let lib = "pub component Card(t: String) {\n  Nope(x: 1)\n}\n";
    let e = err_of(
        "Main.seq",
        &main_seq("    Text(\"x\")"),
        &[("ui/card.seq", lib)],
    );
    assert_eq!(e.file, "ui/card.seq");
    assert!(e.message.contains("unknown component"), "{e}");
}

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("seq_imports_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("ui")).unwrap();
    dir
}

#[test]
fn load_file_resolves_imports_from_disk() {
    let dir = tmp_dir("fs");
    std::fs::write(dir.join("Main.seq"), main_seq("    Card(title: \"AB\")")).unwrap();
    std::fs::write(dir.join("ui/card.seq"), LIB).unwrap();
    let path = dir.join("Main.seq");
    let loaded = SeqLoader::load_file(path.to_str().unwrap()).expect("loads from disk");
    assert_eq!(loaded.files.len(), 2, "main + import to watch");
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    assert_eq!(glyphs(&h), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parent_dir_import_works() {
    let dir = tmp_dir("parent");
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(
        dir.join("app/Main.seq"),
        "import \"../ui/card.seq\"\napp Demo {\n  title: \"Demo\"\n  size: (400, 300)\n  window M {\n    Card(title: \"AB\")\n  }\n}\n",
    )
    .unwrap();
    std::fs::write(dir.join("ui/card.seq"), LIB).unwrap();
    let path = dir.join("app/Main.seq");
    let loaded = SeqLoader::load_file(path.to_str().unwrap()).expect("`../` works");
    let h = Harness::new(loaded.view);
    h.layout(400.0, 300.0);
    assert_eq!(glyphs(&h), 2);
    let _ = std::fs::remove_dir_all(&dir);
}
