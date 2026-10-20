use std::rc::Rc;

use crate::bindings::SeqRowData;

#[derive(Clone)]
pub(crate) enum Value {
    Int(i64),

    Float(f64),

    Str(Rc<String>),

    Bool(bool),

    Tuple(Rc<Vec<Value>>),

    Option(Option<Box<Value>>),

    Row(SeqRowData),

    Null,
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "Int({i})"),
            Self::Float(v) => write!(f, "Float({v})"),
            Self::Str(s) => write!(f, "Str({s:?})"),
            Self::Bool(b) => write!(f, "Bool({b})"),
            Self::Tuple(items) => write!(f, "Tuple({items:?})"),
            Self::Option(v) => match v {
                Some(x) => write!(f, "Some({x:?})"),
                None => write!(f, "None"),
            },
            Self::Row(row) => write!(f, "Row(id: {})", row.id),
            Self::Null => write!(f, "Null"),
        }
    }
}

impl Value {
    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::Str(_) => "String",
            Self::Bool(_) => "Bool",
            Self::Tuple(_) => "krotka",
            Self::Option(_) => "Option",
            Self::Row(_) => "wiersz",
            Self::Null => "no value",
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub(crate) fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(i) => Some(*i as f64),
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub(crate) fn is_number(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Float(_))
    }
}
