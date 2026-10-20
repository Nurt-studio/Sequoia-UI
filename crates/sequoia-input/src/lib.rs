#![forbid(unsafe_code)]

use bitflags::bitflags;

use sequoia_core::{Point, Vec2};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Modifiers: u8 {
        const NONE = 0;

        const SHIFT = 1 << 0;

        const CONTROL = 1 << 1;

        const ALT = 1 << 2;

        const SUPER = 1 << 3;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Key(u32);

impl Key {
    pub const NONE: Self = Self(0);

    pub const TAB: Self = Self(1);

    pub const ENTER: Self = Self(2);

    pub const ESCAPE: Self = Self(3);

    pub const SPACE: Self = Self(4);

    pub const BACKSPACE: Self = Self(5);

    pub const DELETE: Self = Self(6);

    pub const ARROW_UP: Self = Self(7);

    pub const ARROW_DOWN: Self = Self(8);

    pub const ARROW_LEFT: Self = Self(9);

    pub const ARROW_RIGHT: Self = Self(10);

    pub const HOME: Self = Self(11);

    pub const END: Self = Self(12);

    pub const INSERT: Self = Self(13);

    pub const PAGE_UP: Self = Self(14);

    pub const PAGE_DOWN: Self = Self(15);

    pub const F1: Self = Self(20);

    pub const F2: Self = Self(21);

    pub const F3: Self = Self(22);

    pub const F4: Self = Self(23);

    pub const F5: Self = Self(24);

    pub const F6: Self = Self(25);

    pub const F7: Self = Self(26);

    pub const F8: Self = Self(27);

    pub const F9: Self = Self(28);

    pub const F10: Self = Self(29);

    pub const F11: Self = Self(30);

    pub const F12: Self = Self(31);

    const CHAR_BASE: u32 = 0x1000;

    pub fn from_char(c: char) -> Self {
        Self(Self::CHAR_BASE + c as u32)
    }

    pub fn char(self) -> Option<char> {
        if self.0 >= Self::CHAR_BASE {
            char::from_u32(self.0 - Self::CHAR_BASE)
        } else {
            None
        }
    }

    pub fn allows_repeat(self) -> bool {
        if self.char().is_some() {
            return true;
        }
        matches!(
            self,
            Self::SPACE
                | Self::BACKSPACE
                | Self::DELETE
                | Self::ARROW_UP
                | Self::ARROW_DOWN
                | Self::ARROW_LEFT
                | Self::ARROW_RIGHT
                | Self::HOME
                | Self::END
                | Self::PAGE_UP
                | Self::PAGE_DOWN
                | Self::INSERT
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PointerButton {
    Left,

    Right,

    Middle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PenTool {
    #[default]
    Mouse,

    Pen,

    Eraser,

    Touch,

    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum PointerEvent {
    Move {
        pos: Point,
        modifiers: Modifiers,
    },
    Down {
        button: PointerButton,
        pos: Point,
        modifiers: Modifiers,
    },
    Up {
        button: PointerButton,
        pos: Point,
        modifiers: Modifiers,
    },
    Cancel,
    Wheel {
        delta: Vec2,
        pos: Point,
        modifiers: Modifiers,
    },

    PenMove {
        pos: Point,
        pressure: f32,
        tilt: Vec2,
        tool: PenTool,
        modifiers: Modifiers,
    },
    PenDown {
        button: PointerButton,
        pos: Point,
        pressure: f32,
        tilt: Vec2,
        tool: PenTool,
        modifiers: Modifiers,
    },
    PenUp {
        button: PointerButton,
        pos: Point,
        pressure: f32,
        modifiers: Modifiers,
    },

    TouchDown {
        id: u64,
        pos: Point,
        force: Option<f32>,
        modifiers: Modifiers,
    },
    TouchMove {
        id: u64,
        pos: Point,
        force: Option<f32>,
        modifiers: Modifiers,
    },
    TouchUp {
        id: u64,
        pos: Point,
        modifiers: Modifiers,
    },

    Pinch {
        delta: f32,
        pos: Point,
        modifiers: Modifiers,
    },
}

impl PointerEvent {
    pub fn pos(self) -> Point {
        match self {
            Self::Move { pos, .. }
            | Self::Down { pos, .. }
            | Self::Up { pos, .. }
            | Self::Wheel { pos, .. }
            | Self::PenMove { pos, .. }
            | Self::PenDown { pos, .. }
            | Self::PenUp { pos, .. }
            | Self::TouchDown { pos, .. }
            | Self::TouchMove { pos, .. }
            | Self::TouchUp { pos, .. }
            | Self::Pinch { pos, .. } => pos,
            Self::Cancel => Point::ZERO,
        }
    }

    pub fn pressure(self) -> f32 {
        match self {
            Self::PenMove { pressure, .. }
            | Self::PenDown { pressure, .. }
            | Self::PenUp { pressure, .. } => pressure.clamp(0.0, 1.0),
            Self::TouchDown { force, .. } | Self::TouchMove { force, .. } => {
                force.unwrap_or(1.0).clamp(0.0, 1.0)
            }
            Self::Cancel => 0.0,
            _ => 1.0,
        }
    }

    pub fn tool(self) -> PenTool {
        match self {
            Self::PenMove { tool, .. } | Self::PenDown { tool, .. } => tool,
            Self::PenUp { .. } => PenTool::Pen,
            Self::TouchDown { .. } | Self::TouchMove { .. } | Self::TouchUp { .. } => {
                PenTool::Touch
            }
            _ => PenTool::Mouse,
        }
    }

    pub fn touch_id(self) -> Option<u64> {
        match self {
            Self::TouchDown { id, .. } | Self::TouchMove { id, .. } | Self::TouchUp { id, .. } => {
                Some(id)
            }
            _ => None,
        }
    }

    pub fn modifiers(self) -> Modifiers {
        match self {
            Self::Move { modifiers, .. }
            | Self::Down { modifiers, .. }
            | Self::Up { modifiers, .. }
            | Self::Wheel { modifiers, .. }
            | Self::PenMove { modifiers, .. }
            | Self::PenDown { modifiers, .. }
            | Self::PenUp { modifiers, .. }
            | Self::TouchDown { modifiers, .. }
            | Self::TouchMove { modifiers, .. }
            | Self::TouchUp { modifiers, .. }
            | Self::Pinch { modifiers, .. } => modifiers,
            Self::Cancel => Modifiers::NONE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyEvent {
    pub key: Key,

    pub modifiers: Modifiers,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ImeEvent {
    Enabled,

    Preedit {
        value: String,

        cursor: Option<(usize, usize)>,
    },

    Commit(String),

    Disabled,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    Pointer(PointerEvent),

    Key(KeyEvent),

    Ime(ImeEvent),

    FilesHovered {
        paths: Vec<std::path::PathBuf>,
        position: Point,
    },

    FilesDropped {
        paths: Vec<std::path::PathBuf>,
        position: Point,
    },

    FilesCancelled,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DropEffect {
    None,

    #[default]
    Move,

    Copy,

    Link,
}

pub fn effect_for_modifiers(mods: Modifiers) -> DropEffect {
    if mods.contains(Modifiers::CONTROL) {
        DropEffect::Copy
    } else if mods.contains(Modifiers::ALT) {
        DropEffect::Link
    } else {
        DropEffect::Move
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum CursorIcon {
    #[default]
    Default,

    Pointer,

    Grab,

    Grabbing,

    Text,

    ColResize,

    RowResize,

    Crosshair,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPhase {
    Capture,

    Target,

    Bubble,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_char_roundtrip() {
        let k = Key::from_char('a');
        assert_eq!(k.char(), Some('a'));
        assert_ne!(k, Key::from_char('b'));
        assert_eq!(Key::TAB.char(), None);
    }

    #[test]
    fn modifiers_ops() {
        let m = Modifiers::SHIFT | Modifiers::CONTROL;
        assert!(m.contains(Modifiers::SHIFT));
        assert!(!m.contains(Modifiers::ALT));
        assert_eq!(Modifiers::default(), Modifiers::NONE);
    }

    #[test]
    fn drop_effect_for_modifiers() {
        assert_eq!(effect_for_modifiers(Modifiers::NONE), DropEffect::Move);
        assert_eq!(effect_for_modifiers(Modifiers::CONTROL), DropEffect::Copy);
        assert_eq!(effect_for_modifiers(Modifiers::ALT), DropEffect::Link);

        assert_eq!(
            effect_for_modifiers(Modifiers::CONTROL | Modifiers::ALT),
            DropEffect::Copy
        );
    }
}
