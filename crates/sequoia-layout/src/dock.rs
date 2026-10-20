use sequoia_core::{Length, Rect};

use crate::container::Axis;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dock {
    Left,

    Right,

    Top,

    Bottom,

    #[default]
    Fill,
}

impl Dock {
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    pub const fn is_side(self) -> bool {
        !matches!(self, Self::Fill)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DockStyle {
    pub docks: Vec<Dock>,

    pub gap: f32,

    pub last_child_fill: bool,

    pub ordered: bool,
}

impl Default for DockStyle {
    fn default() -> Self {
        Self {
            docks: Vec::new(),
            gap: 0.0,
            last_child_fill: false,
            ordered: false,
        }
    }
}

impl DockStyle {
    pub fn new(docks: impl IntoIterator<Item = Dock>) -> Self {
        Self {
            docks: docks.into_iter().collect(),
            gap: 0.0,
            last_child_fill: false,
            ordered: false,
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = sanitize_gap(gap);
        self
    }

    pub fn last_child_fill(mut self, fill: bool) -> Self {
        self.last_child_fill = fill;
        self
    }

    pub fn ordered(mut self, ordered: bool) -> Self {
        self.ordered = ordered;
        self
    }

    pub fn border(mut self, border: bool) -> Self {
        self.ordered = !border;
        self
    }
}

pub fn sanitize_gap(gap: f32) -> f32 {
    if gap.is_finite() { gap.max(0.0) } else { 0.0 }
}

pub fn dock_thickness(len: Length, intrinsic: f32, available: f32) -> f32 {
    let intrinsic = if intrinsic.is_finite() {
        intrinsic.max(0.0)
    } else {
        0.0
    };
    match len {
        Length::Px(v) => {
            if v.is_finite() {
                v.max(0.0)
            } else {
                intrinsic
            }
        }
        Length::Percent(p) => {
            if available.is_finite() && p.is_finite() {
                (available * p / 100.0).max(0.0)
            } else {
                intrinsic
            }
        }
        Length::Fill(_) | Length::Fit => intrinsic,
    }
}

pub fn effective_dock(docks: &[Dock], last_child_fill: bool, index: usize, count: usize) -> Dock {
    if count > 0 && last_child_fill && index + 1 == count {
        return Dock::Fill;
    }
    docks.get(index).copied().unwrap_or(Dock::Fill)
}

pub fn resolve_dock(
    docks: &[Dock],
    last_child_fill: bool,
    ordered: bool,
    index: usize,
    count: usize,
) -> Dock {
    if ordered {
        effective_dock(docks, last_child_fill, index, count)
    } else {
        docks.get(index).copied().unwrap_or(Dock::Fill)
    }
}

pub fn dock_layout_order(
    docks: &[Dock],
    last_child_fill: bool,
    ordered: bool,
    count: usize,
) -> Vec<usize> {
    let mut order: Vec<usize> = (0..count).collect();
    if ordered || count == 0 {
        return order;
    }
    order.sort_by_key(|&i| match docks.get(i).copied().unwrap_or(Dock::Fill) {
        Dock::Top | Dock::Bottom => 0,
        Dock::Left | Dock::Right => 1,
        Dock::Fill => 2,
    });
    let _ = last_child_fill;
    order
}

pub fn sanitize_frac(frac: f32) -> f32 {
    if frac.is_finite() {
        frac.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

pub fn sanitize_preview_frac(frac: f32) -> f32 {
    if frac.is_finite() {
        frac.clamp(0.15, 0.85)
    } else {
        0.5
    }
}

pub fn split_axis_for_dock(dock: Dock) -> Option<(Axis, bool)> {
    match dock {
        Dock::Left => Some((Axis::Horizontal, true)),
        Dock::Right => Some((Axis::Horizontal, false)),
        Dock::Top => Some((Axis::Vertical, true)),
        Dock::Bottom => Some((Axis::Vertical, false)),
        Dock::Fill => None,
    }
}

pub fn dock_preview_rect(host: Rect, dock: Dock, frac: f32) -> Rect {
    let f = sanitize_preview_frac(frac);
    let x = host.origin.x;
    let y = host.origin.y;
    let w = host.size.width.max(0.0);
    let h = host.size.height.max(0.0);
    match dock {
        Dock::Left => Rect::from_xywh(x, y, w * f, h),
        Dock::Right => Rect::from_xywh(x + w * (1.0 - f), y, w * f, h),
        Dock::Top => Rect::from_xywh(x, y, w, h * f),
        Dock::Bottom => Rect::from_xywh(x, y + h * (1.0 - f), w, h * f),
        Dock::Fill => host,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn px_stays_and_percent_scales_with_available() {
        assert!(approx(
            dock_thickness(Length::Px(100.0), 40.0, 400.0),
            100.0
        ));
        assert!(approx(
            dock_thickness(Length::Percent(25.0), 40.0, 400.0),
            100.0
        ));
    }

    #[test]
    fn auto_and_fill_take_intrinsic() {
        assert!(approx(dock_thickness(Length::Fit, 70.0, 400.0), 70.0));
        assert!(approx(dock_thickness(Length::Fill(1.0), 70.0, 400.0), 70.0));
    }

    #[test]
    fn unbounded_axis_falls_back_to_content() {
        assert!(approx(
            dock_thickness(Length::Percent(50.0), 80.0, f32::INFINITY),
            80.0
        ));
    }

    #[test]
    fn negative_and_non_finite_inputs_clamp_to_zero() {
        assert!(approx(dock_thickness(Length::Px(-5.0), 10.0, 400.0), 0.0));
        assert!(approx(dock_thickness(Length::Fit, -3.0, 400.0), 0.0));
        assert!(approx(dock_thickness(Length::Fit, f32::NAN, 400.0), 0.0));
    }

    #[test]
    fn effective_dock_last_child_fill_wins() {
        let docks = [Dock::Left, Dock::Left];
        assert_eq!(effective_dock(&docks, true, 0, 2), Dock::Left);
        assert_eq!(effective_dock(&docks, true, 1, 2), Dock::Fill);
        assert_eq!(effective_dock(&docks, false, 1, 2), Dock::Left);

        assert_eq!(effective_dock(&[], true, 0, 1), Dock::Fill);
        assert_eq!(effective_dock(&[Dock::Top], false, 3, 4), Dock::Fill);
    }

    #[test]
    fn gap_sanitizes() {
        assert!(approx(sanitize_gap(10.0), 10.0));
        assert!(approx(sanitize_gap(-2.0), 0.0));
        assert!(approx(sanitize_gap(f32::NAN), 0.0));
        assert!(approx(sanitize_gap(f32::INFINITY), 0.0));
    }

    #[test]
    fn frac_sanitizes_like_split_panel() {
        assert_eq!(sanitize_frac(2.0), 1.0);
        assert_eq!(sanitize_frac(-1.0), 0.0);
        assert_eq!(sanitize_frac(f32::NAN), 0.5);
        assert_eq!(sanitize_preview_frac(99.0), 0.85);
        assert_eq!(sanitize_preview_frac(f32::NAN), 0.5);
    }

    #[test]
    fn split_axis_maps_sides() {
        use crate::container::Axis;
        assert_eq!(
            split_axis_for_dock(Dock::Left),
            Some((Axis::Horizontal, true))
        );
        assert_eq!(
            split_axis_for_dock(Dock::Right),
            Some((Axis::Horizontal, false))
        );
        assert_eq!(split_axis_for_dock(Dock::Top), Some((Axis::Vertical, true)));
        assert_eq!(
            split_axis_for_dock(Dock::Bottom),
            Some((Axis::Vertical, false))
        );
        assert_eq!(split_axis_for_dock(Dock::Fill), None);
    }

    #[test]
    fn preview_rect_splits_host() {
        use sequoia_core::Rect;
        let host = Rect::from_xywh(0.0, 0.0, 400.0, 300.0);
        let l = dock_preview_rect(host, Dock::Left, 0.5);
        assert!(approx(l.size.width, 200.0));
        assert!(approx(l.size.height, 300.0));
        let b = dock_preview_rect(host, Dock::Bottom, 0.25);
        assert!(approx(b.size.height, 75.0));
        assert_eq!(dock_preview_rect(host, Dock::Fill, 0.5), host);
    }

    #[test]
    fn border_order_groups_sides_before_fill() {
        use Dock::*;
        let docks = [Left, Fill, Bottom, Right, Top];

        assert_eq!(
            dock_layout_order(&docks, false, false, 5),
            vec![2, 4, 0, 3, 1]
        );

        assert_eq!(
            dock_layout_order(&docks, false, true, 5),
            vec![0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn border_ignores_last_child_fill() {
        use Dock::*;
        let docks = [Left, Left];
        assert_eq!(resolve_dock(&docks, true, false, 1, 2), Left);
        assert_eq!(resolve_dock(&docks, true, true, 1, 2), Fill);
        assert!(!DockStyle::default().ordered);
        assert!(!DockStyle::default().last_child_fill);
    }
}
