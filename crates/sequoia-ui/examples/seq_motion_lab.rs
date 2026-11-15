//! A tour of the 0.1.6 `.seq` additions: reactive attribute expressions,
//! `enter:`/`exit:` motion chains, spring behavior and typed `include_seq!`
//! bindings.
//!
//! The `.seq` file declares `prop heading: String`, `prop level: Float`,
//! `prop open: Bool`, `on toggle()` and `on nudge()`. The macro turns each of
//! them into a typed builder method, so a wrong name or signature fails the
//! build instead of `load()`.
//!
//! Run it:
//!     cargo run --example seq_motion_lab -p sequoia-ui

use sequoia_ui::include_seq;
use sequoia_ui::prelude::*;

fn main() {
    let heading = Signal::new("Seq Motion Lab".to_string());
    let level = Signal::new(0.45f64);
    let open = Signal::new(true);

    let toggle_open = open.clone();
    let bump_level = level.clone();

    let loaded = include_seq!("examples/seq_motion_lab.seq")
        .prop_heading(heading)
        .prop_level(level)
        .prop_open(open)
        .on_toggle(move || toggle_open.set(!toggle_open.get_untracked()))
        .on_nudge(move || {
            let next = bump_level.get_untracked() + 0.2;
            bump_level.set(if next > 1.0 { 0.2 } else { next });
        })
        .load()
        .expect("invalid examples/seq_motion_lab.seq");

    App::new()
        .title(loaded.meta.title)
        .size(loaded.meta.size.0, loaded.meta.size.1)
        .theme(loaded.meta.theme)
        .root(loaded.view)
        .run()
        .expect("event loop failed");
}
