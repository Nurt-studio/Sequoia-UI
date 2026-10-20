//! Simple counter demo.
//!
//! Build & run with:
//!     cargo run --example counter -p sequoia-ui

use sequoia_ui::prelude::*;

fn main() {
    // The counter lives in a reactive `Signal`. Every widget that reads it
    // will automatically re-render when the value changes — no manual
    // state management is required.
    let count = Signal::new(0i32);

    // Each button callback captures its own clone of the signal. All
    // clones point at the same underlying cell, so any click updates a
    // single shared value.
    let inc = count.clone();
    let dec = count.clone();
    let reset = count.clone();
    let shown = count.clone();

    // `Column` lays out children vertically; `center` centers them on both
    // axes, `fill_size` stretches the column to the whole window, and `gap`
    // inserts 16 px between children.
    let view = Column::new()
        .center()
        .gap(16.0)
        .fill_size()
        .padding(24.0)
        .child(
            // `Text::with` builds the label inside a closure, so the widget
            // subscribes to the signal and refreshes on every change.
            Text::with(move || format!("{}", shown.get()))
                .size(72.0)
                .bold(),
        )
        .child(
            Row::new()
                .center()
                .gap(8.0)
                .child(
                    Button::labeled("− 1")
                        .style(ButtonStyle::SECONDARY)
                        .on_click(move || dec.update(|v| *v -= 1)),
                )
                .child(
                    Button::labeled("Reset")
                        .style(ButtonStyle::SECONDARY)
                        .on_click(move || reset.set(0)),
                )
                .child(
                    Button::labeled("+ 1")
                        .style(ButtonStyle::PRIMARY)
                        .on_click(move || inc.update(|v| *v += 1)),
                ),
        );

    // `App` creates the window, mounts the view tree, and starts the event
    // loop on the current thread.
    App::new()
        .title("Counter")
        .size(420.0, 280.0)
        .root(view)
        .run()
        .expect("event loop failed");
}
