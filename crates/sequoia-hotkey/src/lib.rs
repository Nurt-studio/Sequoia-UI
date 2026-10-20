use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

use sequoia_input::{Key, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HotkeyId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,

    UnmappableKey,

    OsRejected,

    BackendGone,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "global shortcuts not supported on this platform"),
            Self::UnmappableKey => write!(f, "the key cannot be used as a system shortcut"),
            Self::OsRejected => write!(f, "the system rejected the shortcut (busy?)"),
            Self::BackendGone => write!(f, "shortcut thread is not responding"),
        }
    }
}

impl std::error::Error for Error {}

pub fn key_to_vk(key: Key) -> Option<u16> {
    if let Some(c) = key.char() {
        let up = c.to_ascii_uppercase();
        if up.is_ascii_alphanumeric() {
            return Some(up as u16);
        }
        return None;
    }
    Some(match key {
        Key::SPACE => 0x20,
        Key::ARROW_LEFT => 0x25,
        Key::ARROW_UP => 0x26,
        Key::ARROW_RIGHT => 0x27,
        Key::ARROW_DOWN => 0x28,
        Key::INSERT => 0x2D,
        Key::DELETE => 0x2E,
        Key::HOME => 0x24,
        Key::END => 0x23,
        Key::PAGE_UP => 0x21,
        Key::PAGE_DOWN => 0x22,
        Key::F1 => 0x70,
        Key::F2 => 0x71,
        Key::F3 => 0x72,
        Key::F4 => 0x73,
        Key::F5 => 0x74,
        Key::F6 => 0x75,
        Key::F7 => 0x76,
        Key::F8 => 0x77,
        Key::F9 => 0x78,
        Key::F10 => 0x79,
        Key::F11 => 0x7A,
        Key::F12 => 0x7B,
        _ => return None,
    })
}

pub(crate) fn modifiers_to_native(mods: Modifiers) -> u32 {
    let mut out = 0u32;
    if mods.contains(Modifiers::ALT) {
        out |= 0x0001;
    }
    if mods.contains(Modifiers::CONTROL) {
        out |= 0x0002;
    }
    if mods.contains(Modifiers::SHIFT) {
        out |= 0x0004;
    }
    if mods.contains(Modifiers::SUPER) {
        out |= 0x0008;
    }
    out
}

enum BackendMsg {
    Register {
        id: u32,
        mods: u32,
        vk: u16,
        reply: Sender<Result<(), Error>>,
    },
    Unregister {
        id: u32,
    },
    Shutdown,
}

pub struct GlobalHotkeys {
    next_id: u32,
    registered: HashMap<HotkeyId, (Key, Modifiers)>,
    backend_tx: Option<Sender<BackendMsg>>,
    fired_rx: Receiver<HotkeyId>,
    #[allow(dead_code)]
    fired_tx: Sender<HotkeyId>,
    #[allow(dead_code)]
    backend_thread_id: Option<u32>,
    _thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(windows)]
fn kick(thread_id: u32) {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_APP};
    unsafe {
        let _ = PostThreadMessageW(thread_id, WM_APP + 77, WPARAM(0), LPARAM(0));
    }
}

impl GlobalHotkeys {
    pub fn new() -> Result<Self, Error> {
        let (fired_tx, fired_rx) = std::sync::mpsc::channel();
        #[cfg(windows)]
        {
            let (tx, rx) = std::sync::mpsc::channel::<BackendMsg>();
            let (ready_tx, ready_rx) = std::sync::mpsc::channel::<u32>();
            let fired = fired_tx.clone();
            let handle = std::thread::Builder::new()
                .name("sequoia-hotkeys".into())
                .spawn(move || windows_backend(rx, fired, ready_tx))
                .map_err(|_| Error::BackendGone)?;

            let thread_id = ready_rx.recv().map_err(|_| Error::BackendGone)?;
            Ok(Self {
                next_id: 1,
                registered: HashMap::new(),
                backend_tx: Some(tx),
                fired_rx,
                fired_tx,
                backend_thread_id: Some(thread_id),
                _thread: Some(handle),
            })
        }
        #[cfg(not(windows))]
        {
            let _ = fired_tx;
            Err(Error::Unsupported)
        }
    }

    pub fn stub() -> Self {
        let (fired_tx, fired_rx) = std::sync::mpsc::channel();
        Self {
            next_id: 1,
            registered: HashMap::new(),
            backend_tx: None,
            fired_rx,
            fired_tx,
            backend_thread_id: None,
            _thread: None,
        }
    }

    fn alloc_id(&mut self) -> Result<u32, Error> {
        for _ in 0..=u32::MAX {
            let id = self.next_id;
            self.next_id = self.next_id.checked_add(1).unwrap_or(1);
            if self.next_id == 0 {
                self.next_id = 1;
            }
            if id != 0 {
                return Ok(id);
            }
        }
        Err(Error::OsRejected)
    }

    pub fn register(&mut self, key: Key, mods: Modifiers) -> Result<HotkeyId, Error> {
        if self.backend_tx.is_none() {
            key_to_vk(key).ok_or(Error::UnmappableKey)?;
            let id = HotkeyId(self.alloc_id()?);
            self.registered.insert(id, (key, mods));
            return Ok(id);
        }
        let vk = key_to_vk(key).ok_or(Error::UnmappableKey)?;
        let id_raw = self.alloc_id()?;
        let tx = self.backend_tx.as_ref().ok_or(Error::BackendGone)?;
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        tx.send(BackendMsg::Register {
            id: id_raw,
            mods: modifiers_to_native(mods),
            vk,
            reply: reply_tx,
        })
        .map_err(|_| Error::BackendGone)?;
        #[cfg(windows)]
        if let Some(tid) = self.backend_thread_id {
            kick(tid);
        }
        reply_rx.recv().map_err(|_| Error::BackendGone)??;
        let id = HotkeyId(id_raw);
        self.registered.insert(id, (key, mods));
        Ok(id)
    }

    pub fn unregister(&mut self, id: HotkeyId) -> bool {
        if self.registered.remove(&id).is_none() {
            return false;
        }
        if let Some(tx) = &self.backend_tx {
            let _ = tx.send(BackendMsg::Unregister { id: id.0 });
            #[cfg(windows)]
            if let Some(tid) = self.backend_thread_id {
                kick(tid);
            }
        }
        true
    }

    pub fn drain(&self) -> Vec<HotkeyId> {
        let mut out = Vec::new();
        while let Ok(id) = self.fired_rx.try_recv() {
            out.push(id);
        }
        out
    }

    pub fn registered(&self) -> &HashMap<HotkeyId, (Key, Modifiers)> {
        &self.registered
    }
}

impl Drop for GlobalHotkeys {
    fn drop(&mut self) {
        if let Some(tx) = self.backend_tx.take() {
            let _ = tx.send(BackendMsg::Shutdown);
            #[cfg(windows)]
            if let Some(tid) = self.backend_thread_id {
                kick(tid);
            }
        }
    }
}

#[cfg(windows)]
fn windows_backend(rx: Receiver<BackendMsg>, fired: Sender<HotkeyId>, ready: Sender<u32>) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, PM_NOREMOVE, PeekMessageW};

    const WM_HOTKEY: u32 = 0x0312;
    const NULL_HWND: HWND = HWND(0);

    let mut peek = MSG::default();
    unsafe {
        let _ = PeekMessageW(&mut peek, NULL_HWND, 0, 0, PM_NOREMOVE);
    }
    let _ = ready.send(unsafe { GetCurrentThreadId() });
    let mut live: Vec<u32> = Vec::new();
    loop {
        while let Ok(m) = rx.try_recv() {
            match m {
                BackendMsg::Register {
                    id,
                    mods,
                    vk,
                    reply,
                } => {
                    let ok = unsafe {
                        RegisterHotKey(NULL_HWND, id as i32, HOT_KEY_MODIFIERS(mods), vk as u32)
                    };
                    match ok {
                        Ok(()) => {
                            live.push(id);
                            let _ = reply.send(Ok(()));
                        }
                        Err(_) => {
                            let _ = reply.send(Err(Error::OsRejected));
                        }
                    }
                }
                BackendMsg::Unregister { id } => {
                    unsafe {
                        let _ = UnregisterHotKey(NULL_HWND, id as i32);
                    }
                    live.retain(|l| *l != id);
                }
                BackendMsg::Shutdown => {
                    for id in live.drain(..) {
                        unsafe {
                            let _ = UnregisterHotKey(NULL_HWND, id as i32);
                        }
                    }
                    return;
                }
            }
        }
        let mut m = MSG::default();
        let got = unsafe { GetMessageW(&mut m, NULL_HWND, 0, 0) };
        if got.0 == 0 || got.0 == -1 {
            for id in live.drain(..) {
                unsafe {
                    let _ = UnregisterHotKey(NULL_HWND, id as i32);
                }
            }
            return;
        }
        if m.message == WM_HOTKEY {
            let _ = fired.send(HotkeyId(m.wParam.0 as u32));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_mapping_covers_letters_digits_and_fkeys() {
        assert_eq!(key_to_vk(Key::from_char('a')), Some(0x41));
        assert_eq!(key_to_vk(Key::from_char('5')), Some(0x35));
        assert_eq!(key_to_vk(Key::F5), Some(0x74));
        assert_eq!(key_to_vk(Key::ENTER), None);
        assert_eq!(key_to_vk(Key::from_char('ą')), None);
    }

    #[test]
    fn modifiers_map_to_win_flags() {
        assert_eq!(
            modifiers_to_native(Modifiers::CONTROL | Modifiers::SHIFT),
            0x0002 | 0x0004
        );
        assert_eq!(modifiers_to_native(Modifiers::NONE), 0);
    }

    #[test]
    fn stub_registers_and_unregisters_without_os() {
        let mut hk = GlobalHotkeys::stub();
        let id = hk
            .register(Key::from_char('m'), Modifiers::CONTROL)
            .unwrap();
        assert!(hk.registered().contains_key(&id));
        assert!(hk.drain().is_empty());
        assert!(hk.unregister(id));
        assert!(!hk.unregister(id));
        assert!(hk.register(Key::ENTER, Modifiers::NONE).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn message_queue_exists_right_after_new() {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_APP};

        let hk = match GlobalHotkeys::new() {
            Ok(hk) => hk,

            Err(_) => return,
        };
        let tid = hk
            .backend_thread_id
            .expect("backend thread right after new()");

        let posted = unsafe { PostThreadMessageW(tid, WM_APP + 77, WPARAM(0), LPARAM(0)) };
        assert!(
            posted.is_ok(),
            "the backend thread's message queue must exist immediately after new()"
        );
    }
}
