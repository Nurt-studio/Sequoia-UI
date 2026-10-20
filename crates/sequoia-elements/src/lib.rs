#![forbid(unsafe_code)]

mod avatar;
mod badge;
mod buffer;
mod button;
mod checkbox;
mod chip;
mod clipboard;
pub mod color_picker;
mod command;
mod container;
mod context_menu;
mod dialog;
mod divider;
mod drag_value;
mod draggable;
mod drawer;
mod dropdown;
mod droppable;
mod floating_menu;
mod form;
mod kbd;
mod modal;
mod popup;
pub mod primitives;
mod progress;
mod resize_handle;
mod sheet;
pub mod slider;
pub mod slider1d;
mod svpad;
mod switch;
mod tab_strip;
mod text_area;
mod text_field;
mod toast;
mod tooltip;
mod underline;

pub use avatar::{Avatar, avatar_color_for, initials_for};
pub use badge::{Badge, badge_label};
pub use button::{BUTTON_BLEND_RATE, Button};
pub use checkbox::Checkbox;
pub use chip::{Chip, remove_hit_rect};
pub use clipboard::{Clipboard, DataClipboard, MemClipboard, SharedClipboard, SharedDataClipboard};
pub use color_picker::{ColorPicker, PIPETTE_SVG, Picker, PickerFloat};
pub use command::{Command, CommandPalette, CommandRegistry, MenuBar};
pub use container::{ClickTrigger, Container};
pub use context_menu::ContextMenu;
pub use dialog::Dialog;
pub use divider::{Divider, DividerAxis, divider_line_rect};
pub use drag_value::DragValue;
pub use draggable::Draggable;
pub use drawer::{Drawer, DrawerSide};
pub use dropdown::DropdownButton;
pub use droppable::Droppable;
pub use floating_menu::{CustomMenuItem, FloatingMenu, MENU_WIDTH, MenuItem};
pub use form::{DateField, Field, Form, MultiSelect, NumberField, ValidField, parse_iso_date};
pub use kbd::Kbd;
pub use modal::{DEFAULT_SCRIM, Modal, ModalScrim};
pub use popup::PopupState;
pub use primitives::{Deco, PRESS_BLEND_RATE};
pub use progress::{ProgressBar, Skeleton, Spinner};
pub use resize_handle::ResizeHandle;
pub use sequoia_style::{
    AvatarStyle, BadgeStyle, ButtonStyle, CheckboxStyle, ChipStyle, ContextMenuStyle,
    DIALOG_DANGER, DialogStyle, DividerStyle, DragValueStyle, DrawerStyle, FORM_ERROR, FormStyle,
    KbdStyle, ModalStyle, ResizeHandleStyle, SheetStyle, SliderStyle, SplitPanelStyle, SwitchStyle,
    TableStyle, TextAreaStyle, TextFieldStyle, ToastStyle, TooltipStyle, TreeStyle,
};
pub use sheet::{Sheet, SheetSide};
pub use slider::{FILL_COLOR, Slider, THUMB_CENTER, THUMB_OUTLINE, TRACK_COLOR};
pub use slider1d::{
    HEIGHT as SLIDER1D_HEIGHT, KNOB_R as SLIDER1D_KNOB_R, Slider1D, TRACK_H as SLIDER1D_TRACK_H,
};
pub use svpad::{HEIGHT as SVPAD_HEIGHT, KNOB_R as SVPAD_KNOB_R, SvPad};
pub use switch::Switch;
pub use tab_strip::{TabStrip, TabTear};
pub use text_area::TextArea;
pub use text_field::TextField;
pub use toast::{DEFAULT_MAX_VISIBLE, DEFAULT_TTL, Toast, ToastKind, ToastQueue, ToastStack};
pub use tooltip::Tooltip;

pub mod prelude {
    pub use crate::avatar::{Avatar, avatar_color_for, initials_for};
    pub use crate::badge::{Badge, badge_label};
    pub use crate::button::Button;
    pub use crate::checkbox::Checkbox;
    pub use crate::chip::{Chip, remove_hit_rect};
    pub use crate::clipboard::{
        Clipboard, DataClipboard, MemClipboard, SharedClipboard, SharedDataClipboard,
    };
    pub use crate::color_picker::{ColorPicker, PIPETTE_SVG, Picker, PickerFloat};
    pub use crate::command::{Command, CommandPalette, CommandRegistry, MenuBar};
    pub use crate::container::{ClickTrigger, Container};
    pub use crate::context_menu::ContextMenu;
    pub use crate::dialog::Dialog;
    pub use crate::divider::{Divider, DividerAxis, divider_line_rect};
    pub use crate::drag_value::DragValue;
    pub use crate::draggable::Draggable;
    pub use crate::drawer::{Drawer, DrawerSide};
    pub use crate::dropdown::DropdownButton;
    pub use crate::droppable::Droppable;
    pub use crate::floating_menu::{CustomMenuItem, FloatingMenu, MenuItem};
    pub use crate::form::{DateField, Field, Form, MultiSelect, NumberField, ValidField};
    pub use crate::kbd::Kbd;
    pub use crate::modal::{Modal, ModalScrim};
    pub use crate::popup::PopupState;
    pub use crate::primitives::{Deco, PRESS_BLEND_RATE};
    pub use crate::progress::{ProgressBar, Skeleton, Spinner};
    pub use crate::resize_handle::ResizeHandle;
    pub use crate::sheet::{Sheet, SheetSide};
    pub use crate::slider::{FILL_COLOR, Slider, THUMB_CENTER, THUMB_OUTLINE, TRACK_COLOR};
    pub use crate::slider1d::Slider1D;
    pub use crate::svpad::SvPad;
    pub use crate::switch::Switch;
    pub use crate::tab_strip::{TabStrip, TabTear};
    pub use crate::text_area::TextArea;
    pub use crate::text_field::TextField;
    pub use crate::toast::{Toast, ToastKind, ToastQueue, ToastStack};
    pub use crate::tooltip::Tooltip;
    pub use sequoia_input::{
        CursorIcon, Event, EventPhase, Key, KeyEvent, Modifiers, PointerButton, PointerEvent,
    };
    pub use sequoia_layout::{Axis, ContainerStyle, Indents, LayoutCtx, LayoutStyle};
    pub use sequoia_style::{
        AvatarStyle, BadgeStyle, ButtonStyle, CheckboxStyle, ChipStyle, ContextMenuStyle,
        DIALOG_DANGER, DialogStyle, DividerStyle, DragValueStyle, DrawerStyle, FORM_ERROR,
        FormStyle, KbdStyle, ModalStyle, ResizeHandleStyle, SheetStyle, SliderStyle,
        SplitPanelStyle, SwitchStyle, TableStyle, TextAreaStyle, TextFieldStyle, ToastStyle,
        TooltipStyle, TreeStyle,
    };
    pub use sequoia_view::prelude::*;
}
