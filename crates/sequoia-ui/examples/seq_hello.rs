use sequoia_ui::prelude::*;

const APP: &str = r#"
app SeqHello {
  title: "Seq Hello"
  size: (460, 280)
  theme: dark
  on greet()
  window Main {
    Column(padding: 24, gap: 12) {
      Text("Hello from Seq", size: 26, bold: true)
      Text("This window is described in the .seq language.")
      Button("Say hello", on_click: { greet() })
    }
  }
}
"#;

fn main() {
    let bindings = HostBindings::new().on("greet", || println!("hello from Rust"));
    let loaded =
        SeqLoader::load_str_with("SeqHello.seq", APP, &bindings).expect("invalid .seq source");

    App::new()
        .title(loaded.meta.title)
        .size(loaded.meta.size.0, loaded.meta.size.1)
        .theme(loaded.meta.theme)
        .root(loaded.view)
        .run()
        .expect("failed to run the seq example");
}
