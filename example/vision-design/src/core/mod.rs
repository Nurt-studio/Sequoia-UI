//! Editor domain layer: model, geometry, hit testing and snapping.
//!
//! This module does not know `sequoia-ui` beyond value types (`Color`, `Point`).

pub(crate) mod geometry;
pub(crate) mod hit_test;
pub(crate) mod layer;
pub(crate) mod snap;
