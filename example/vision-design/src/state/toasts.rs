use super::EditorState;

use crate::state::ui::{ToastEntry, ToastKind};

impl EditorState {
    pub(crate) fn toast(&self, msg: impl Into<String>) {
        self.toast_kind(msg, ToastKind::Info);
    }
    pub(crate) fn toast_warn(&self, msg: impl Into<String>) {
        self.toast_kind(msg, ToastKind::Warn);
    }
    pub(crate) fn toast_error(&self, msg: impl Into<String>) {
        self.toast_kind(msg, ToastKind::Error);
    }

    fn toast_kind(&self, msg: impl Into<String>, kind: ToastKind) {
        let mut seq = self.toast_seq.borrow_mut();
        *seq += 1;
        let id = *seq;
        let msg = msg.into();
        self.toasts.update(|ts| {
            ts.push(ToastEntry { id, msg, kind });
            while ts.len() > 3 {
                ts.remove(0);
            }
        });
    }

    pub(crate) fn dismiss_toast(&self, id: u64) {
        self.toasts.update(|ts| ts.retain(|t| t.id != id));
    }

}
