//! Scene preparation between the paint list and GPU draw calls.
//!
//! Paint items are converted to instance data, split into contiguous draw runs,
//! and planned together with backdrop markers and filter ranges.

pub mod grouping;
pub mod instances;
pub mod planning;
