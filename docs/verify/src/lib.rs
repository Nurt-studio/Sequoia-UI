//! Compiles the Rust examples from the Sequoia book.
//!
//! This crate exists only to run the book's code blocks as doctests with the
//! workspace dependencies available. Run it with:
//!
//! ```text
//! cargo test -p sequoia-docs-verify --doc
//! ```

#![doc = include_str!("../../src/introduction.md")]
#![doc = include_str!("../../src/getting-started/installation.md")]
#![doc = include_str!("../../src/getting-started/first-app.md")]
#![doc = include_str!("../../src/getting-started/project-structure.md")]
#![doc = include_str!("../../src/getting-started/examples.md")]
#![doc = include_str!("../../src/concepts/reactivity.md")]
#![doc = include_str!("../../src/concepts/app-lifecycle.md")]
#![doc = include_str!("../../src/concepts/view-tree.md")]
#![doc = include_str!("../../src/concepts/layout.md")]
#![doc = include_str!("../../src/concepts/events.md")]
#![doc = include_str!("../../src/concepts/styling.md")]
#![doc = include_str!("../../src/concepts/rendering.md")]
#![doc = include_str!("../../src/concepts/text.md")]
#![doc = include_str!("../../src/concepts/animation.md")]
#![doc = include_str!("../../src/concepts/async.md")]
#![doc = include_str!("../../src/concepts/accessibility.md")]
#![doc = include_str!("../../src/guides/widgets.md")]
#![doc = include_str!("../../src/guides/inputs.md")]
#![doc = include_str!("../../src/guides/hotkeys.md")]
#![doc = include_str!("../../src/guides/overlays.md")]
#![doc = include_str!("../../src/guides/collections.md")]
#![doc = include_str!("../../src/guides/scrolling.md")]
#![doc = include_str!("../../src/guides/drag-and-drop.md")]
#![doc = include_str!("../../src/guides/docking.md")]
#![doc = include_str!("../../src/guides/canvas.md")]
#![doc = include_str!("../../src/guides/assets.md")]
#![doc = include_str!("../../src/guides/seq.md")]
#![doc = include_str!("../../src/guides/localization.md")]
#![doc = include_str!("../../src/guides/desktop-integration.md")]
#![doc = include_str!("../../src/guides/testing.md")]
#![doc = include_str!("../../src/guides/export.md")]
#![doc = include_str!("../../src/guides/custom-views.md")]
#![doc = include_str!("../../src/contributing/documentation.md")]
