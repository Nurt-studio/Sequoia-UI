#![allow(clippy::unwrap_used, clippy::expect_used)]

use sequoia_reactive::Signal;
use sequoia_seq::{HostBindings, SeqLoader};
use sequoia_test::Harness;

#[test]
fn anim_wrappers_lower_and_paint() {
    let src = r#"
app A {
  title: "A"
  size: (100, 100)
  prop open: Bool
  prop zoom: Float
  window M {
    If(when: dynamic(open), enter: smooth, exit: smooth) {
      Scale(value: dynamic(zoom), animate: fast) {
        Text("hi")
      }
    }
    Width(value: 50) { Text("w") }
    Height(value: 20) { Text("h") }
    Opacity(value: 0.5) { Text("x") }
    Rotate(value: 45) { Text("r") }
    Translate(x: 10, y: 20) { Text("t") }
    Pulse(min: 0.2, period: 1.0) { Text("p") }
    Shake(amplitude: 4, period: 0.1, duration: 0.2) { Text("s") }
  }
}
"#;
    let zoom = Signal::new(1.0f64);
    let open = Signal::new(true);
    let b = HostBindings::new()
        .prop_bool("open", open)
        .prop_float("zoom", zoom);
    let loaded = SeqLoader::load_str_with("A.seq", src, &b).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(100.0, 100.0);
    let list = h.paint();
    assert!(!list.is_empty());
}

#[test]
fn anim_wrappers_static_lower() {
    let src = r#"
app A {
  title: "A"
  size: (10, 10)
  window M {
    Width(value: 50) { Text("w") }
    Height(value: 20) { Text("h") }
    Opacity(value: 0.5) { Text("x") }
    Scale(value: 2) { Text("s") }
    Rotate(value: 90) { Text("r") }
    Translate(x: 5, y: 5) { Text("t") }
    Pulse() { Text("p") }
    Shake() { Text("s") }
  }
}
"#;
    let loaded = SeqLoader::load_str("A.seq", src).unwrap();
    let h = Harness::new(loaded.view);
    h.layout(10.0, 10.0);
    let list = h.paint();
    assert!(!list.is_empty());
}
