use crate::runtime::ElementId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AccessibleRole {
    Button,
    Text,
    TextField,
    TextArea,
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
    Radio,
    RadioGroup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accessible {
    pub role: AccessibleRole,
    pub label: String,

    /// Current text value for inputs. Secure fields expose their masked value,
    /// never the real one.
    pub value: Option<String>,

    /// Caret position in the value, in characters.
    pub caret: Option<usize>,

    /// Selection range in characters, `(start, end)`.
    pub selection: Option<(usize, usize)>,

    pub disabled: bool,
}

impl Accessible {
    pub fn new(role: AccessibleRole, label: impl Into<String>) -> Self {
        Self {
            role,
            label: label.into(),
            value: None,
            caret: None,
            selection: None,
            disabled: false,
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn caret(mut self, caret: Option<usize>) -> Self {
        self.caret = caret;
        self
    }

    pub fn selection(mut self, selection: Option<(usize, usize)>) -> Self {
        self.selection = selection;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessibleNode {
    pub element: ElementId,
    pub role: AccessibleRole,
    pub label: String,
    pub value: Option<String>,
    pub caret: Option<usize>,
    pub selection: Option<(usize, usize)>,
    pub disabled: bool,
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
