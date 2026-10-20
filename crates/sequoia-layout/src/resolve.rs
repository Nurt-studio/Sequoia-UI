use sequoia_core::{Constraints, Length, Size};

pub fn resolve_main(len: Length, intrinsic: f32, available: f32) -> (f32, f32) {
    match len {
        Length::Px(v) => (v, 0.0),
        Length::Percent(p) => {
            if available.is_finite() && p.is_finite() {
                ((available * p / 100.0).max(0.0), 0.0)
            } else {
                (intrinsic, 0.0)
            }
        }
        Length::Fill(w) => (intrinsic, w.max(0.0)),
        Length::Fit => (intrinsic, 0.0),
    }
}

pub fn resolve_cross(len: Length, intrinsic: f32, available: f32) -> f32 {
    match len {
        Length::Px(v) => v,
        Length::Percent(p) => {
            if available.is_finite() && p.is_finite() {
                (available * p / 100.0).max(0.0)
            } else {
                intrinsic
            }
        }
        Length::Fill(_) => {
            if available.is_finite() {
                available
            } else {
                intrinsic
            }
        }
        Length::Fit => intrinsic,
    }
}

pub fn resolve_flex_basis(
    basis: Option<Length>,
    fallback: (f32, f32),
    intrinsic: f32,
    available: f32,
) -> (f32, f32) {
    let Some(b) = basis else {
        return fallback;
    };
    match b {
        Length::Px(v) => (v.max(0.0), 0.0),
        Length::Percent(p) => {
            if available.is_finite() && p.is_finite() {
                ((available * p / 100.0).max(0.0), 0.0)
            } else {
                fallback
            }
        }
        Length::Fill(w) => (intrinsic, w.max(0.0)),
        Length::Fit => (intrinsic, 0.0),
    }
}

pub fn clamp_to(size: Size, constraints: Constraints) -> Size {
    constraints.constrain(size)
}

pub fn distribute_main(bases: &mut [f32], grows: &[f32], free: f32) {
    debug_assert_eq!(bases.len(), grows.len());

    if bases.is_empty() || free == 0.0 || free.is_nan() {
        return;
    }

    let sum_grow: f32 = grows.iter().copied().sum();

    if free > 0.0 {
        if sum_grow > 0.0 {
            for (i, base) in bases.iter_mut().enumerate() {
                if grows[i] > 0.0 {
                    *base += free * grows[i] / sum_grow;
                }
            }
        }
        return;
    }

    let mut deficit = -free;

    let flex_sum: f32 = bases
        .iter()
        .zip(grows)
        .filter(|(_, g)| **g > 0.0)
        .map(|(b, _)| *b)
        .sum();
    if flex_sum > 0.0 {
        let take = flex_sum.min(deficit);
        let k = 1.0 - take / flex_sum;
        for (i, base) in bases.iter_mut().enumerate() {
            if grows[i] > 0.0 {
                *base *= k;
            }
        }
        deficit -= take;
    }

    if deficit > 0.0 {
        let fixed_sum: f32 = bases
            .iter()
            .zip(grows)
            .filter(|(_, g)| **g <= 0.0)
            .map(|(b, _)| *b)
            .sum();
        if fixed_sum > 0.0 {
            let k = ((fixed_sum - deficit) / fixed_sum).max(0.0);
            for (i, base) in bases.iter_mut().enumerate() {
                if grows[i] <= 0.0 {
                    *base *= k;
                }
            }
        }
    }
}

const CLAMP_EPS: f32 = 0.001;

pub fn distribute_main_clamped(
    bases: &mut [f32],
    grows: &[f32],
    mins: &[f32],
    maxs: &[f32],
    gaps: f32,
    avail_main: f32,
) {
    debug_assert_eq!(bases.len(), grows.len());
    if bases.is_empty() {
        return;
    }

    let limit = |i: usize| -> (f32, f32) {
        let mn = mins.get(i).copied().unwrap_or(0.0);
        let mx = maxs.get(i).copied().unwrap_or(f32::INFINITY);
        let mn = if mn.is_finite() { mn.max(0.0) } else { 0.0 };
        let mx = if mx.is_nan() {
            f32::INFINITY
        } else if mx.is_infinite() {
            if mx > 0.0 { f32::INFINITY } else { 0.0 }
        } else {
            mx.max(0.0)
        };
        if mn > mx { (mx, mx) } else { (mn, mx) }
    };
    let clamp_one = |i: usize, v: f32| -> f32 {
        let (mn, mx) = limit(i);
        v.clamp(mn, mx)
    };
    for (i, base) in bases.iter_mut().enumerate() {
        *base = clamp_one(i, *base);
    }
    if !avail_main.is_finite() || avail_main.is_nan() {
        return;
    }
    let gaps = if gaps.is_finite() { gaps.max(0.0) } else { 0.0 };

    for _ in 0..bases.len() + 1 {
        let used: f32 = bases.iter().sum::<f32>() + gaps;
        let free = avail_main - used;
        if free.abs() <= CLAMP_EPS || free.is_nan() {
            break;
        }
        if free > 0.0 {
            let grow_of = |i: usize| grows.get(i).copied().unwrap_or(0.0);
            let mut sum_grow = 0.0;
            for (i, base) in bases.iter().enumerate() {
                let (_, mx) = limit(i);
                if is_finite_positive(grow_of(i)) && *base < mx - CLAMP_EPS {
                    sum_grow += grow_of(i);
                }
            }
            if !(sum_grow > 0.0) {
                break;
            }
            for (i, base) in bases.iter_mut().enumerate() {
                let (_, mx) = limit(i);
                if is_finite_positive(grow_of(i)) && *base < mx - CLAMP_EPS {
                    *base = clamp_one(i, *base + free * grow_of(i) / sum_grow);
                }
            }
        } else {
            let grow_of = |i: usize| grows.get(i).copied().unwrap_or(0.0);
            let flex_ready = bases
                .iter()
                .enumerate()
                .any(|(i, b)| grow_of(i) > 0.0 && *b > limit(i).0 + CLAMP_EPS);

            let mut pool_sum = 0.0;
            for (i, base) in bases.iter().enumerate() {
                let (mn, _) = limit(i);
                let is_flex = grow_of(i) > 0.0;
                if *base > mn + CLAMP_EPS && (is_flex || !flex_ready) {
                    pool_sum += *base;
                }
            }
            if !(pool_sum > 0.0) {
                break;
            }
            let mut deficit = -free;
            let take = pool_sum.min(deficit);
            let k = 1.0 - take / pool_sum;
            for (i, base) in bases.iter_mut().enumerate() {
                let (mn, _) = limit(i);
                let is_flex = grow_of(i) > 0.0;
                if *base > mn + CLAMP_EPS && (is_flex || !flex_ready) {
                    *base = clamp_one(i, *base * k);
                }
            }
            deficit -= take;
            if deficit <= CLAMP_EPS {
                continue;
            }
        }
    }
}

fn is_finite_positive(v: f32) -> bool {
    v.is_finite() && v > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn clamped_matches_plain_without_limits() {
        let mut a = vec![100.0, 50.0];
        let mut b = a.clone();
        distribute_main(&mut a, &[1.0, 3.0], 250.0);
        distribute_main_clamped(&mut b, &[1.0, 3.0], &[], &[], 0.0, 400.0);
        assert!(approx(a[0], b[0]) && approx(a[1], b[1]));
        assert!(approx(b[0] + b[1], 400.0));
    }

    #[test]
    fn leftover_after_max_goes_to_siblings() {
        let mut bases = vec![0.0, 0.0];
        distribute_main_clamped(
            &mut bases,
            &[1.0, 1.0],
            &[],
            &[200.0, f32::INFINITY],
            0.0,
            400.0,
        );
        assert!(approx(bases[0], 200.0), "{bases:?}");
        assert!(approx(bases[1], 200.0), "{bases:?}");
    }

    #[test]
    fn deficit_stops_at_min_and_overflows() {
        let mut bases = vec![60.0, 60.0];
        distribute_main_clamped(&mut bases, &[0.0, 0.0], &[60.0, 0.0], &[], 0.0, 100.0);
        assert!(approx(bases[0], 60.0), "{bases:?}");
        assert!(approx(bases[1], 40.0), "{bases:?}");

        let mut bases = vec![60.0, 60.0];
        distribute_main_clamped(&mut bases, &[0.0, 0.0], &[60.0, 60.0], &[], 0.0, 100.0);
        assert!(approx(bases[0], 60.0), "{bases:?}");
        assert!(approx(bases[1], 60.0), "{bases:?}");
    }

    #[test]
    fn unbounded_only_clamps() {
        let mut bases = vec![500.0, 10.0];
        distribute_main_clamped(
            &mut bases,
            &[1.0, 1.0],
            &[],
            &[200.0, f32::INFINITY],
            0.0,
            f32::INFINITY,
        );
        assert!(approx(bases[0], 200.0), "{bases:?}");
        assert!(approx(bases[1], 10.0), "{bases:?}");
    }
}
