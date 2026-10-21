#[allow(unused_imports)]
use std::any::Any;

use crate::types::*;
use sequoia_core::{BackdropSpan, FilterSpan};

/// Merges backdrop markers and filter ranges into ordered compositing steps.
///
/// A filter consumes the items in its range; markers or nested filters that
/// fall inside an already consumed range are skipped.
pub(crate) fn plan_composite_ops(
    backdrops: &[BackdropSpan],
    filters: &[FilterSpan],
    items_len: usize,
) -> Vec<CompOp> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Marker,
        Filter,
    }
    let mut events: Vec<(usize, Kind, usize)> = Vec::new();
    for (i, span) in backdrops.iter().enumerate() {
        events.push((span.at.min(items_len), Kind::Marker, i));
    }
    for (i, f) in filters.iter().enumerate() {
        events.push((f.start.min(items_len), Kind::Filter, i));
    }
    events.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| match (a.1, b.1) {
            (Kind::Marker, Kind::Filter) => std::cmp::Ordering::Less,
            (Kind::Filter, Kind::Marker) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        })
    });
    let mut ops = Vec::new();
    let mut consumed_end = 0usize;
    for (pos, kind, idx) in events {
        if pos < consumed_end {
            continue;
        }
        match kind {
            Kind::Marker => ops.push(CompOp::Marker {
                at: pos,
                span_idx: idx,
            }),
            Kind::Filter => {
                let end = filters[idx].end.min(items_len);
                if end > pos {
                    ops.push(CompOp::FilterRange {
                        start: pos,
                        end,
                        span_idx: idx,
                    });
                    consumed_end = end;
                }
            }
        }
    }
    ops
}
