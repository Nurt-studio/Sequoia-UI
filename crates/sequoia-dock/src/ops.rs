use crate::drop::DockDrop;
use crate::model::{DockModel, DockNode, FloatingPanel, PanelId, SplitAxis};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DockOp {
    Insert {
        panel: PanelId,
        target: PanelId,
        drop: DockDrop,
        frac: f32,
        moving: bool,
    },

    Float {
        panel: PanelId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },

    DockFloating {
        panel: PanelId,
        target: PanelId,
        drop: DockDrop,
        frac: f32,
    },

    Close {
        panel: PanelId,
    },

    SetFrac {
        panel: PanelId,
        frac: f32,
    },

    Activate {
        panel: PanelId,
    },

    MoveTab {
        panel: PanelId,
        to: usize,
    },
}

impl DockModel {
    pub fn apply(&mut self, op: DockOp) -> bool {
        match op {
            DockOp::Insert {
                panel,
                target,
                drop,
                frac,
                moving,
            } => self.insert(panel, target, drop, frac, moving),
            DockOp::Float { panel, x, y, w, h } => self.float(panel, x, y, w, h),
            DockOp::DockFloating {
                panel,
                target,
                drop,
                frac,
            } => self.dock_floating(panel, target, drop, frac),
            DockOp::Close { panel } => self.close(panel),
            DockOp::SetFrac { panel, frac } => self.set_frac(panel, frac),
            DockOp::Activate { panel } => self.activate(panel),
            DockOp::MoveTab { panel, to } => self.move_tab(panel, to),
        }
    }

    pub fn close(&mut self, panel: PanelId) -> bool {
        let mut removed = false;
        if let Some(root) = self.root.take() {
            match remove_from_node(root, panel) {
                (None, true) => removed = true,
                (Some(r), found) => {
                    self.root = Some(r);
                    removed = found;
                }
                (None, false) => {}
            }
        }
        let n = self.floating.len();
        self.floating.retain(|f| f.panel != panel);
        if self.floating.len() != n {
            removed = true;
        }
        self.sanitize();
        removed
    }

    fn take_from_tree(&mut self, panel: PanelId) -> bool {
        if let Some(root) = self.root.take() {
            match remove_from_node(root, panel) {
                (None, found) => found,
                (Some(r), found) => {
                    self.root = Some(r);
                    found
                }
            }
        } else {
            false
        }
    }

    fn insert(
        &mut self,
        panel: PanelId,
        target: PanelId,
        drop: DockDrop,
        frac: f32,
        moving: bool,
    ) -> bool {
        if panel == target {
            return false;
        }
        if drop == DockDrop::Float {
            return false;
        }

        let target_in_tree = self.root.as_ref().is_some_and(|r| r.contains(target));
        if !target_in_tree {
            return false;
        }
        if moving {
            let from_tree = self.take_from_tree(panel);
            let from_float = {
                let n = self.floating.len();
                self.floating.retain(|f| f.panel != panel);
                self.floating.len() != n
            };
            if !from_tree && !from_float {}

            if self.root.is_none() {
                self.root = Some(DockNode::Leaf(panel));
                self.sanitize();
                return true;
            }
            if !self.root.as_ref().is_some_and(|r| r.contains(target)) {
                self.root = Some(DockNode::Leaf(panel));
                self.sanitize();
                return true;
            }
        } else if self.contains(panel) {
            return false;
        }
        let f = DockNode::sanitized_frac(frac);
        let mut root = self.root.take();
        let ok = match &mut root {
            Some(r) => insert_into_node(r, panel, target, drop, f),
            None => false,
        };
        if !ok {
            self.root = root;
            return false;
        }
        self.root = root;
        self.sanitize();
        true
    }

    pub fn float(&mut self, panel: PanelId, x: f32, y: f32, w: f32, h: f32) -> bool {
        if self.is_floating(panel) {
            if let Some(f) = self.floating.iter_mut().find(|f| f.panel == panel) {
                if x.is_finite() {
                    f.x = x;
                }
                if y.is_finite() {
                    f.y = y;
                }
                if w.is_finite() && w > 0.0 {
                    f.w = w;
                }
                if h.is_finite() && h > 0.0 {
                    f.h = h;
                }
                return true;
            }
            return false;
        }
        if !self.root.as_ref().is_some_and(|r| r.contains(panel)) {
            return false;
        }
        self.take_from_tree(panel);

        self.floating
            .push(FloatingPanel::overlay(panel, x, y, w, h));
        self.sanitize();
        true
    }

    fn dock_floating(
        &mut self,
        panel: PanelId,
        target: PanelId,
        drop: DockDrop,
        frac: f32,
    ) -> bool {
        if panel == target || drop == DockDrop::Float {
            return false;
        }
        if !self.is_floating(panel) {
            return false;
        }
        if !self.root.as_ref().is_some_and(|r| r.contains(target)) {
            if self.root.is_none() {
                self.floating.retain(|f| f.panel != panel);
                self.root = Some(DockNode::Leaf(panel));
                self.sanitize();
                return true;
            }
            return false;
        }
        self.floating.retain(|f| f.panel != panel);
        let f = DockNode::sanitized_frac(frac);
        let mut root = self.root.take();
        let ok = match &mut root {
            Some(r) => insert_into_node(r, panel, target, drop, f),
            None => false,
        };
        self.root = root;
        if !ok {
            self.floating
                .push(FloatingPanel::overlay(panel, 40.0, 40.0, 320.0, 240.0));
            return false;
        }
        self.sanitize();
        true
    }

    fn set_frac(&mut self, panel: PanelId, frac: f32) -> bool {
        let f = DockNode::sanitized_frac(frac);
        fn go(node: &mut DockNode, panel: PanelId, f: f32) -> bool {
            match node {
                DockNode::Leaf(_) | DockNode::Tabs { .. } => false,
                DockNode::Split {
                    frac,
                    first,
                    second,
                    ..
                } => {
                    if first.contains(panel) || second.contains(panel) {
                        if go(first, panel, f) || go(second, panel, f) {
                            return true;
                        }
                        *frac = f;
                        true
                    } else {
                        false
                    }
                }
            }
        }
        let ok = self.root.as_mut().is_some_and(|r| go(r, panel, f));
        if ok {
            self.sanitize();
        }
        ok
    }

    fn activate(&mut self, panel: PanelId) -> bool {
        fn go(node: &mut DockNode, panel: PanelId) -> bool {
            match node {
                DockNode::Leaf(_) => false,
                DockNode::Tabs { tabs, active } => {
                    if let Some(i) = tabs.iter().position(|p| *p == panel) {
                        *active = i;
                        true
                    } else {
                        false
                    }
                }
                DockNode::Split { first, second, .. } => go(first, panel) || go(second, panel),
            }
        }
        let ok = self.root.as_mut().is_some_and(|r| go(r, panel));
        if ok {
            self.sanitize();
        }
        ok
    }

    fn move_tab(&mut self, panel: PanelId, to: usize) -> bool {
        fn go(node: &mut DockNode, panel: PanelId, to: usize) -> bool {
            match node {
                DockNode::Leaf(_) => false,
                DockNode::Tabs { tabs, active } => {
                    if let Some(from) = tabs.iter().position(|p| *p == panel) {
                        let item = tabs.remove(from);
                        let t = to.min(tabs.len());
                        tabs.insert(t, item);
                        *active = t;
                        true
                    } else {
                        false
                    }
                }
                DockNode::Split { first, second, .. } => {
                    go(first, panel, to) || go(second, panel, to)
                }
            }
        }
        let ok = self.root.as_mut().is_some_and(|r| go(r, panel, to));
        if ok {
            self.sanitize();
        }
        ok
    }

    pub fn attach_window(&mut self, panel: PanelId, window: Option<u64>) -> bool {
        if let Some(f) = self.floating.iter_mut().find(|f| f.panel == panel) {
            f.window = window;
            true
        } else {
            false
        }
    }

    pub fn set_frac_at(&mut self, path: &[bool], frac: f32) -> bool {
        let f = DockNode::sanitized_frac(frac);
        let mut node = match &mut self.root {
            Some(r) => r,
            None => return false,
        };
        for step in path {
            match node {
                DockNode::Split { first, second, .. } => {
                    node = if *step { second } else { first };
                }
                _ => return false,
            }
        }
        match node {
            DockNode::Split { frac: slot, .. } => {
                *slot = f;
                self.sanitize();
                true
            }
            _ => false,
        }
    }

    pub fn dock_first_tab(&mut self, panel: PanelId) -> bool {
        if !self.is_floating(panel) {
            return false;
        }
        let target = match &self.root {
            None => {
                self.floating.retain(|f| f.panel != panel);
                self.root = Some(DockNode::Leaf(panel));
                self.sanitize();
                return true;
            }
            Some(root) => match root.panels().first().copied() {
                Some(t) => t,
                None => {
                    self.floating.retain(|f| f.panel != panel);
                    self.root = Some(DockNode::Leaf(panel));
                    self.sanitize();
                    return true;
                }
            },
        };
        self.dock_floating(panel, target, crate::drop::DockDrop::Tab, 0.5)
    }

    pub fn frac_at(&self, path: &[bool]) -> Option<f32> {
        let mut node = self.root.as_ref()?;
        for step in path {
            match node {
                DockNode::Split { first, second, .. } => {
                    node = if *step { second } else { first };
                }
                _ => return None,
            }
        }
        match node {
            DockNode::Split { frac, .. } => Some(*frac),
            _ => None,
        }
    }
}

fn insert_into_node(
    node: &mut DockNode,
    panel: PanelId,
    target: PanelId,
    drop: DockDrop,
    frac: f32,
) -> bool {
    match node {
        DockNode::Leaf(p) => {
            if *p == target {
                let old = *p;
                match drop {
                    DockDrop::Tab => {
                        *node = DockNode::Tabs {
                            tabs: vec![old, panel],
                            active: 1,
                        };
                    }
                    DockDrop::Float => return false,
                    side => {
                        let (axis, first_new) = split_for(side);
                        let (a, b) = if first_new {
                            (DockNode::Leaf(panel), DockNode::Leaf(old))
                        } else {
                            (DockNode::Leaf(old), DockNode::Leaf(panel))
                        };
                        *node = DockNode::Split {
                            axis,
                            frac,
                            first: Box::new(a),
                            second: Box::new(b),
                        };
                    }
                }
                true
            } else {
                false
            }
        }
        DockNode::Tabs { tabs, active } => {
            if tabs.contains(&target) {
                match drop {
                    DockDrop::Tab => {
                        tabs.push(panel);
                        *active = tabs.len() - 1;
                    }
                    DockDrop::Float => return false,
                    side => {
                        let old = DockNode::Tabs {
                            tabs: std::mem::take(tabs),
                            active: *active,
                        };
                        let (axis, first_new) = split_for(side);
                        let (a, b) = if first_new {
                            (DockNode::Leaf(panel), old)
                        } else {
                            (old, DockNode::Leaf(panel))
                        };
                        *node = DockNode::Split {
                            axis,
                            frac,
                            first: Box::new(a),
                            second: Box::new(b),
                        };
                    }
                }
                true
            } else {
                false
            }
        }
        DockNode::Split { first, second, .. } => {
            insert_into_node(first, panel, target, drop, frac)
                || insert_into_node(second, panel, target, drop, frac)
        }
    }
}

fn split_for(drop: DockDrop) -> (SplitAxis, bool) {
    use sequoia_layout::{Dock, split_axis_for_dock};
    let dock = match drop {
        DockDrop::Left => Dock::Left,
        DockDrop::Right => Dock::Right,
        DockDrop::Top => Dock::Top,
        DockDrop::Bottom => Dock::Bottom,
        DockDrop::Tab | DockDrop::Float => return (SplitAxis::Horizontal, false),
    };
    match split_axis_for_dock(dock) {
        Some((axis, first)) => (SplitAxis::from_axis(axis), first),
        None => (SplitAxis::Horizontal, false),
    }
}

fn remove_from_node(node: DockNode, panel: PanelId) -> (Option<DockNode>, bool) {
    match node {
        DockNode::Leaf(p) => {
            if p == panel {
                (None, true)
            } else {
                (Some(DockNode::Leaf(p)), false)
            }
        }
        DockNode::Tabs {
            mut tabs,
            mut active,
        } => {
            if let Some(i) = tabs.iter().position(|p| *p == panel) {
                tabs.remove(i);
                if tabs.is_empty() {
                    (None, true)
                } else {
                    if active >= tabs.len() {
                        active = tabs.len() - 1;
                    }

                    if tabs.len() == 1 {
                        (Some(DockNode::Leaf(tabs[0])), true)
                    } else {
                        (Some(DockNode::Tabs { tabs, active }), true)
                    }
                }
            } else {
                (Some(DockNode::Tabs { tabs, active }), false)
            }
        }
        DockNode::Split {
            axis,
            frac,
            first,
            second,
        } => {
            let (a, fa) = remove_from_node(*first, panel);
            if fa {
                match a {
                    None => (Some(*second), true),
                    Some(a) => (
                        Some(DockNode::Split {
                            axis,
                            frac,
                            first: Box::new(a),
                            second,
                        }),
                        true,
                    ),
                }
            } else {
                let Some(a_restored) = a else {
                    debug_assert!(false, "remove_from_node: naruszony kontrakt (first)");

                    return remove_from_node(*second, panel);
                };
                let (b, fb) = remove_from_node(*second, panel);
                match (b, fb) {
                    (None, true) => (Some(a_restored), true),
                    (Some(b), true) => (
                        Some(DockNode::Split {
                            axis,
                            frac,
                            first: Box::new(a_restored),
                            second: Box::new(b),
                        }),
                        true,
                    ),
                    (b_opt, false) => {
                        let Some(b_restored) = b_opt else {
                            debug_assert!(false, "remove_from_node: naruszony kontrakt (second)");
                            return (Some(a_restored), false);
                        };
                        (
                            Some(DockNode::Split {
                                axis,
                                frac,
                                first: Box::new(a_restored),
                                second: Box::new(b_restored),
                            }),
                            false,
                        )
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u64) -> PanelId {
        PanelId::new(id)
    }

    #[test]
    fn insert_splits_leaf() {
        let mut m = DockModel::single(p(1));
        assert!(m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.5,
            moving: false
        }));
        assert!(matches!(m.root, Some(DockNode::Split { .. })));
        assert_eq!(m.panels().len(), 2);
    }

    #[test]
    fn insert_tab_appends() {
        let mut m = DockModel::single(p(1));
        assert!(m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Tab,
            frac: 0.5,
            moving: false
        }));
        assert!(matches!(m.root, Some(DockNode::Tabs { .. })));
        assert_eq!(m.active_tab_of(p(2)), Some(1));
    }

    #[test]
    fn same_panel_is_noop() {
        let mut m = DockModel::single(p(1));
        assert!(!m.apply(DockOp::Insert {
            panel: p(1),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.5,
            moving: true
        }));
    }

    #[test]
    fn move_panel_between_slots() {
        let mut m = DockModel::single(p(1));
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.5,
            moving: false,
        });
        m.apply(DockOp::Insert {
            panel: p(3),
            target: p(2),
            drop: DockDrop::Tab,
            frac: 0.5,
            moving: false,
        });

        assert!(m.apply(DockOp::Insert {
            panel: p(1),
            target: p(2),
            drop: DockDrop::Tab,
            frac: 0.5,
            moving: true
        }));
        assert_eq!(m.panels().len(), 3);
    }

    #[test]
    fn close_collapses_split() {
        let mut m = DockModel::single(p(1));
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.5,
            moving: false,
        });
        assert!(m.apply(DockOp::Close { panel: p(2) }));
        assert_eq!(m.root, Some(DockNode::Leaf(p(1))));
    }

    #[test]
    fn float_and_dock_back() {
        let mut m = DockModel::single(p(1));
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.5,
            moving: false,
        });
        assert!(m.apply(DockOp::Float {
            panel: p(2),
            x: 10.0,
            y: 10.0,
            w: 300.0,
            h: 200.0
        }));
        assert!(m.is_floating(p(2)));
        assert!(m.apply(DockOp::DockFloating {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Tab,
            frac: 0.5
        }));
        assert!(!m.is_floating(p(2)));
    }

    #[test]
    fn set_frac_and_activate() {
        let mut m = DockModel::single(p(1));
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.3,
            moving: false,
        });
        assert!(m.apply(DockOp::SetFrac {
            panel: p(1),
            frac: 0.7
        }));
        assert!(!m.apply(DockOp::SetFrac {
            panel: p(99),
            frac: 0.5
        }));
        m.apply(DockOp::Insert {
            panel: p(3),
            target: p(2),
            drop: DockDrop::Tab,
            frac: 0.5,
            moving: false,
        });
        assert!(m.apply(DockOp::Activate { panel: p(2) }));
        assert_eq!(m.active_tab_of(p(2)), Some(0));
        assert!(m.apply(DockOp::MoveTab { panel: p(3), to: 0 }));
        assert_eq!(m.active_tab_of(p(3)), Some(0));
    }

    #[test]
    fn close_last_leaves_empty_root() {
        let mut m = DockModel::single(p(1));
        assert!(m.apply(DockOp::Close { panel: p(1) }));
        assert!(m.root.is_none());
        assert!(!m.apply(DockOp::Close { panel: p(1) }));
    }

    #[test]
    fn frac_at_path_roundtrip() {
        let mut m = DockModel::single(p(1));
        m.apply(DockOp::Insert {
            panel: p(2),
            target: p(1),
            drop: DockDrop::Right,
            frac: 0.3,
            moving: false,
        });
        assert_eq!(m.frac_at(&[]), Some(0.3));
        assert!(m.set_frac_at(&[], 0.7));
        assert_eq!(m.frac_at(&[]), Some(0.7));
        assert!(!m.set_frac_at(&[false], 0.5));
        assert!(!m.set_frac_at(&[true, false], 0.5));
    }
}
