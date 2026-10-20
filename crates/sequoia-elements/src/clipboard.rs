use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub trait Clipboard {
    fn get_text(&self) -> Option<String>;

    fn set_text(&self, text: &str);
}

#[derive(Clone)]
pub struct SharedClipboard(pub Rc<dyn Clipboard>);

#[derive(Clone, Default)]
pub struct MemClipboard {
    text: Rc<RefCell<String>>,
}

impl MemClipboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(&self) -> String {
        self.text.borrow().clone()
    }
}

impl Clipboard for MemClipboard {
    fn get_text(&self) -> Option<String> {
        let text = self.text.borrow();
        if text.is_empty() {
            None
        } else {
            Some(text.clone())
        }
    }

    fn set_text(&self, text: &str) {
        *self.text.borrow_mut() = text.to_owned();
    }
}

#[derive(Clone, Default)]
pub struct DataClipboard {
    slots: Rc<RefCell<HashMap<TypeId, Rc<dyn Any>>>>,
}

impl DataClipboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<T: Clone + 'static>(&self, value: T) {
        self.slots
            .borrow_mut()
            .insert(TypeId::of::<T>(), Rc::new(value));
    }

    pub fn get<T: Clone + 'static>(&self) -> Option<T> {
        self.slots
            .borrow()
            .get(&TypeId::of::<T>())?
            .downcast_ref::<T>()
            .cloned()
    }

    pub fn clear<T: 'static>(&self) -> bool {
        self.slots.borrow_mut().remove(&TypeId::of::<T>()).is_some()
    }
}

#[derive(Clone)]
pub struct SharedDataClipboard(pub Rc<DataClipboard>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_clipboard_roundtrip() {
        let clip = MemClipboard::new();
        assert_eq!(clip.get_text(), None, "empty at start");
        clip.set_text("hello");
        assert_eq!(clip.get_text().as_deref(), Some("hello"));
        assert_eq!(clip.content(), "hello");
        let shared = SharedClipboard(Rc::new(clip.clone()));
        shared.0.set_text("shared");
        assert_eq!(clip.get_text().as_deref(), Some("shared"), "same clipboard");
    }

    #[test]
    fn data_clipboard_is_typed() {
        #[derive(Clone, PartialEq, Debug)]
        struct Layer(String);
        #[derive(Clone, PartialEq, Debug)]
        struct Other(i32);
        let clip = DataClipboard::new();
        assert_eq!(clip.get::<Layer>(), None, "empty at start");
        clip.set(Layer("hero".into()));
        clip.set(Other(7));
        assert_eq!(clip.get::<Layer>(), Some(Layer("hero".into())));
        assert_eq!(clip.get::<Other>(), Some(Other(7)));

        clip.set(Layer("footer".into()));
        assert_eq!(clip.get::<Layer>(), Some(Layer("footer".into())));
        assert!(clip.clear::<Layer>());
        assert_eq!(clip.get::<Layer>(), None);
        assert!(!clip.clear::<Layer>());

        let shared = SharedDataClipboard(Rc::new(clip.clone()));
        shared.0.set(Other(9));
        assert_eq!(clip.get::<Other>(), Some(Other(9)), "same slots");
    }
}
