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
#![doc = include_str!("../../src/contributing/documentation.md")]
