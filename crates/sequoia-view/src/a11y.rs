use crate::runtime::ElementId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AccessibleRole {
    Button,
    Text,
    TextField,
    Checkbox,
    Switch,
    Slider,
    List,
    ListItem,
    Table,
    Tree,
    TreeItem,
    Dialog,
    Menu,
    MenuItem,
    Progress,
    Image,
    Group,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accessible {
    pub role: AccessibleRole,
    pub label: String,
}

impl Accessible {
    pub fn new(role: AccessibleRole, label: impl Into<String>) -> Self {
        Self {
            role,
            label: label.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessibleNode {
    pub element: ElementId,
    pub role: AccessibleRole,
    pub label: String,
    pub children: Vec<AccessibleNode>,
}

impl AccessibleNode {
    pub fn flat_labels(&self) -> Vec<String> {
        let mut out = vec![self.label.clone()];
        for c in &self.children {
            out.extend(c.flat_labels());
        }
        out
    }
}
