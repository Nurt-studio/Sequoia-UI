//! Flex grow, shrink, and justification along a container's main axis.

use crate::container::Justify;

/// Flex parameters for one item along a container's main axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexItem {
    /// Base size before growing or shrinking.
    pub base: f32,
    /// Relative weight used to claim free space.
    pub grow: f32,
    /// Relative weight used to absorb a deficit.
    pub shrink: f32,
    /// Minimum size the item may shrink to.
    pub min: f32,
    /// Maximum size the item may grow to.
    pub max: f32,
}

impl FlexItem {
    /// Creates a sanitized flex item.
    ///
    /// Non-finite or negative values are clamped: `base` and `min` to zero,
    /// `grow` to zero, `shrink` to one, and `max` becomes infinity when it is
    /// NaN or positive infinity.
    pub fn new(base: f32, grow: f32, shrink: f32, min: f32, max: f32) -> Self {
        Self {
            base: if base.is_finite() { base.max(0.0) } else { 0.0 },
            grow: if grow.is_finite() && grow > 0.0 {
                grow
            } else {
                0.0
            },
            shrink: if shrink.is_finite() && shrink >= 0.0 {
                shrink
            } else {
                1.0
            },
            min: if min.is_finite() { min.max(0.0) } else { 0.0 },
            max: if max.is_nan() {
                f32::INFINITY
            } else if max.is_infinite() {
                if max > 0.0 { f32::INFINITY } else { 0.0 }
            } else {
                max.max(0.0)
            },
        }
    }

    fn clamp(self, v: f32) -> f32 {
        let (mn, mx) = if self.min > self.max {
            (self.max, self.max)
        } else {
            (self.min, self.max)
        };
        v.clamp(mn, mx)
    }
}

/// Distributes `avail_main` among `sizes` according to the flex items.
///
/// `sizes` initially holds the base sizes and is updated in place. `gaps` is
/// the total space consumed by gaps between items. Items grow until their max
/// and shrink until their min; any remaining space is passed to siblings.
pub fn distribute_flex(items: &[FlexItem], sizes: &mut [f32], gaps: f32, avail_main: f32) {
    assert_eq!(items.len(), sizes.len());
    if items.is_empty() {
        return;
    }
    for (item, size) in items.iter().zip(sizes.iter_mut()) {
        if !size.is_finite() {
            *size = 0.0;
        }
        *size = item.clamp(*size);
    }
    if !avail_main.is_finite() || avail_main.is_nan() {
        return;
    }
    let gaps = if gaps.is_finite() { gaps.max(0.0) } else { 0.0 };
    const EPS: f32 = 0.001;

    for _ in 0..items.len() + 1 {
        let used: f32 = sizes.iter().sum::<f32>() + gaps;
        let free = avail_main - used;
        if free.abs() <= EPS || free.is_nan() {
            break;
        }
        if free > 0.0 {
            let mut sum_grow = 0.0;
            for (item, size) in items.iter().zip(sizes.iter()) {
                if item.grow > 0.0 && *size < item.max - EPS {
                    sum_grow += item.grow;
                }
            }
            if !(sum_grow > 0.0) {
                break;
            }
            for (item, size) in items.iter().zip(sizes.iter_mut()) {
                if item.grow > 0.0 && *size < item.max - EPS {
                    *size = item.clamp(*size + free * item.grow / sum_grow);
                }
            }
        } else {
            let mut deficit = -free;
            for pass in [true, false] {
                if deficit <= EPS {
                    break;
                }
                let mut pool = 0.0;
                for (item, size) in items.iter().zip(sizes.iter()) {
                    let is_flex = item.grow > 0.0;
                    if is_flex == pass && *size > item.min + EPS {
                        pool += item.shrink * *size;
                    }
                }

                if !pass {
                    let flex_ready = items
                        .iter()
                        .zip(sizes.iter())
                        .any(|(it, s)| it.grow > 0.0 && *s > it.min + EPS);
                    if flex_ready {
                        continue;
                    }
                }
                if !(pool > 0.0) {
                    continue;
                }
                let take = pool.min(deficit * items_scale(items, sizes, pass));

                for (item, size) in items.iter().zip(sizes.iter_mut()) {
                    let is_flex = item.grow > 0.0;
                    if is_flex == pass && *size > item.min + EPS {
                        let w = item.shrink * *size;
                        let cut = take * w / pool;
                        *size = item.clamp(*size - cut);
                    }
                }
                let used2: f32 = sizes.iter().sum::<f32>() + gaps;
                deficit = (used2 - avail_main).max(0.0);
            }
            // A clamped item changed the pool, so the next outer pass can still
            // absorb a remaining deficit on siblings that are above their min.
        }
    }
}

fn items_scale(_items: &[FlexItem], _sizes: &[f32], _flex_pass: bool) -> f32 {
    1.0
}

/// Computes the leading offset and inter-item gap for a justified line.
///
/// Returns `(start_offset, added_gap)`; `used` is the space already occupied
/// by items and `avail` is the available main-axis space.
pub fn justify_layout(
    justify: Justify,
    count: usize,
    base_gap: f32,
    used: f32,
    avail: f32,
) -> (f32, f32) {
    let extra = if avail.is_finite() {
        (avail - used).max(0.0)
    } else {
        0.0
    };
    let _ = base_gap;
    justify.distribute(count, base_gap, extra)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.05
    }

    #[test]
    fn grow_splits_free_space() {
        let items = [
            FlexItem::new(100.0, 1.0, 1.0, 0.0, f32::INFINITY),
            FlexItem::new(50.0, 3.0, 1.0, 0.0, f32::INFINITY),
        ];
        let mut sizes = [100.0, 50.0];
        distribute_flex(&items, &mut sizes, 0.0, 400.0);
        assert!(approx(sizes[0] + sizes[1], 400.0));
        assert!(approx(sizes[0], 162.5));
        assert!(approx(sizes[1], 237.5));
    }

    #[test]
    fn max_cap_overflow_goes_to_sibling() {
        let items = [
            FlexItem::new(0.0, 1.0, 1.0, 0.0, 200.0),
            FlexItem::new(0.0, 1.0, 1.0, 0.0, f32::INFINITY),
        ];
        let mut sizes = [0.0, 0.0];
        distribute_flex(&items, &mut sizes, 0.0, 400.0);
        assert!(approx(sizes[0], 200.0), "{sizes:?}");
        assert!(approx(sizes[1], 200.0), "{sizes:?}");
    }

    #[test]
    fn shrink_prefers_scaled_weight() {
        let items = [
            FlexItem::new(100.0, 1.0, 1.0, 0.0, f32::INFINITY),
            FlexItem::new(100.0, 1.0, 1.0, 0.0, f32::INFINITY),
        ];
        let mut sizes = [100.0, 100.0];
        distribute_flex(&items, &mut sizes, 0.0, 120.0);
        assert!(approx(sizes[0] + sizes[1], 120.0), "{sizes:?}");
    }

    #[test]
    fn zero_shrink_item_keeps_size() {
        let items = [
            FlexItem::new(100.0, 0.0, 0.0, 0.0, f32::INFINITY),
            FlexItem::new(100.0, 1.0, 1.0, 0.0, f32::INFINITY),
        ];
        let mut sizes = [100.0, 100.0];
        distribute_flex(&items, &mut sizes, 0.0, 120.0);
        assert!(approx(sizes[0], 100.0), "{sizes:?}");
        assert!(approx(sizes[1], 20.0), "{sizes:?}");
    }
}
