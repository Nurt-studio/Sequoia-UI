use sequoia_core::{Rect, Vec2};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DockDrop {
    Left,
    Right,
    Top,
    Bottom,

    #[default]
    Tab,

    Float,
}

impl DockDrop {
    pub const fn is_side(self) -> bool {
        matches!(self, Self::Left | Self::Right | Self::Top | Self::Bottom)
    }

    pub const fn is_tab(self) -> bool {
        matches!(self, Self::Tab)
    }

    pub const fn is_float(self) -> bool {
        matches!(self, Self::Float)
    }
}

pub const EDGE_FRACTION: f32 = 0.22;

pub fn drop_from_fraction(fraction: Vec2, edge: f32) -> DockDrop {
    let e = if edge.is_finite() {
        edge.clamp(0.0, 0.45)
    } else {
        EDGE_FRACTION
    };
    let fx = fraction.x.clamp(0.0, 1.0);
    let fy = fraction.y.clamp(0.0, 1.0);

    let dist_left = fx;
    let dist_right = 1.0 - fx;
    let dist_top = fy;
    let dist_bottom = 1.0 - fy;
    let min_h = dist_left.min(dist_right);
    let min_v = dist_top.min(dist_bottom);
    if min_h < e || min_v < e {
        if min_h < min_v {
            if dist_left < dist_right {
                DockDrop::Left
            } else {
                DockDrop::Right
            }
        } else if dist_top < dist_bottom {
            DockDrop::Top
        } else {
            DockDrop::Bottom
        }
    } else {
        DockDrop::Tab
    }
}

pub fn drop_from_fraction_or_float(fraction: Vec2, edge: f32, force_float: bool) -> DockDrop {
    if force_float {
        DockDrop::Float
    } else {
        drop_from_fraction(fraction, edge)
    }
}

pub fn preview_rect(host: Rect, drop: DockDrop, frac: f32) -> Rect {
    use sequoia_layout::{Dock, dock_preview_rect};
    match drop {
        DockDrop::Left => dock_preview_rect(host, Dock::Left, frac),
        DockDrop::Right => dock_preview_rect(host, Dock::Right, frac),
        DockDrop::Top => dock_preview_rect(host, Dock::Top, frac),
        DockDrop::Bottom => dock_preview_rect(host, Dock::Bottom, frac),
        DockDrop::Tab => host,
        DockDrop::Float => {
            let w = host.size.width.max(0.0);
            let h = host.size.height.max(0.0);
            let pw = w * 0.6;
            let ph = h * 0.6;
            Rect::from_xywh(
                host.origin.x + (w - pw) * 0.5,
                host.origin.y + (h - ph) * 0.5,
                pw,
                ph,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn edges_win_over_center() {
        assert_eq!(
            drop_from_fraction(v(0.05, 0.5), EDGE_FRACTION),
            DockDrop::Left
        );
        assert_eq!(
            drop_from_fraction(v(0.95, 0.5), EDGE_FRACTION),
            DockDrop::Right
        );
        assert_eq!(
            drop_from_fraction(v(0.5, 0.05), EDGE_FRACTION),
            DockDrop::Top
        );
        assert_eq!(
            drop_from_fraction(v(0.5, 0.95), EDGE_FRACTION),
            DockDrop::Bottom
        );
        assert_eq!(
            drop_from_fraction(v(0.5, 0.5), EDGE_FRACTION),
            DockDrop::Tab
        );
    }

    #[test]
    fn bad_edge_falls_back() {
        assert_eq!(drop_from_fraction(v(0.5, 0.5), f32::NAN), DockDrop::Tab);
        assert_eq!(drop_from_fraction(v(0.05, 0.5), -1.0), DockDrop::Tab);
    }

    #[test]
    fn force_float_wins() {
        assert_eq!(
            drop_from_fraction_or_float(v(0.05, 0.5), EDGE_FRACTION, true),
            DockDrop::Float
        );
        assert_eq!(
            drop_from_fraction_or_float(v(0.05, 0.5), EDGE_FRACTION, false),
            DockDrop::Left
        );
    }

    #[test]
    fn preview_splits_host() {
        let host = Rect::from_xywh(0.0, 0.0, 400.0, 300.0);
        let l = preview_rect(host, DockDrop::Left, 0.5);
        assert_eq!((l.size.width, l.size.height), (200.0, 300.0));
        let b = preview_rect(host, DockDrop::Bottom, 0.25);
        assert!((b.size.height - 75.0).abs() < 0.01);
        let t = preview_rect(host, DockDrop::Tab, 0.5);
        assert_eq!(t, host);
        let f = preview_rect(host, DockDrop::Float, 0.5);
        assert!(f.size.width < host.size.width && f.size.height < host.size.height);
    }

    #[test]
    fn preview_frac_clamps() {
        let host = Rect::from_xywh(0.0, 0.0, 400.0, 300.0);
        let l = preview_rect(host, DockDrop::Left, f32::NAN);
        assert!((l.size.width - 200.0).abs() < 0.01);
        let l2 = preview_rect(host, DockDrop::Left, 99.0);
        assert!((l2.size.width - 400.0 * 0.85).abs() < 0.01);
    }
}
