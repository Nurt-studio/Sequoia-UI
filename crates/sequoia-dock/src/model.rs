use sequoia_core::WindowId;
use sequoia_layout::Axis;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PanelId(pub u64);

impl PanelId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for PanelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "panel#{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SplitAxis {
    #[default]
    Horizontal,
    Vertical,
}

impl SplitAxis {
    pub const fn axis(self) -> Axis {
        match self {
            Self::Horizontal => Axis::Horizontal,
            Self::Vertical => Axis::Vertical,
        }
    }

    pub const fn from_axis(axis: Axis) -> Self {
        match axis {
            Axis::Horizontal => Self::Horizontal,
            Axis::Vertical => Self::Vertical,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DockNode {
    Leaf(PanelId),

    Split {
        axis: SplitAxis,
        frac: f32,
        first: Box<DockNode>,
        second: Box<DockNode>,
    },

    Tabs {
        tabs: Vec<PanelId>,
        active: usize,
    },
}

impl DockNode {
    pub fn leaf(panel: PanelId) -> Self {
        Self::Leaf(panel)
    }

    pub fn tabs(tabs: impl IntoIterator<Item = PanelId>) -> Self {
        let tabs: Vec<PanelId> = tabs.into_iter().collect();
        Self::Tabs { tabs, active: 0 }
    }

    pub fn sanitized_frac(frac: f32) -> f32 {
        if frac.is_finite() {
            frac.clamp(0.0, 1.0)
        } else {
            0.5
        }
    }

    pub fn panels(&self) -> Vec<PanelId> {
        let mut out = Vec::new();
        self.collect_panels(&mut out);
        out
    }

    fn collect_panels(&self, out: &mut Vec<PanelId>) {
        match self {
            Self::Leaf(p) => out.push(*p),
            Self::Split { first, second, .. } => {
                first.collect_panels(out);
                second.collect_panels(out);
            }
            Self::Tabs { tabs, .. } => out.extend(tabs.iter().copied()),
        }
    }

    pub fn contains(&self, panel: PanelId) -> bool {
        match self {
            Self::Leaf(p) => *p == panel,
            Self::Split { first, second, .. } => first.contains(panel) || second.contains(panel),
            Self::Tabs { tabs, .. } => tabs.contains(&panel),
        }
    }

    pub fn path_to(&self, panel: PanelId) -> Option<Vec<bool>> {
        match self {
            Self::Leaf(p) => {
                if *p == panel {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            Self::Tabs { tabs, .. } => {
                if tabs.contains(&panel) {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            Self::Split { first, second, .. } => {
                if let Some(mut p) = first.path_to(panel) {
                    let mut out = vec![false];
                    out.append(&mut p);
                    Some(out)
                } else if let Some(mut p) = second.path_to(panel) {
                    let mut out = vec![true];
                    out.append(&mut p);
                    Some(out)
                } else {
                    None
                }
            }
        }
    }

    fn sanitize_tabs(&mut self) {
        if let Self::Tabs { tabs, active } = self {
            if tabs.is_empty() {
                return;
            }
            if *active >= tabs.len() {
                *active = tabs.len() - 1;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatingPanel {
    pub panel: PanelId,

    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,

    pub window: Option<u64>,
}

impl FloatingPanel {
    pub fn overlay(panel: PanelId, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            panel,
            x: sanitize_pos(x),
            y: sanitize_pos(y),
            w: sanitize_size(w, 240.0),
            h: sanitize_size(h, 160.0),
            window: None,
        }
    }

    pub fn window_id(&self) -> Option<WindowId> {
        self.window.map(WindowId::new)
    }

    pub fn is_overlay(&self) -> bool {
        self.window.is_none()
    }
}

fn sanitize_pos(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

fn sanitize_size(v: f32, fallback: f32) -> f32 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        fallback
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DockModel {
    pub root: Option<DockNode>,
    pub floating: Vec<FloatingPanel>,
}

impl Default for DockModel {
    fn default() -> Self {
        Self::new()
    }
}

impl DockModel {
    pub fn new() -> Self {
        Self {
            root: None,
            floating: Vec::new(),
        }
    }

    pub fn single(panel: PanelId) -> Self {
        Self {
            root: Some(DockNode::Leaf(panel)),
            floating: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none() && self.floating.is_empty()
    }

    pub fn panels(&self) -> Vec<PanelId> {
        let mut out = self.root.as_ref().map(|r| r.panels()).unwrap_or_default();
        out.extend(self.floating.iter().map(|f| f.panel));
        out
    }

    pub fn contains(&self, panel: PanelId) -> bool {
        self.root.as_ref().is_some_and(|r| r.contains(panel))
            || self.floating.iter().any(|f| f.panel == panel)
    }

    pub fn is_floating(&self, panel: PanelId) -> bool {
        self.floating.iter().any(|f| f.panel == panel)
    }

    pub fn floating_of(&self, panel: PanelId) -> Option<FloatingPanel> {
        self.floating.iter().find(|f| f.panel == panel).copied()
    }

    pub fn active_tab_of(&self, panel: PanelId) -> Option<usize> {
        fn go(node: &DockNode, panel: PanelId) -> Option<usize> {
            match node {
                DockNode::Leaf(_) => None,
                DockNode::Tabs { tabs, active } => {
                    if tabs.contains(&panel) {
                        Some(*active)
                    } else {
                        None
                    }
                }
                DockNode::Split { first, second, .. } => {
                    go(first, panel).or_else(|| go(second, panel))
                }
            }
        }
        self.root.as_ref().and_then(|r| go(r, panel))
    }

    pub(crate) fn sanitize(&mut self) {
        fn go(node: &mut DockNode) {
            match node {
                DockNode::Leaf(_) => {}
                DockNode::Tabs { tabs, active } => {
                    if *active >= tabs.len() && !tabs.is_empty() {
                        *active = tabs.len() - 1;
                    }
                    let _ = tabs;
                }
                DockNode::Split {
                    frac,
                    first,
                    second,
                    ..
                } => {
                    *frac = DockNode::sanitized_frac(*frac);
                    go(first);
                    go(second);
                }
            }
        }
        if let Some(r) = &mut self.root {
            go(r);
            r.sanitize_tabs();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_contains_and_lists() {
        let m = DockModel::single(PanelId::new(7));
        assert!(m.contains(PanelId::new(7)));
        assert!(!m.contains(PanelId::new(8)));
        assert_eq!(m.panels(), vec![PanelId::new(7)]);
    }

    #[test]
    fn path_to_finds_nested() {
        let root = DockNode::Split {
            axis: SplitAxis::Horizontal,
            frac: 0.3,
            first: Box::new(DockNode::Leaf(PanelId::new(1))),
            second: Box::new(DockNode::Tabs {
                tabs: vec![PanelId::new(2), PanelId::new(3)],
                active: 1,
            }),
        };
        assert_eq!(root.path_to(PanelId::new(1)), Some(vec![false]));
        assert_eq!(root.path_to(PanelId::new(3)), Some(vec![true]));
        assert_eq!(root.path_to(PanelId::new(9)), None);
    }

    #[test]
    fn frac_sanitizes() {
        assert_eq!(DockNode::sanitized_frac(2.0), 1.0);
        assert_eq!(DockNode::sanitized_frac(f32::NAN), 0.5);
        assert_eq!(DockNode::sanitized_frac(-1.0), 0.0);
    }

    #[test]
    fn floating_overlay_defaults() {
        let f = FloatingPanel::overlay(PanelId::new(1), f32::NAN, 10.0, -5.0, 100.0);
        assert!(f.is_overlay());
        assert_eq!((f.x, f.y, f.w, f.h), (0.0, 10.0, 240.0, 100.0));
    }
}
