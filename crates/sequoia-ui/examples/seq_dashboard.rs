//! A declarative `.seq` dashboard with no animation: components, reactive
//! attribute expressions, two-way fields, and typed `include_seq!` bindings.
//!
//! The `.seq` file declares four props and two callbacks. The macro turns each
//! one into a typed builder method, so a typo or a wrong signature fails the
//! build instead of `load()`.
//!
//! Run it:
//!     cargo run --example seq_dashboard -p sequoia-ui

use sequoia_ui::include_seq;
use sequoia_ui::prelude::*;

fn main() {
    let title = Signal::new("Release board".to_string());
    let owner = Signal::new("Ada".to_string());
    let coverage = Signal::new(0.72f64);
    let stable = Signal::new(true);

    let bump = coverage.clone();
    let reset_coverage = coverage.clone();
    let reset_owner = owner.clone();
    let reset_stable = stable.clone();

    let loaded = include_seq!("examples/seq_dashboard.seq")
        .prop_title(title)
        .prop_owner(owner)
        .prop_coverage(coverage)
        .prop_stable(stable)
        .on_refresh(move || {
            let next = bump.get_untracked() + 0.05;
            bump.set(if next > 1.0 { 0.72 } else { next });
        })
        .on_reset(move || {
            reset_coverage.set(0.72);
            reset_owner.set("Ada".to_string());
            reset_stable.set(true);
        })
        .load()
        .expect("invalid examples/seq_dashboard.seq");

    App::new()
        .title(loaded.meta.title)
        .size(loaded.meta.size.0, loaded.meta.size.1)
        .theme(loaded.meta.theme)
        .root(loaded.view)
        .run()
        .expect("event loop failed");
}
