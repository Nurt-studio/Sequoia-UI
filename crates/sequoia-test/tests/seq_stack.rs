#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};

fn deep_app(depth: usize) -> String {
    let mut src = String::from(
        "app Deep {\n  title: \"Deep\"\n  size: (400, 300)\n  prop label: String\n  window Main {\n",
    );
    for level in 0..depth {
        src.push_str(&format!(
            "{indent}Container(width: px({}), height: px({}), padding: 1) {{\n",
            400 - level,
            300 - level,
            indent = "  ".repeat(level + 2)
        ));
    }
    src.push_str(&format!("{}Text(dynamic: label)\n", "  ".repeat(depth + 2)));
    for level in (0..depth).rev() {
        src.push_str(&format!("{}}}\n", "  ".repeat(level + 2)));
    }
    src.push_str("  }\n}\n");
    src
}

fn bindings() -> HostBindings {
    HostBindings::new().prop_str("label", Signal::new("x".to_string()))
}

#[test]
fn deep_lowering_fits_1mb_stack() {
    let src = deep_app(12);
    let ok = std::thread::Builder::new()
        .name("deep-lowering".to_string())
        .stack_size(1024 * 1024)
        .spawn(move || SeqLoader::load_str_with("Deep.seq", &src, &bindings()).is_ok())
        .unwrap()
        .join()
        .unwrap();
    assert!(ok, "12 nested Containers must lower on a 1 MB stack");
}

#[test]
fn lowering_depth_limit_is_reported() {
    let src = deep_app(70);
    let message = std::thread::Builder::new()
        .name("depth-limit".to_string())
        .stack_size(16 * 1024 * 1024)
        .spawn(
            move || match SeqLoader::load_str_with("Deep.seq", &src, &bindings()) {
                Ok(_) => None,
                Err(err) => Some(err.message),
            },
        )
        .unwrap()
        .join()
        .unwrap();
    let message = message.expect("expected the lowering depth limit to reject 70 levels");
    assert!(
        message.contains("lowering nesting limit exceeded"),
        "{message}"
    );
}
