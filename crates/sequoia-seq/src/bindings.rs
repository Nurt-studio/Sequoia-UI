use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_async::Resource;
use sequoia_reactive::{I18n, Memo, Signal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropType {
    Str,

    Bool,

    Int,

    Float,
}

impl PropType {
    pub fn from_seq(name: &str) -> Option<Self> {
        match name {
            "String" => Some(Self::Str),
            "Bool" => Some(Self::Bool),
            "Int" => Some(Self::Int),
            "Float" => Some(Self::Float),
            _ => None,
        }
    }

    pub fn seq_name(self) -> &'static str {
        match self {
            Self::Str => "String",
            Self::Bool => "Bool",
            Self::Int => "Int",
            Self::Float => "Float",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionSig {
    A0,

    Int,

    Float,

    Str,

    Bool,

    Point,

    PointNamed,

    Wheel,

    Pinch,
}

#[derive(Clone)]
pub enum FieldSignal {
    Str(Signal<String>),

    Bool(Signal<bool>),

    Int(Signal<i64>),

    Float(Signal<f64>),
}

pub trait SeqRow {
    fn id(&self) -> i64;

    fn fields(&self) -> Vec<(String, FieldSignal)>;
}

#[derive(Clone)]
pub struct SeqRowData {
    pub id: i64,

    pub fields: HashMap<String, FieldSignal>,
}

pub(crate) type RowsFn = std::rc::Rc<dyn Fn() -> Vec<SeqRowData>>;

pub(crate) type PointNamedAction = Rc<dyn Fn(f64, f64, String)>;

pub(crate) type WheelAction = Rc<dyn Fn(f64, f64, f64)>;

pub(crate) type PinchAction = Rc<dyn Fn(f64, f64, f64)>;

impl SeqRowData {
    fn snapshot<R: SeqRow>(row: &R) -> Self {
        Self {
            id: row.id(),
            fields: row.fields().into_iter().collect(),
        }
    }
}

#[derive(Clone, Default)]
pub struct HostBindings {
    strings: HashMap<String, Signal<String>>,
    bools: HashMap<String, Signal<bool>>,
    ints: HashMap<String, Signal<i64>>,
    floats: HashMap<String, Signal<f64>>,
    memo_strings: HashMap<String, Memo<String>>,
    memo_bools: HashMap<String, Memo<bool>>,
    memo_ints: HashMap<String, Memo<i64>>,
    memo_floats: HashMap<String, Memo<f64>>,
    env_strings: HashMap<String, Signal<String>>,
    env_bools: HashMap<String, Signal<bool>>,
    env_ints: HashMap<String, Signal<i64>>,
    env_floats: HashMap<String, Signal<f64>>,
    i18n: Option<I18n>,
    resources: HashMap<String, Resource<String>>,
    rows: HashMap<String, RowsFn>,
    actions0: HashMap<String, Rc<dyn Fn()>>,
    actions_int: HashMap<String, Rc<dyn Fn(i64)>>,
    actions_float: HashMap<String, Rc<dyn Fn(f64)>>,
    actions_str: HashMap<String, Rc<dyn Fn(String)>>,
    actions_bool: HashMap<String, Rc<dyn Fn(bool)>>,
    actions_point: HashMap<String, Rc<dyn Fn(f64, f64)>>,
    actions_point_named: HashMap<String, PointNamedAction>,
    actions_wheel: HashMap<String, WheelAction>,
    actions_pinch: HashMap<String, PinchAction>,

    tick_angles: HashMap<String, Rc<Cell<f32>>>,

    seq: Option<Rc<crate::eval::SeqRuntime>>,
}

impl HostBindings {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub(crate) fn with_seq(mut self, rt: Rc<crate::eval::SeqRuntime>) -> Self {
        self.seq = Some(rt);
        self
    }

    pub(crate) fn get_seq(&self) -> Option<Rc<crate::eval::SeqRuntime>> {
        self.seq.clone()
    }

    pub fn prop_str(mut self, name: impl Into<String>, sig: Signal<String>) -> Self {
        self.strings.insert(name.into(), sig);
        self
    }

    pub fn prop_bool(mut self, name: impl Into<String>, sig: Signal<bool>) -> Self {
        self.bools.insert(name.into(), sig);
        self
    }

    pub fn prop_int(mut self, name: impl Into<String>, sig: Signal<i64>) -> Self {
        self.ints.insert(name.into(), sig);
        self
    }

    pub fn prop_float(mut self, name: impl Into<String>, sig: Signal<f64>) -> Self {
        self.floats.insert(name.into(), sig);
        self
    }

    pub fn tick_angle(mut self, name: impl Into<String>, angle: Rc<Cell<f32>>) -> Self {
        self.tick_angles.insert(name.into(), angle);
        self
    }

    pub fn memo_str(mut self, name: impl Into<String>, memo: Memo<String>) -> Self {
        self.memo_strings.insert(name.into(), memo);
        self
    }

    pub fn memo_bool(mut self, name: impl Into<String>, memo: Memo<bool>) -> Self {
        self.memo_bools.insert(name.into(), memo);
        self
    }

    pub fn memo_int(mut self, name: impl Into<String>, memo: Memo<i64>) -> Self {
        self.memo_ints.insert(name.into(), memo);
        self
    }

    pub fn memo_float(mut self, name: impl Into<String>, memo: Memo<f64>) -> Self {
        self.memo_floats.insert(name.into(), memo);
        self
    }

    pub fn env_str(mut self, name: impl Into<String>, sig: Signal<String>) -> Self {
        self.env_strings.insert(name.into(), sig);
        self
    }

    pub fn env_bool(mut self, name: impl Into<String>, sig: Signal<bool>) -> Self {
        self.env_bools.insert(name.into(), sig);
        self
    }

    pub fn env_int(mut self, name: impl Into<String>, sig: Signal<i64>) -> Self {
        self.env_ints.insert(name.into(), sig);
        self
    }

    pub fn env_float(mut self, name: impl Into<String>, sig: Signal<f64>) -> Self {
        self.env_floats.insert(name.into(), sig);
        self
    }

    pub fn i18n(mut self, i18n: I18n) -> Self {
        self.i18n = Some(i18n);
        self
    }

    pub fn resource(mut self, name: impl Into<String>, res: Resource<String>) -> Self {
        self.resources.insert(name.into(), res);
        self
    }

    pub fn on(mut self, name: impl Into<String>, f: impl Fn() + 'static) -> Self {
        self.actions0.insert(name.into(), Rc::new(f));
        self
    }

    pub fn on_int(mut self, name: impl Into<String>, f: impl Fn(i64) + 'static) -> Self {
        self.actions_int.insert(name.into(), Rc::new(f));
        self
    }

    pub fn on_float(mut self, name: impl Into<String>, f: impl Fn(f64) + 'static) -> Self {
        self.actions_float.insert(name.into(), Rc::new(f));
        self
    }

    pub fn on_str(mut self, name: impl Into<String>, f: impl Fn(String) + 'static) -> Self {
        self.actions_str.insert(name.into(), Rc::new(f));
        self
    }

    pub fn on_bool(mut self, name: impl Into<String>, f: impl Fn(bool) + 'static) -> Self {
        self.actions_bool.insert(name.into(), Rc::new(f));
        self
    }

    pub fn on_point(mut self, name: impl Into<String>, f: impl Fn(f64, f64) + 'static) -> Self {
        self.actions_point.insert(name.into(), Rc::new(f));
        self
    }

    pub fn on_point_named(
        mut self,
        name: impl Into<String>,
        f: impl Fn(f64, f64, String) + 'static,
    ) -> Self {
        self.actions_point_named.insert(name.into(), Rc::new(f));
        self
    }

    pub fn rows<R>(mut self, name: impl Into<String>, sig: Signal<Vec<R>>) -> Self
    where
        R: SeqRow + Clone + PartialEq + 'static,
    {
        self.rows.insert(
            name.into(),
            Rc::new(move || sig.get().iter().map(SeqRowData::snapshot).collect()),
        );
        self
    }

    pub(crate) fn prop_type(&self, name: &str) -> Option<PropType> {
        if self.strings.contains_key(name) || self.memo_strings.contains_key(name) {
            Some(PropType::Str)
        } else if self.bools.contains_key(name) || self.memo_bools.contains_key(name) {
            Some(PropType::Bool)
        } else if self.ints.contains_key(name) || self.memo_ints.contains_key(name) {
            Some(PropType::Int)
        } else if self.floats.contains_key(name) || self.memo_floats.contains_key(name) {
            Some(PropType::Float)
        } else {
            None
        }
    }

    pub(crate) fn get_str(&self, name: &str) -> Option<Signal<String>> {
        self.strings.get(name).cloned()
    }

    pub(crate) fn get_bool(&self, name: &str) -> Option<Signal<bool>> {
        self.bools.get(name).cloned()
    }

    pub(crate) fn get_int(&self, name: &str) -> Option<Signal<i64>> {
        self.ints.get(name).cloned()
    }

    pub(crate) fn get_float(&self, name: &str) -> Option<Signal<f64>> {
        self.floats.get(name).cloned()
    }

    pub(crate) fn get_tick_angle(&self, name: &str) -> Option<Rc<Cell<f32>>> {
        self.tick_angles.get(name).cloned()
    }

    pub(crate) fn get_memo_str(&self, name: &str) -> Option<Memo<String>> {
        self.memo_strings.get(name).cloned()
    }

    pub(crate) fn get_memo_bool(&self, name: &str) -> Option<Memo<bool>> {
        self.memo_bools.get(name).cloned()
    }

    pub(crate) fn get_memo_int(&self, name: &str) -> Option<Memo<i64>> {
        self.memo_ints.get(name).cloned()
    }

    pub(crate) fn get_memo_float(&self, name: &str) -> Option<Memo<f64>> {
        self.memo_floats.get(name).cloned()
    }

    pub(crate) fn get_env_str(&self, name: &str) -> Option<Signal<String>> {
        self.env_strings.get(name).cloned()
    }

    pub(crate) fn get_env_bool(&self, name: &str) -> Option<Signal<bool>> {
        self.env_bools.get(name).cloned()
    }

    pub(crate) fn get_env_int(&self, name: &str) -> Option<Signal<i64>> {
        self.env_ints.get(name).cloned()
    }

    pub(crate) fn get_env_float(&self, name: &str) -> Option<Signal<f64>> {
        self.env_floats.get(name).cloned()
    }

    pub(crate) fn get_i18n(&self) -> Option<I18n> {
        self.i18n.clone()
    }

    pub(crate) fn get_resource(&self, name: &str) -> Option<Resource<String>> {
        self.resources.get(name).cloned()
    }

    pub(crate) fn get_rows(&self, name: &str) -> Option<RowsFn> {
        self.rows.get(name).cloned()
    }

    pub(crate) fn action_sig(&self, name: &str) -> Option<ActionSig> {
        if self.actions0.contains_key(name) {
            Some(ActionSig::A0)
        } else if self.actions_int.contains_key(name) {
            Some(ActionSig::Int)
        } else if self.actions_float.contains_key(name) {
            Some(ActionSig::Float)
        } else if self.actions_str.contains_key(name) {
            Some(ActionSig::Str)
        } else if self.actions_bool.contains_key(name) {
            Some(ActionSig::Bool)
        } else if self.actions_point.contains_key(name) {
            Some(ActionSig::Point)
        } else if self.actions_point_named.contains_key(name) {
            Some(ActionSig::PointNamed)
        } else if self.actions_wheel.contains_key(name) {
            Some(ActionSig::Wheel)
        } else if self.actions_pinch.contains_key(name) {
            Some(ActionSig::Pinch)
        } else {
            None
        }
    }

    pub(crate) fn get_action0(&self, name: &str) -> Option<Rc<dyn Fn()>> {
        self.actions0.get(name).cloned()
    }

    pub(crate) fn get_action_int(&self, name: &str) -> Option<Rc<dyn Fn(i64)>> {
        self.actions_int.get(name).cloned()
    }

    pub(crate) fn get_action_float(&self, name: &str) -> Option<Rc<dyn Fn(f64)>> {
        self.actions_float.get(name).cloned()
    }

    pub(crate) fn get_action_str(&self, name: &str) -> Option<Rc<dyn Fn(String)>> {
        self.actions_str.get(name).cloned()
    }

    pub(crate) fn get_action_bool(&self, name: &str) -> Option<Rc<dyn Fn(bool)>> {
        self.actions_bool.get(name).cloned()
    }

    pub(crate) fn get_action_point(&self, name: &str) -> Option<Rc<dyn Fn(f64, f64)>> {
        self.actions_point.get(name).cloned()
    }

    pub(crate) fn get_action_point_named(&self, name: &str) -> Option<PointNamedAction> {
        self.actions_point_named.get(name).cloned()
    }

    pub(crate) fn get_action_wheel(&self, name: &str) -> Option<WheelAction> {
        self.actions_wheel.get(name).cloned()
    }

    pub(crate) fn get_action_pinch(&self, name: &str) -> Option<PinchAction> {
        self.actions_pinch.get(name).cloned()
    }

    pub fn on_wheel(
        mut self,
        name: impl Into<String>,
        f: impl Fn(f64, f64, f64) + 'static,
    ) -> Self {
        self.actions_wheel.insert(name.into(), Rc::new(f));
        self
    }

    pub fn on_pinch(
        mut self,
        name: impl Into<String>,
        f: impl Fn(f64, f64, f64) + 'static,
    ) -> Self {
        self.actions_pinch.insert(name.into(), Rc::new(f));
        self
    }
}
