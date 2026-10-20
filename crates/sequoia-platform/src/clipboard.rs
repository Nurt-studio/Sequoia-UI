use std::cell::RefCell;

use sequoia_elements::Clipboard;

pub struct OsClipboard {
    inner: RefCell<Option<arboard::Clipboard>>,
}

impl OsClipboard {
    pub fn new() -> Option<Self> {
        match arboard::Clipboard::new() {
            Ok(clip) => Some(Self {
                inner: RefCell::new(Some(clip)),
            }),
            Err(e) => {
                log::debug!("sequoia-platform: clipboard unavailable ({e})");
                None
            }
        }
    }
}

impl Clipboard for OsClipboard {
    fn get_text(&self) -> Option<String> {
        self.inner.borrow_mut().as_mut()?.get_text().ok()
    }

    fn set_text(&self, text: &str) {
        if let Some(clip) = self.inner.borrow_mut().as_mut()
            && let Err(e) = clip.set_text(text.to_owned())
        {
            log::debug!("sequoia-platform: clipboard write failed ({e})");
        }
    }
}
