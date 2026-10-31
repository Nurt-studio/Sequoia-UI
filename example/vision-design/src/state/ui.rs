//! UI helper types kept in the state: menu, panels, picker, toasts.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sequoia_ui::prelude::*;
use sequoia_ui::view::ElementId;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickerTarget {
    Fill,
    Stroke,
}

#[derive(Clone)]
pub(crate) struct ColorPickerState {
    pub(crate) open: Signal<bool>,
    pub(crate) target: Signal<PickerTarget>,
    pub(crate) hue: Signal<f32>,
    pub(crate) pos: Signal<Vec2>,
    pub(crate) dragging: Rc<Cell<bool>>,
    pub(crate) drag_offset: Rc<RefCell<Vec2>>,
    pub(crate) has_positioned: Rc<Cell<bool>>,
}

impl ColorPickerState {
    pub(crate) fn new() -> Self {
        Self {
            open: Signal::new(false),
            target: Signal::new(PickerTarget::Fill),
            hue: Signal::new(0.0),
            pos: Signal::new(Vec2::new(0.0, 0.0)),
            dragging: Rc::new(Cell::new(false)),
            drag_offset: Rc::new(RefCell::new(Vec2::new(0.0, 0.0))),
            has_positioned: Rc::new(Cell::new(false)),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Menu {
    pub(crate) open: Signal<bool>,
    pub(crate) anchor: Signal<Option<ElementId>>,
}

impl Menu {
    pub(crate) fn new() -> Self {
        Self {
            open: Signal::new(false),
            anchor: Signal::new(None),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Menus {
    pub(crate) file: Menu,
    pub(crate) edit: Menu,
    pub(crate) view: Menu,
    pub(crate) object: Menu,
    pub(crate) window: Menu,
    pub(crate) help: Menu,
}

impl Menus {
    pub(crate) fn new() -> Self {
        Self {
            file: Menu::new(),
            edit: Menu::new(),
            view: Menu::new(),
            object: Menu::new(),
            window: Menu::new(),
            help: Menu::new(),
        }
    }
    pub(crate) fn by_id(&self, id: u8) -> &Menu {
        match id {
            1 => &self.file,
            2 => &self.edit,
            3 => &self.view,
            4 => &self.object,
            5 => &self.window,
            _ => &self.help,
        }
    }
    pub(crate) fn others(&self, id: u8) -> Vec<Signal<bool>> {
        (1..=6u8)
            .filter(|i| *i != id)
            .map(|i| self.by_id(i).open.clone())
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Design,
    Inspect,
}

#[derive(Clone)]
pub(crate) struct PanelSections {
    pub(crate) alignment: Signal<bool>,
    pub(crate) position: Signal<bool>,
    pub(crate) size: Signal<bool>,
    pub(crate) appearance: Signal<bool>,
    pub(crate) auto_layout: Signal<bool>,
    pub(crate) constraints: Signal<bool>,
    pub(crate) fill: Signal<bool>,
    pub(crate) stroke: Signal<bool>,
    pub(crate) export: Signal<bool>,
}

impl PanelSections {
    pub(crate) fn new() -> Self {
        Self {
            alignment: Signal::new(true),
            position: Signal::new(true),
            size: Signal::new(true),
            appearance: Signal::new(true),
            auto_layout: Signal::new(true),
            constraints: Signal::new(true),
            fill: Signal::new(true),
            stroke: Signal::new(false),
            export: Signal::new(false),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToastKind {
    Info,
    Warn,
    Error,
}

#[derive(Clone, PartialEq)]
pub(crate) struct ToastEntry {
    pub(crate) id: u64,
    pub(crate) msg: String,
    pub(crate) kind: ToastKind,
}

