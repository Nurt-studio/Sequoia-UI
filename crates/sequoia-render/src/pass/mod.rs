//! Individual render passes used to compose a frame.
//!
//! Backdrops and filters are rendered in separate passes so they can sample the
//! scene that has already been drawn, while GUI content is recorded by the
//! shared run-drawing helper.

pub mod backdrop;
pub mod filter;
pub mod gui;
pub mod opacity;
