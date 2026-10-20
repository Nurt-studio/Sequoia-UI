use std::any::Any;
use std::rc::Rc;

use sequoia_core::Rect;
use sequoia_input::{KeyEvent, PointerEvent};

#[derive(Clone)]
pub struct SurfaceDraw {
    pub rect: Rect,

    pub full: Rect,

    pub target: Rc<dyn Any>,
}

pub struct SurfaceInputBinding {
    pub on_pointer: Box<dyn FnMut(&PointerEvent, Rect)>,

    pub on_key: Box<dyn FnMut(&KeyEvent)>,
}
