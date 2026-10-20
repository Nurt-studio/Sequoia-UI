#[allow(unused_imports)]
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::types::*;
use crate::utils::*;
use sequoia_core::{Color, Edges, Length, Rect};
use sequoia_view::{KeyEvent, LayoutStyle, PointerEvent, SurfaceInputBinding, Unit, View};

pub struct WgpuFrame<'a> {
    pub device: &'a wgpu::Device,

    pub queue: &'a wgpu::Queue,

    pub encoder: &'a mut wgpu::CommandEncoder,

    pub target: &'a wgpu::TextureView,

    pub depth: Option<&'a wgpu::TextureView>,

    pub format: wgpu::TextureFormat,

    pub size: (u32, u32),

    pub scale: f32,

    pub region: Region,

    pub adapter: wgpu::AdapterInfo,

    pub backend: GpuBackend,

    pub index: u64,

    pub delta: std::time::Duration,
    pub(crate) redraw: &'a Cell<bool>,
}

impl WgpuFrame<'_> {
    pub fn request_frame(&self) {
        self.redraw.set(true);
    }
}

pub trait WgpuRenderer {
    fn render(&mut self, frame: &mut WgpuFrame);

    fn is_dirty(&self) -> bool {
        true
    }

    fn mark_clean(&mut self) {}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingFrames {
    pub(crate) pending: u32,
}

impl PendingFrames {
    pub fn new() -> Self {
        Self { pending: 0 }
    }

    pub fn request(&mut self, n: u32) {
        self.pending = self.pending.max(n);
    }

    pub fn request_if(&mut self, changed: bool, n: u32) {
        if changed {
            self.request(n);
        }
    }

    pub fn tick(&mut self, request: impl FnOnce()) -> u32 {
        if self.pending > 0 {
            self.pending -= 1;
            request();
        }
        self.pending
    }

    pub fn pending(&self) -> u32 {
        self.pending
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Revision {
    pub(crate) last: u64,
    pub(crate) seen: bool,
}

impl Revision {
    pub fn new() -> Self {
        Self {
            last: 0,
            seen: false,
        }
    }

    pub fn changed(&mut self, hash: u64) -> bool {
        let changed = !self.seen || self.last != hash;
        self.last = hash;
        self.seen = true;
        changed
    }
}

#[allow(clippy::type_complexity)]
pub(crate) type SurfaceDrawFn = dyn for<'a> Fn(&mut WgpuFrame<'a>);

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SurfaceFrame {
    pub(crate) background: Option<[f32; 4]>,

    pub(crate) border: Option<([f32; 4], f32)>,

    pub(crate) radius: f32,

    pub(crate) inset: [f32; 4],
}

pub(crate) struct SurfaceBinding {
    pub(crate) draw: Box<SurfaceDrawFn>,
    pub(crate) frame: SurfaceFrame,

    pub(crate) key: usize,

    pub(crate) cached: bool,

    pub(crate) samples: u32,
    pub(crate) check_dirty: Box<dyn Fn() -> bool>,
    pub(crate) mark_clean: RefCell<Box<dyn FnMut()>>,
}

pub(crate) fn sanitize_samples(samples: u32) -> u32 {
    match samples {
        0 | 1 => 1,
        2 => 2,
        _ => 4,
    }
}

pub(crate) struct CachedJob {
    pub(crate) binding: Rc<SurfaceBinding>,

    pub(crate) size: (u32, u32),

    pub(crate) full: Rect,

    pub(crate) screen: Rect,
}

pub struct WgpuSurface<R> {
    pub(crate) renderer: Rc<RefCell<R>>,
    pub(crate) style: LayoutStyle,
    pub(crate) background: Color,
    pub(crate) border_color: Color,
    pub(crate) border_width: f32,
    pub(crate) radius: f32,
    pub(crate) padding: Edges,
    pub(crate) cached: bool,
    pub(crate) samples: u32,
    pub(crate) on_pointer: Option<Rc<dyn Fn(&PointerEvent, Rect)>>,
    pub(crate) on_key: Option<Rc<dyn Fn(&KeyEvent)>>,
    pub(crate) click_through: bool,
    pub(crate) focusable_flag: bool,
}

impl<R> Clone for WgpuSurface<R> {
    fn clone(&self) -> Self {
        Self {
            renderer: Rc::clone(&self.renderer),
            style: self.style,
            background: self.background,
            border_color: self.border_color,
            border_width: self.border_width,
            radius: self.radius,
            padding: self.padding,
            cached: self.cached,
            samples: self.samples,
            on_pointer: self.on_pointer.clone(),
            on_key: self.on_key.clone(),
            click_through: self.click_through,
            focusable_flag: self.focusable_flag,
        }
    }
}

impl<R: WgpuRenderer + 'static> WgpuSurface<R> {
    pub fn new(renderer: R) -> Self {
        Self {
            renderer: Rc::new(RefCell::new(renderer)),
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
            background: Color::TRANSPARENT,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            radius: 0.0,
            padding: Edges::ZERO,
            cached: false,
            samples: 1,
            on_pointer: None,
            on_key: None,
            click_through: false,
            focusable_flag: false,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width.max(0.0);
        self.border_color = color;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    pub fn padding(mut self, padding: impl Into<Edges>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn pad(self, padding: impl Into<Edges>) -> Self {
        self.padding(padding)
    }

    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.padding = Edges::xy(x, y);
        self
    }

    pub fn padding_x(mut self, x: f32) -> Self {
        self.padding.left = x;
        self.padding.right = x;
        self
    }

    pub fn padding_y(mut self, y: f32) -> Self {
        self.padding.top = y;
        self.padding.bottom = y;
        self
    }

    pub fn padding_left(mut self, v: f32) -> Self {
        self.padding.left = v;
        self
    }

    pub fn padding_right(mut self, v: f32) -> Self {
        self.padding.right = v;
        self
    }

    pub fn padding_top(mut self, v: f32) -> Self {
        self.padding.top = v;
        self
    }

    pub fn padding_bottom(mut self, v: f32) -> Self {
        self.padding.bottom = v;
        self
    }

    pub fn cached(mut self, cached: bool) -> Self {
        self.cached = cached;
        self
    }

    pub fn samples(mut self, samples: u32) -> Self {
        self.samples = sanitize_samples(samples);
        self
    }

    pub fn on_pointer(mut self, f: impl Fn(&PointerEvent, Rect) + 'static) -> Self {
        self.on_pointer = Some(Rc::new(f));
        self
    }

    pub fn on_key(mut self, f: impl Fn(&KeyEvent) + 'static) -> Self {
        self.on_key = Some(Rc::new(f));
        self
    }

    pub fn click_through(mut self, click_through: bool) -> Self {
        self.click_through = click_through;
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable_flag = focusable;
        self
    }

    pub fn renderer(&self) -> Rc<RefCell<R>> {
        Rc::clone(&self.renderer)
    }
}

impl<R: WgpuRenderer + 'static> View for WgpuSurface<R> {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
    }

    fn captures_pointer(&self) -> bool {
        !self.click_through
    }

    fn focusable(&self) -> bool {
        self.focusable_flag
    }

    fn surface_input(&self) -> Option<SurfaceInputBinding> {
        if self.on_pointer.is_none() && self.on_key.is_none() {
            return None;
        }
        let pointer = self.on_pointer.clone();
        let key = self.on_key.clone();
        Some(SurfaceInputBinding {
            on_pointer: Box::new(move |ev, area| {
                if let Some(f) = &pointer {
                    f(ev, area);
                }
            }),
            on_key: Box::new(move |ev| {
                if let Some(f) = &key {
                    f(ev);
                }
            }),
        })
    }

    fn surface_target(&self) -> Option<Rc<dyn Any>> {
        fn rgba(c: Color) -> [f32; 4] {
            linear_rgba(c)
        }
        let has_border = self.border_width > 0.0 && self.border_color.a > 0.0;
        let bw = if has_border { self.border_width } else { 0.0 };
        let rc = Rc::clone(&self.renderer);
        let key = Rc::as_ptr(&rc) as usize;
        let rc_dirty = Rc::clone(&rc);
        let rc_clean = Rc::clone(&rc);
        let binding: Rc<dyn Any> = Rc::new(SurfaceBinding {
            draw: Box::new(move |frame| {
                rc.borrow_mut().render(frame);
            }),
            frame: SurfaceFrame {
                background: (self.background.a > 0.0).then_some(rgba(self.background)),
                border: has_border.then_some((rgba(self.border_color), self.border_width)),
                radius: self.radius,
                inset: [
                    self.padding.left + bw,
                    self.padding.top + bw,
                    self.padding.right + bw,
                    self.padding.bottom + bw,
                ],
            },
            key,
            cached: self.cached,
            samples: self.samples,
            check_dirty: Box::new(move || rc_dirty.borrow().is_dirty()),
            mark_clean: RefCell::new(Box::new(move || rc_clean.borrow_mut().mark_clean())),
        });
        Some(binding)
    }
}
