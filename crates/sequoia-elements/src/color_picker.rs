use sequoia_core::{BrushStop, Color, Fill, Length, Point, Rect};
use sequoia_input::{CursorIcon, Event, PointerButton, PointerEvent};
use sequoia_layout::{Align, Inset, LayoutStyle, Position};
use sequoia_reactive::{EffectScope, Environment, Signal, batch, effect};
use sequoia_style::VisualState;
use sequoia_view::{
    AnyView, Canvas, Column, EventCtx, Orientation, Row, Spacer, Text, Unit, View, Wrap,
};

use crate::button::Button;
use crate::clipboard::SharedClipboard;
use crate::container::Container;
use crate::context_menu::ContextMenu;
use crate::drag_value::DragValue;
use crate::dropdown::DropdownButton;
use crate::floating_menu::FloatingMenu;
use crate::slider1d::Slider1D;
use crate::svpad::SvPad;
use crate::text_field::TextField;
use crate::toast::ToastQueue;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

const BG_PANEL: Color = Color::rgb8(0x26, 0x26, 0x26);
const BG_HEADER: Color = Color::rgb8(0x1E, 0x1E, 0x1E);
const BG_INPUT: Color = Color::rgb8(0x2A, 0x2A, 0x2A);
const BG_SECTION: Color = Color::rgb8(0x30, 0x30, 0x30);
const BORDER_PANEL: Color = Color::rgb8(0x00, 0x00, 0x00);
const BORDER_HAIRLINE: Color = Color::rgb8(0x1F, 0x1F, 0x1F);
const BORDER_CONTROL: Color = Color::rgb8(0x45, 0x45, 0x45);
const HAIR_TRACK: Color = Color::rgb8(0x1A, 0x1A, 0x1A);
const TEXT_TITLE: Color = Color::rgb8(0xED, 0xED, 0xED);
const TEXT_PRIMARY: Color = Color::rgb8(0xD4, 0xD4, 0xD4);
const TEXT_SECONDARY: Color = Color::rgb8(0x9A, 0x9A, 0x9A);
const ACCENT_ACTIVE: Color = Color::rgb8(0xE8, 0x84, 0x2C);

pub const PIPETTE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M10.763 8.177a2 2 0 0 1-.263.323l-4.621 4.621A3 3 0 0 1 3.758 14H3l-.793.793a.708.708 0 0 1-1-1L2 13v-.758a3 3 0 0 1 .879-2.12L7.5 5.5q.151-.15.322-.264zM12.879 1a2.121 2.121 0 0 1 1.5 3.621L12.5 6.5l.5.5a.707.707 0 0 1-1 1L8 4a.707.707 0 0 1 1-1l.5.5l1.879-1.879a2.12 2.12 0 0 1 1.5-.621"/></svg>"#;

const DEFAULT_SWATCHES: [&str; 8] = [
    "E5484D", "F76B15", "FFC53D", "46A758", "12A594", "35C2F1", "6E56CF", "E93D82",
];

const DEFAULT_H: f32 = 201.0;
const DEFAULT_S: f32 = 0.52;
const DEFAULT_V: f32 = 0.92;
const HIST_LIMIT: usize = 100;
const HIST_WINDOW: Duration = Duration::from_millis(800);

fn clampf(v: f32, lo: f32, hi: f32) -> f32 {
    v.clamp(lo, hi)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0);
    let f = |n: f32| {
        let k = (n + h / 60.0) % 6.0;
        v - v * s * 0f32.max((k.min(4.0 - k)).min(1.0))
    };
    let q = |c: f32| (clampf(c, 0.0, 1.0) * 255.0).round() as u8;
    (q(f(5.0)), q(f(3.0)), q(f(1.0)))
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    let mut h = 0.0;
    if d > 0.0 {
        if mx == r {
            h = ((g - b) / d) % 6.0;
        } else if mx == g {
            h = (b - r) / d + 2.0;
        } else {
            h = (r - g) / d + 4.0;
        }
        h *= 60.0;
        if h < 0.0 {
            h += 360.0;
        }
    }
    (h, if mx > 0.0 { d / mx } else { 0.0 }, mx)
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    let l = (mx + mn) / 2.0;
    let mut h = 0.0;
    let mut s = 0.0;
    if d > 0.0 {
        s = d / (1.0 - (2.0 * l - 1.0).abs());
        if mx == r {
            h = ((g - b) / d) % 6.0;
        } else if mx == g {
            h = (b - r) / d + 2.0;
        } else {
            h = (r - g) / d + 4.0;
        }
        h *= 60.0;
        if h < 0.0 {
            h += 360.0;
        }
    }
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0);
    let a = s * l.min(1.0 - l);
    let f = |n: f32| {
        let k = (n + h / 30.0) % 12.0;
        l - a * (-1f32).max((k - 3.0).min(9.0 - k).min(1.0))
    };
    let q = |c: f32| (clampf(c, 0.0, 1.0) * 255.0).round() as u8;
    (q(f(0.0)), q(f(8.0)), q(f(4.0)))
}

fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("{r:02X}{g:02X}{b:02X}")
}

fn parse_hex(s: &str) -> Option<String> {
    let mut clean = s.trim();
    if let Some(rest) = clean.strip_prefix('#') {
        clean = rest;
    } else if let Some(rest) = clean
        .strip_prefix("0x")
        .or_else(|| clean.strip_prefix("0X"))
    {
        clean = rest;
    }
    if clean.is_empty() || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let rgb = match clean.len() {
        3 => clean.chars().flat_map(|c| [c, c]).collect::<String>(),
        4 => clean[..3].chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => clean.to_string(),
        8 => clean[..6].to_string(),
        _ => return None,
    };
    Some(rgb.to_uppercase())
}

fn hex_to_rgb(hx: &str) -> (u8, u8, u8) {
    let n = |i: usize| u8::from_str_radix(&hx[i..i + 2], 16).unwrap_or(0);
    (n(0), n(2), n(4))
}

fn color_components(s: &str) -> Vec<(f32, bool)> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let negative = bytes[i] == b'-';
        let start_digit = if negative { i + 1 } else { i };
        if start_digit >= bytes.len() || !bytes[start_digit].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        i = start_digit;
        let mut dot = false;
        while i < bytes.len() {
            match bytes[i] {
                b'0'..=b'9' => i += 1,
                b'.' if !dot => {
                    dot = true;
                    i += 1;
                }
                _ => break,
            }
        }
        let Ok(value) = s[start..i].parse::<f32>() else {
            continue;
        };
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        let percent = j < bytes.len() && bytes[j] == b'%';
        out.push((value, percent));
    }
    out
}

fn channel_byte(value: f32, percent: bool) -> u8 {
    let v = if percent { value * 2.55 } else { value };
    clampf(v, 0.0, 255.0).round() as u8
}

fn parse_color_text(s: &str) -> Option<(u8, u8, u8)> {
    if let Some(hx) = parse_hex(s) {
        return Some(hex_to_rgb(&hx));
    }
    let low = s.trim().to_ascii_lowercase();
    let nums = color_components(&low);
    if nums.len() < 3 {
        return None;
    }
    if low.starts_with("rgb") {
        return Some((
            channel_byte(nums[0].0, nums[0].1),
            channel_byte(nums[1].0, nums[1].1),
            channel_byte(nums[2].0, nums[2].1),
        ));
    }
    if low.starts_with("hsl") {
        let h = nums[0].0;
        let sat = clampf(nums[1].0, 0.0, 100.0) / 100.0;
        let light = clampf(nums[2].0, 0.0, 100.0) / 100.0;
        return Some(hsl_to_rgb(h, sat, light));
    }
    None
}

#[allow(dead_code)]
fn fmt_val(v: f32, max: f32) -> String {
    if max == 255.0 || max == 360.0 {
        return format!("{}", v.round() as i32);
    }
    let r = v.round();
    if (v - r).abs() < 0.05 {
        format!("{r}")
    } else {
        format!("{:.1}", (v * 10.0).round() / 10.0)
    }
}

struct Rec {
    last_hex: String,
    last_push: Option<Instant>,
    in_history: bool,
}

#[derive(Clone, Copy)]
struct PickerDrag {
    origin: Point,
    start: (f32, f32),
}

#[derive(Clone)]
pub struct ColorPicker {
    hue: Signal<f32>,
    sat: Signal<f32>,
    val: Signal<f32>,
    model: Signal<usize>,
    ref_hex: Signal<String>,
    hex_text: Signal<String>,
    editing_hex: Signal<bool>,
    swatches: Signal<Vec<String>>,
    undo: Signal<Vec<String>>,
    redo: Signal<Vec<String>>,
    active: Signal<Option<usize>>,
    closed: Signal<bool>,
    ch: [Signal<f32>; 3],
    toasts: ToastQueue,
    rec: Rc<RefCell<Rec>>,
    on_close: Rc<RefCell<Option<Rc<dyn Fn()>>>>,

    pos: Signal<(f32, f32)>,
    drag: Rc<Cell<Option<PickerDrag>>>,
    effects: Rc<RefCell<Option<EffectScope>>>,
}

pub type Picker = ColorPicker;

impl Default for ColorPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorPicker {
    pub fn new() -> Self {
        let (h, s, v) = (DEFAULT_H, DEFAULT_S, DEFAULT_V);
        let (r, g, b) = hsv_to_rgb(h, s, v);
        let start_hex = rgb_to_hex(r, g, b);
        let p = Self {
            hue: Signal::new(h),
            sat: Signal::new(s),
            val: Signal::new(v),
            model: Signal::new(0),
            ref_hex: Signal::new(start_hex.clone()),
            hex_text: Signal::new(start_hex.clone()),
            editing_hex: Signal::new(false),
            swatches: Signal::new(DEFAULT_SWATCHES.iter().map(|s| s.to_string()).collect()),
            undo: Signal::new(Vec::new()),
            redo: Signal::new(Vec::new()),
            active: Signal::new(None),
            closed: Signal::new(false),
            ch: [Signal::new(0.0), Signal::new(0.0), Signal::new(0.0)],
            toasts: ToastQueue::new(),
            rec: Rc::new(RefCell::new(Rec {
                last_hex: start_hex,
                last_push: None,
                in_history: false,
            })),
            on_close: Rc::new(RefCell::new(None)),
            pos: Signal::new((90.0, 70.0)),
            drag: Rc::new(Cell::new(None)),
            effects: Rc::new(RefCell::new(None)),
        };
        p.sync_channels();
        p
    }

    pub fn set_hex(&self, hx: &str) {
        self.apply_hex(hx);
    }

    pub fn hsv_signals(&self) -> (Signal<f32>, Signal<f32>, Signal<f32>) {
        (self.hue.clone(), self.sat.clone(), self.val.clone())
    }

    pub fn toasts(&self) -> ToastQueue {
        self.toasts.clone()
    }

    pub fn pos_signal(&self) -> Signal<(f32, f32)> {
        self.pos.clone()
    }

    pub fn set_on_close(&self, f: impl Fn() + 'static) {
        *self.on_close.borrow_mut() = Some(Rc::new(f));
    }

    fn close(&self) {
        self.closed.set(true);
        if let Some(f) = self.on_close.borrow().clone() {
            f();
        }
    }

    pub fn open(&self) {
        self.closed.set(false);
    }

    pub fn is_open(&self) -> bool {
        !self.closed.get_untracked()
    }

    pub fn cur_rgb(&self) -> (u8, u8, u8) {
        hsv_to_rgb(
            self.hue.get_untracked(),
            self.sat.get_untracked(),
            self.val.get_untracked(),
        )
    }

    pub fn cur_hex(&self) -> String {
        let (r, g, b) = self.cur_rgb();
        rgb_to_hex(r, g, b)
    }

    pub fn cur_color(&self) -> Color {
        let (r, g, b) = self.cur_rgb();
        Color::rgb8(r, g, b)
    }

    fn set_rgb(&self, r: u8, g: u8, b: u8) {
        let (nh, ns, nv) = rgb_to_hsv(r, g, b);
        batch(|| {
            if nv > 0.0001 {
                if ns > 0.0001 {
                    self.hue.set(nh);
                }
                self.sat.set(ns);
            }
            self.val.set(nv);
        });
    }

    fn apply_hex(&self, hx: &str) {
        if let Some(n) = parse_hex(hx) {
            let (r, g, b) = hex_to_rgb(&n);
            self.set_rgb(r, g, b);
        }
    }

    fn channel_value(&self, idx: usize) -> f32 {
        match self.model.get_untracked() {
            0 => {
                let (r, g, b) = self.cur_rgb();
                match idx {
                    0 => r as f32,
                    1 => g as f32,
                    _ => b as f32,
                }
            }
            1 => match idx {
                0 => self.hue.get_untracked(),
                1 => self.sat.get_untracked() * 100.0,
                _ => self.val.get_untracked() * 100.0,
            },
            _ => {
                let (r, g, b) = self.cur_rgb();
                let (h, s, l) = rgb_to_hsl(r, g, b);
                let _ = h;
                match idx {
                    0 => self.hue.get_untracked(),
                    1 => s * 100.0,
                    _ => l * 100.0,
                }
            }
        }
    }

    fn apply_channel(&self, idx: usize, v: f32) {
        batch(|| match self.model.get_untracked() {
            0 => {
                let (r, g, b) = self.cur_rgb();
                let q = clampf(v, 0.0, 255.0).round() as u8;
                match idx {
                    0 => self.set_rgb(q, g, b),
                    1 => self.set_rgb(r, q, b),
                    _ => self.set_rgb(r, g, q),
                }
            }
            1 => match idx {
                0 => self.hue.set(v.rem_euclid(360.0)),
                1 => self.sat.set(clampf(v, 0.0, 100.0) / 100.0),
                _ => self.val.set(clampf(v, 0.0, 100.0) / 100.0),
            },
            _ => {
                if idx == 0 {
                    self.hue.set(v.rem_euclid(360.0));
                } else {
                    let (r, g, b) = self.cur_rgb();
                    let (hh, mut ss, mut ll) = rgb_to_hsl(r, g, b);
                    if idx == 1 {
                        ss = clampf(v, 0.0, 100.0) / 100.0;
                    } else {
                        ll = clampf(v, 0.0, 100.0) / 100.0;
                    }
                    let (nr, ng, nb) = hsl_to_rgb(hh, ss, ll);
                    self.set_rgb(nr, ng, nb);
                }
            }
        });
    }

    fn sync_channels(&self) {
        for i in 0..3 {
            let want = self.channel_value(i);
            if (self.ch[i].get_untracked() - want).abs() > 1e-4 {
                self.ch[i].set(want);
            }
        }
        if !self.editing_hex.get_untracked() {
            let hx = self.cur_hex();
            if self.hex_text.get_untracked() != hx {
                self.hex_text.set(hx);
            }
        }
    }

    fn note_change(&self, new_hex: String) {
        let mut r = self.rec.borrow_mut();
        if r.in_history || new_hex == r.last_hex {
            return;
        }
        let now = Instant::now();
        let coalesce = r
            .last_push
            .map(|t| now.duration_since(t) < HIST_WINDOW)
            .unwrap_or(false);
        if !coalesce {
            let prev = r.last_hex.clone();
            self.undo.update(|u| {
                if u.last() != Some(&prev) {
                    u.push(prev);
                    if u.len() > HIST_LIMIT {
                        u.remove(0);
                    }
                }
            });
            self.redo.set(Vec::new());
        }
        r.last_push = Some(now);
        r.last_hex = new_hex;
    }

    pub fn do_undo(&self) {
        let mut u = self.undo.get_untracked();
        let Some(prev) = u.pop() else {
            return;
        };
        let cur = self.cur_hex();
        self.redo.update(|r| r.push(cur));
        self.undo.set(u);
        self.rec.borrow_mut().in_history = true;
        self.apply_hex(&prev);
        self.sync_channels();
        let mut r = self.rec.borrow_mut();
        r.last_hex = prev;
        r.last_push = None;
        r.in_history = false;
    }

    pub fn do_redo(&self) {
        let mut rd = self.redo.get_untracked();
        let Some(next) = rd.pop() else {
            return;
        };
        let cur = self.cur_hex();
        self.undo.update(|u| u.push(cur));
        self.redo.set(rd);
        self.rec.borrow_mut().in_history = true;
        self.apply_hex(&next);
        self.sync_channels();
        let mut r = self.rec.borrow_mut();
        r.last_hex = next;
        r.last_push = None;
        r.in_history = false;
    }

    pub fn set_reference(&self) {
        self.ref_hex.set(self.cur_hex());
        self.toast("Reference color set");
    }

    fn revert(&self) {
        let rf = self.ref_hex.get_untracked();
        if rf == self.cur_hex() {
            return;
        }
        self.apply_hex(&rf);
        self.toast("Reverted to reference");
    }

    fn with_clip(&self, env: &Environment, f: impl FnOnce(&SharedClipboard)) -> bool {
        match env.get::<SharedClipboard>() {
            Some(c) => {
                f(&c);
                true
            }
            None => false,
        }
    }

    fn toast(&self, msg: impl Into<String>) {
        self.toasts.info(msg, "");
    }

    fn copy_hex(&self, env: &Environment) {
        let t = format!("#{}", self.cur_hex());
        if self.with_clip(env, |c| c.0.set_text(&t)) {
            self.toast(format!("Copied {t}"));
        } else {
            self.toast("Clipboard unavailable");
        }
    }

    fn copy_rgb(&self, env: &Environment) {
        let (r, g, b) = self.cur_rgb();
        let t = format!("rgb({r}, {g}, {b})");
        if self.with_clip(env, |c| c.0.set_text(&t)) {
            self.toast(format!("Copied {t}"));
        } else {
            self.toast("Clipboard unavailable");
        }
    }

    fn copy_hsl(&self, env: &Environment) {
        let (r, g, b) = self.cur_rgb();
        let (h, s, l) = rgb_to_hsl(r, g, b);
        let t = format!(
            "hsl({}, {}%, {}%)",
            h.round() as i32,
            (s * 100.0).round() as i32,
            (l * 100.0).round() as i32
        );
        if self.with_clip(env, |c| c.0.set_text(&t)) {
            self.toast(format!("Copied {t}"));
        } else {
            self.toast("Clipboard unavailable");
        }
    }

    fn paste(&self, env: &Environment) {
        let Some(clip) = env.get::<SharedClipboard>() else {
            self.toast("Clipboard access denied");
            return;
        };
        let Some(txt) = clip.0.get_text() else {
            self.toast("Clipboard access denied");
            return;
        };
        let txt = txt.trim().to_string();
        if let Some((r, g, b)) = parse_color_text(&txt) {
            self.set_rgb(r, g, b);
            self.toast(format!("Pasted #{}", self.cur_hex()));
            return;
        }
        self.toast("No color found in clipboard");
    }

    fn sw_add(&self) {
        let hx = self.cur_hex();
        let mut sw = self.swatches.get_untracked();
        if sw.contains(&hx) {
            self.toast(format!("#{hx} is already in swatches"));
            return;
        }
        sw.push(hx);
        self.swatches.set(sw);
    }

    fn sw_sort(&self) {
        let mut sw = self.swatches.get_untracked();
        sw.sort_by(|a, b| {
            let (r1, g1, b1) = hex_to_rgb(a);
            let (r2, g2, b2) = hex_to_rgb(b);
            let (h1, s1, v1) = rgb_to_hsv(r1, g1, b1);
            let (h2, s2, v2) = rgb_to_hsv(r2, g2, b2);
            let _ = (s1, v1, s2, v2);
            let g1 = if s1 < 0.06 { 999.0 } else { h1 };
            let g2 = if s2 < 0.06 { 999.0 } else { h2 };
            g1.partial_cmp(&g2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(v2.partial_cmp(&v1).unwrap_or(std::cmp::Ordering::Equal))
        });
        self.swatches.set(sw);
    }

    fn effect_handle(&self) -> Self {
        let mut handle = self.clone();
        handle.effects = Rc::new(RefCell::new(None));
        handle
    }

    pub fn install_effects(&self) {
        if self.effects.borrow().is_some() {
            return;
        }
        self.rec.borrow_mut().last_hex = self.cur_hex();

        let p = self.effect_handle();
        let s1 = effect(move || {
            let _ = (p.hue.get(), p.sat.get(), p.val.get(), p.model.get());
            p.note_change(p.cur_hex());
            p.sync_channels();
        });

        let p = self.effect_handle();
        let s2 = effect(move || {
            let cur = [p.ch[0].get(), p.ch[1].get(), p.ch[2].get()];
            let want = [p.channel_value(0), p.channel_value(1), p.channel_value(2)];
            let eps = if p.model.get_untracked() == 0 {
                0.51
            } else {
                0.02
            };
            let mut hit = None;
            for i in 0..3 {
                if (cur[i] - want[i]).abs() > eps {
                    hit = Some(i);
                    break;
                }
            }
            if let Some(i) = hit {
                p.active.set(Some(i));
                p.apply_channel(i, cur[i]);
            }
        });

        let p = self.effect_handle();
        let s3 = effect(move || {
            let t = p.hex_text.get();
            if t.len() == 6
                && let Some(n) = parse_hex(&t)
                && n != p.cur_hex()
            {
                p.apply_hex(&n);
            }
        });

        let mut scope = EffectScope::new();
        scope.extend([s1, s2, s3]);
        *self.effects.borrow_mut() = Some(scope);
    }

    pub fn uninstall_effects(&self) {
        if let Some(mut scope) = self.effects.borrow_mut().take() {
            scope.dispose_all();
        }
    }
}

fn channel_meta(model: usize, idx: usize) -> (&'static str, f32, f32, usize) {
    match model {
        0 => match idx {
            0 => ("R", 0.0, 255.0, 0),
            1 => ("G", 0.0, 255.0, 0),
            _ => ("B", 0.0, 255.0, 0),
        },
        1 => match idx {
            0 => ("H", 0.0, 360.0, 0),
            1 => ("S", 0.0, 100.0, 1),
            _ => ("V", 0.0, 100.0, 1),
        },
        _ => match idx {
            0 => ("H", 0.0, 360.0, 0),
            1 => ("S", 0.0, 100.0, 1),
            _ => ("L", 0.0, 100.0, 1),
        },
    }
}

fn hue_stops() -> Vec<BrushStop> {
    (0..=6)
        .map(|i| {
            BrushStop::new(
                i as f32 / 6.0,
                Color::from_hsva(i as f32 / 6.0, 1.0, 1.0, 1.0),
            )
        })
        .collect()
}

fn channel_stops(p: &ColorPicker, idx: usize) -> Vec<BrushStop> {
    let h = p.hue.get_untracked();
    let s = p.sat.get_untracked();
    let v = p.val.get_untracked();
    let (r, g, b) = hsv_to_rgb(h, s, v);
    match p.model.get_untracked() {
        0 => {
            let (a, c) = match idx {
                0 => (Color::rgb8(0, g, b), Color::rgb8(255, g, b)),
                1 => (Color::rgb8(r, 0, b), Color::rgb8(r, 255, b)),
                _ => (Color::rgb8(r, g, 0), Color::rgb8(r, g, 255)),
            };
            vec![BrushStop::new(0.0, a), BrushStop::new(1.0, c)]
        }
        1 => match idx {
            0 => hue_stops(),
            1 => vec![
                BrushStop::new(0.0, Color::from_hsva(h / 360.0, 0.0, v, 1.0)),
                BrushStop::new(1.0, Color::from_hsva(h / 360.0, 1.0, v, 1.0)),
            ],
            _ => vec![
                BrushStop::new(0.0, Color::BLACK),
                BrushStop::new(1.0, Color::from_hsva(h / 360.0, s, 1.0, 1.0)),
            ],
        },
        _ => {
            let (_, _, l) = rgb_to_hsl(r, g, b);
            match idx {
                0 => hue_stops(),
                1 => vec![
                    BrushStop::new(0.0, Color::from_hsl(h, 0.0, l)),
                    BrushStop::new(1.0, Color::from_hsl(h, 1.0, l)),
                ],
                _ => vec![
                    BrushStop::new(0.0, Color::BLACK),
                    BrushStop::new(0.5, Color::from_hsl(h, 1.0, 0.5)),
                    BrushStop::new(1.0, Color::WHITE),
                ],
            }
        }
    }
}

#[derive(Clone)]
struct ChanRow {
    p: ColorPicker,
    idx: usize,
}

impl View for ChanRow {
    fn body(&self) -> impl View + 'static {
        let m = self.p.model.get();
        let _ = (self.p.hue.get(), self.p.sat.get(), self.p.val.get());
        let act = self.p.active.get();
        let (label, min, max, prec) = channel_meta(m, self.idx);
        let stops = channel_stops(&self.p, self.idx);
        let sig = self.p.ch[self.idx].clone();
        let sig2 = sig.clone();
        let anchored = act == Some(self.idx);

        let pc = self.p.clone();
        let idx = self.idx;
        let track = Slider1D::new(sig, min, max)
            .gradient(stops, Orientation::Horizontal)
            .track_height(8.0)
            .on_commit(move |val| {
                pc.active.set(Some(idx));
                pc.apply_channel(idx, val);
            });

        let pc = self.p.clone();
        let idx = self.idx;
        let mut value = DragValue::new(sig2)
            .min(min)
            .max(max)
            .precision(prec)
            .width(52.0)
            .line_full(true)
            .on_commit(move |val| {
                pc.active.set(Some(idx));
                pc.apply_channel(idx, val);
            });
        if anchored {
            value = value.line_color(ACCENT_ACTIVE).line_width(2.0);
        }

        Row::new()
            .gap(8.0)
            .align_items(Align::Center)
            .child(
                Text::new(label)
                    .size(11.0)
                    .color(if anchored {
                        TEXT_PRIMARY
                    } else {
                        TEXT_SECONDARY
                    })
                    .width(11.0),
            )
            .child_flex(track, 1.0)
            .child(value)
    }
}

#[derive(Clone)]
struct PreviewBox {
    p: ColorPicker,
}

impl PreviewBox {
    const W: f32 = 64.0;
    const H: f32 = 24.0;

    const SPLIT: f32 = Self::W * 1.6 / 2.6;
}

impl View for PreviewBox {
    fn body(&self) -> impl View + 'static {
        let pd = self.p.clone();
        let pc = self.p.clone();
        let pt = self.p.clone();
        let armed = Rc::new(Cell::new(false));
        let armed_up = armed.clone();
        Container::new()
            .width(Self::W)
            .height(Self::H)
            .border(1.0, HAIR_TRACK)
            .radius(2.0)
            .clip(true)
            .child(
                Canvas::new()
                    .cursor(CursorIcon::Pointer)
                    .on_draw(move |g| {
                        let new_c = pd.cur_color();
                        let ref_c = Color::parse_hex(&pd.ref_hex.get_untracked()).unwrap_or(new_c);
                        g.fill(
                            Rect::from_xywh(0.0, 0.0, Self::SPLIT + 1.0, Self::H),
                            Fill::solid(new_c),
                        );
                        g.fill(
                            Rect::from_xywh(Self::SPLIT, 0.0, Self::W - Self::SPLIT, Self::H),
                            Fill::solid(ref_c),
                        );
                    })
                    .on_pointer(
                        move |pos: Point, ev: &PointerEvent, ctx: &mut EventCtx| match ev {
                            PointerEvent::Down {
                                button: PointerButton::Left,
                                ..
                            } => {
                                armed.set(pos.x >= Self::SPLIT);
                                if armed.get() {
                                    ctx.capture_pointer();
                                    ctx.handled();
                                }
                            }
                            PointerEvent::Up {
                                button: PointerButton::Left,
                                ..
                            } => {
                                if armed_up.take() && pos.x >= Self::SPLIT {
                                    pc.revert();
                                }
                                ctx.handled();
                            }
                            _ => {}
                        },
                    )
                    .track(move || {
                        pt.hue.get();
                        pt.sat.get();
                        pt.val.get();
                        pt.ref_hex.get();
                    }),
            )
    }
}

#[derive(Clone)]
struct SwatchBox {
    p: ColorPicker,
    hx: String,
}

impl View for SwatchBox {
    fn body(&self) -> impl View + 'static {
        let _ = (self.p.hue.get(), self.p.sat.get(), self.p.val.get());
        let selected = self.p.cur_hex() == self.hx;
        let bg = Color::parse_hex(&self.hx).unwrap_or(Color::BLACK);
        let pa = self.p.clone();
        let hx = self.hx.clone();
        let cell = Container::new()
            .width(22.0)
            .height(22.0)
            .background(bg)
            .border(
                1.0,
                if selected {
                    Color::rgb8(0x35, 0xC2, 0xF1)
                } else {
                    HAIR_TRACK
                },
            )
            .radius(2.0)
            .on_click(move || pa.apply_hex(&hx));
        let po = self.p.clone();
        let hx_a = self.hx.clone();
        let hx2 = self.hx.clone();
        let hx3 = self.hx.clone();
        let hx_d = self.hx.clone();
        ContextMenu::new(cell)
            .item("Zastosuj", move || po.apply_hex(&hx_a))
            .item("Overwrite with current", {
                let po = self.p.clone();
                move || {
                    let cur = po.cur_hex();
                    po.swatches.update(|sw| {
                        if let Some(slot) = sw.iter_mut().find(|s| *s == &hx2) {
                            *slot = cur;
                        }
                    });
                }
            })
            .item_env("Kopiuj HEX", {
                let po = self.p.clone();
                move |env: &Environment| {
                    let t = format!("#{hx3}");
                    match env.get::<SharedClipboard>() {
                        Some(c) => {
                            c.0.set_text(&t);
                            po.toast(format!("Copied {t}"));
                        }
                        None => po.toast("Clipboard unavailable"),
                    }
                }
            })
            .item("Delete", {
                let po = self.p.clone();
                move || {
                    po.swatches.update(|sw| sw.retain(|s| *s != hx_d));
                }
            })
    }
}

#[derive(Clone)]
struct SwatchesView {
    p: ColorPicker,
}

impl View for SwatchesView {
    fn body(&self) -> impl View + 'static {
        let sw = self.p.swatches.get();
        let _ = (self.p.hue.get(), self.p.sat.get(), self.p.val.get());
        let mut wrap = Wrap::new()
            .gap(6.0)
            .padding_xy(10.0, 0.0)
            .padding_bottom(10.0);
        let pa = self.p.clone();
        wrap = wrap.child(
            Button::icon("plus")
                .icon_size(12.0)
                .padding(5.0, 5.0)
                .on_click(move || pa.sw_add()),
        );
        for hx in sw {
            wrap = wrap.child(SwatchBox {
                p: self.p.clone(),
                hx,
            });
        }
        wrap
    }
}

impl ColorPicker {
    pub fn panel(&self) -> impl View + 'static {
        self.install_effects();
        let p = self.clone();
        let title = Text::with(move || {
            let dirty = p.cur_hex() != p.ref_hex.get();
            format!("BASE COLOR{}", if dirty { " *" } else { "" })
        })
        .size(11.0)
        .bold()
        .color(TEXT_TITLE);

        let pc = self.clone();
        let header = Container::new()
            .fill_width()
            .height(34.0)
            .background(BG_HEADER)
            .border_bottom(1.0, BORDER_PANEL)
            .padding_xy(10.0, 0.0)
            .child(
                Row::new()
                    .fill_size()
                    .gap(8.0)
                    .align_items(Align::Center)
                    .child_flex(title, 1.0)
                    .child(
                        Button::icon("close")
                            .icon_size(12.0)
                            .padding_xy(8.0, 7.0)
                            .on_click(move || pc.close()),
                    ),
            );

        let picker_row = Row::new()
            .gap(8.0)
            .padding_xy(10.0, 10.0)
            .child_flex(
                SvPad::new(self.sat.clone(), self.val.clone(), self.hue.clone()),
                1.0,
            )
            .child(
                Slider1D::vertical(self.hue.clone(), 0.0, 360.0)
                    .gradient(hue_stops(), Orientation::Vertical)
                    .track_height(8.0)
                    .width(14.0),
            );

        let pp = self.clone();
        let hex_field = TextField::new(self.hex_text.clone())
            .max_length(6)
            .width(88.0)
            .height(26.0)
            .font_size(12.0)
            .padding_xy(7.0, 0.0)
            .background(BG_INPUT)
            .border(1.0, BORDER_CONTROL)
            .radius(3.0)
            .on_edit_start({
                let pe = self.clone();
                move || pe.editing_hex.set(true)
            })
            .on_edit_end({
                let pe = self.clone();
                move |t: String| {
                    pe.editing_hex.set(false);
                    if parse_hex(&t).is_some() {
                        pe.apply_hex(&t);
                    } else {
                        pe.toast("Invalid hex");
                    }
                    pe.sync_channels();
                }
            })
            .on_submit({
                let pe = self.clone();
                move |t: String| {
                    pe.editing_hex.set(false);
                    if parse_hex(&t).is_some() {
                        pe.apply_hex(&t);
                    }
                    pe.sync_channels();
                }
            });

        let info_row = Row::new()
            .gap(8.0)
            .padding_x(10.0)
            .align_items(Align::Center)
            .child(PreviewBox { p: self.clone() })
            .child(Spacer::flex(1.0))
            .child(Text::new("#").size(11.0).color(TEXT_SECONDARY))
            .child(hex_field)
            .child(Button::icon("pipette").icon_size(16.0).on_click(move || {
                pp.toast("Eyedropper unavailable in this build (browser-only API)");
            }));

        let pm = self.clone();
        let model_drop = DropdownButton::new(
            self.model.clone(),
            vec!["RGB".to_string(), "HSV".to_string(), "HSL".to_string()],
        )
        .font_size(12.0)
        .padding_xy(7.0, 3.0)
        .on_select(move |i| {
            pm.model.set(i);
            pm.active.set(None);
            pm.sync_channels();
        });
        let model_row = Row::new()
            .height(26.0)
            .padding_x(10.0)
            .align_items(Align::Center)
            .child(model_drop)
            .child(Spacer::flex(1.0))
            .child(self.color_menu());

        let channels = Column::new()
            .gap(4.0)
            .padding_x(10.0)
            .child(ChanRow {
                p: self.clone(),
                idx: 0,
            })
            .child(ChanRow {
                p: self.clone(),
                idx: 1,
            })
            .child(ChanRow {
                p: self.clone(),
                idx: 2,
            });

        let section = Container::new()
            .fill_width()
            .height(24.0)
            .background(BG_SECTION)
            .border_top(1.0, BORDER_HAIRLINE)
            .border_bottom(1.0, BORDER_HAIRLINE)
            .padding_xy(10.0, 0.0)
            .child(
                Row::new()
                    .fill_size()
                    .align_items(Align::Center)
                    .child(Text::new("Swatches").size(11.0).color(TEXT_PRIMARY))
                    .child(Spacer::flex(1.0))
                    .child(self.sw_menu()),
            );

        let body = Column::new()
            .fill_width()
            .gap(10.0)
            .child(picker_row)
            .child(info_row)
            .child(model_row)
            .child(channels)
            .child(section)
            .child(SwatchesView { p: self.clone() });

        Container::new()
            .width(280.0)
            .background(BG_PANEL)
            .border(1.0, BORDER_PANEL)
            .child(Column::new().fill_width().child(header).child(body))
    }

    fn color_menu(&self) -> impl View + 'static {
        let dots = Button::new("···").font_size(11.0).padding_xy(6.0, 2.0);
        let p = self.clone();
        FloatingMenu::wrap(dots)
            .below_invoker()
            .open_on_click()
            .text_row("Cofnij  Ctrl+Z", {
                let p = p.clone();
                move || p.do_undo()
            })
            .text_row("Redo Ctrl+Y", {
                let p = p.clone();
                move || p.do_redo()
            })
            .separator()
            .text_row_env("Kopiuj HEX", {
                let p = p.clone();
                move |env: &Environment| p.copy_hex(env)
            })
            .text_row_env("Kopiuj RGB", {
                let p = p.clone();
                move |env: &Environment| p.copy_rgb(env)
            })
            .text_row_env("Kopiuj HSL", {
                let p = p.clone();
                move |env: &Environment| p.copy_hsl(env)
            })
            .separator()
            .text_row_env("Wklej  Ctrl+Shift+V", {
                let p = p.clone();
                move |env: &Environment| p.paste(env)
            })
            .text_row("Pipeta  E", {
                let p = p.clone();
                move || p.toast("Eyedropper unavailable in this build (browser-only API)")
            })
            .separator()
            .text_row("Ustaw jako odniesienie  Ctrl+S", {
                let p = p.clone();
                move || p.set_reference()
            })
    }

    fn sw_menu(&self) -> impl View + 'static {
        let dots = Button::new("···").font_size(11.0).padding_xy(6.0, 2.0);
        let p = self.clone();
        FloatingMenu::wrap(dots)
            .below_invoker()
            .open_on_click()
            .text_row("Add Current Color", {
                let p = p.clone();
                move || p.sw_add()
            })
            .text_row("Sortuj wg odcienia", {
                let p = p.clone();
                move || p.sw_sort()
            })
            .separator()
            .text_row("Restore Defaults", {
                let p = p.clone();
                move || {
                    p.swatches
                        .set(DEFAULT_SWATCHES.iter().map(|s| s.to_string()).collect());
                    p.toast("Default swatches restored");
                }
            })
            .text_row("Clear", {
                let p = p.clone();
                move || {
                    p.swatches.set(Vec::new());
                    p.toast("Swatches cleared");
                }
            })
    }

    pub fn floating(&self) -> PickerFloat {
        PickerFloat {
            picker: self.clone(),
        }
    }
}

const PICKER_HEADER_H: f32 = 34.0;
const PICKER_CLOSE_W: f32 = 40.0;

#[derive(Clone)]
pub struct PickerFloat {
    picker: ColorPicker,
}

impl PickerFloat {
    fn picker_style(&self) -> LayoutStyle {
        let (x, y) = self.picker.pos.get_untracked();
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            position: Position::Absolute(Inset::xy(x, y)),
            ..LayoutStyle::default()
        }
    }
}

impl View for PickerFloat {
    fn body(&self) -> impl View + 'static {
        let _ = self.picker.pos.get();
        let _ = self.picker.closed.get();
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        if self.picker.closed.get() {
            return Some(Vec::new());
        }
        Some(vec![AnyView::new(self.picker.panel())])
    }

    fn layout_style(&self) -> LayoutStyle {
        self.picker_style()
    }

    fn layout_style_ctx(
        &self,
        _rt: &sequoia_view::Runtime,
        _el: sequoia_view::ElementId,
    ) -> LayoutStyle {
        self.picker_style()
    }

    fn accessibility(&self) -> Option<sequoia_view::Accessible> {
        Some(sequoia_view::Accessible::new(
            sequoia_view::AccessibleRole::Group,
            "Color picker",
        ))
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        if self.picker.drag.get().is_some() {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Default
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if !self.picker.is_open() {
            return;
        }
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) => {
                let head = Rect::from_xywh(
                    ctx.rect.left(),
                    ctx.rect.top(),
                    (ctx.rect.size.width - PICKER_CLOSE_W).max(0.0),
                    PICKER_HEADER_H.min(ctx.rect.size.height),
                );
                if head.contains(*pos) {
                    let (cx, cy) = self.picker.pos.get_untracked();
                    self.picker.drag.set(Some(PickerDrag {
                        origin: *pos,
                        start: (cx, cy),
                    }));
                    ctx.capture_pointer();
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) => {
                if let Some(st) = self.picker.drag.get() {
                    let nx = st.start.0 + (pos.x - st.origin.x);
                    let ny = st.start.1 + (pos.y - st.origin.y);
                    if nx.is_finite() && ny.is_finite() {
                        let win = ctx.window_size();
                        let max_x = (win.width - 120.0).max(0.0);
                        let max_y = (win.height - PICKER_HEADER_H).max(0.0);
                        self.picker
                            .pos
                            .set((nx.clamp(0.0, max_x), ny.clamp(0.0, max_y)));
                    }
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                ..
            }) => {
                if self.picker.drag.take().is_some() {
                    ctx.handled();
                }
            }
            Event::Pointer(PointerEvent::Cancel) => {
                self.picker.drag.set(None);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parser_is_strict_and_accepts_short_forms() {
        assert_eq!(parse_hex("#abc"), Some("AABBCC".to_string()));
        assert_eq!(parse_hex("A1b2C3"), Some("A1B2C3".to_string()));
        assert_eq!(parse_hex("0x112233"), Some("112233".to_string()));
        assert_eq!(parse_hex("11223344"), Some("112233".to_string()));
        assert_eq!(parse_hex("abcd"), Some("AABBCC".to_string()));
        assert_eq!(parse_hex("rgb(255, 0, 0)"), None);
        assert_eq!(parse_hex("GGGGGG"), None);
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex(""), None);
    }

    #[test]
    fn color_text_parser_handles_css_rgb_and_hsl() {
        assert_eq!(parse_color_text("#f00"), Some((255, 0, 0)));
        assert_eq!(parse_color_text("rgb(255, 0, 0)"), Some((255, 0, 0)));
        assert_eq!(parse_color_text("rgb(100%, 0%, 50%)"), Some((255, 0, 128)));
        assert_eq!(
            parse_color_text("rgba(10, 20, 30, 0.5)"),
            Some((10, 20, 30))
        );
        assert_eq!(parse_color_text("hsl(120, 100%, 50%)"), Some((0, 255, 0)));
        assert_eq!(parse_color_text("nope"), None);
    }

    #[test]
    fn set_hex_updates_picker_and_channel_text() {
        let picker = ColorPicker::new();
        picker.install_effects();
        picker.install_effects();
        picker.set_hex("#FF0000");
        assert_eq!(picker.cur_hex(), "FF0000");
        assert_eq!(picker.hex_text.get_untracked(), "FF0000");
        picker.set_hex("#00ff00");
        assert_eq!(picker.cur_hex(), "00FF00");
    }

    #[test]
    fn close_and_open_toggle_floating_state() {
        let picker = ColorPicker::new();
        assert!(picker.is_open());
        picker.close();
        assert!(!picker.is_open());
        picker.open();
        assert!(picker.is_open());
    }

    #[test]
    fn floating_uses_the_position_signal() {
        let picker = ColorPicker::new();
        picker.pos_signal().set((123.0, 45.0));
        let style = picker.floating().picker_style();
        match style.position {
            Position::Absolute(inset) => {
                assert_eq!(inset.left, Some(123.0));
                assert_eq!(inset.top, Some(45.0));
            }
            _ => panic!("expected absolute position"),
        }
    }
}
