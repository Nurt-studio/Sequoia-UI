#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use sequoia_assets::AssetRegistry;
use sequoia_core::{Color, Point, Rect, RenderList, Size};
use sequoia_elements::{SharedClipboard, SharedDataClipboard};
use sequoia_input::{CursorIcon, ImeEvent, Key, Modifiers, PointerButton};
use sequoia_reactive::Environment;
use sequoia_render::{GpuOptions, GpuRenderer};
use sequoia_style::Theme;
use sequoia_text::TextSystem;
use sequoia_view::{AnyView, Runtime, View};

mod cli;
mod clipboard;
mod dock_layout;
mod executor;
mod native_dialog;
mod recent;
mod settings;
mod single_instance;
mod window;

pub use cli::{CliArgs, help_text};
pub use clipboard::OsClipboard;
pub use dock_layout::DockLayoutStore;
pub use executor::{Executor, JobId, JobWake};
pub use native_dialog::{FileFilter, MessageKind, NativeFileDialog, native_message};
pub use recent::RecentFiles;
pub use sequoia_async::{AsyncState, Emitter, Resource, ScopedTask, TaskHandle, TaskId};
pub use sequoia_core::WindowId;
pub use sequoia_hotkey::{Error as HotkeyError, GlobalHotkeys, HotkeyId};
pub use settings::{Settings, SettingsError};
pub use single_instance::SingleInstance;
pub use window::{WindowSettings, WindowSpec, Windows};

pub use sequoia_assets::AssetRegistry as Assets;
use sequoia_render::FrameSkip;
pub use sequoia_render::{
    GpuBackend, PowerPreference as GpuPowerPreference, PresentMode as GpuPresentMode, Region,
    WgpuFrame, WgpuRenderer, WgpuSurface,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{
    CursorGrabMode, Fullscreen, Icon, Window, WindowId as WinitWindowId, WindowLevel,
};

#[derive(Debug)]
#[non_exhaustive]
pub enum AppError {
    EventLoop(String),

    Window(String),

    Gpu(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventLoop(e) => write!(f, "event loop error: {e}"),
            Self::Window(e) => write!(f, "window creation failed: {e}"),
            Self::Gpu(e) => write!(f, "GPU init failed: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RedrawMode {
    #[default]
    OnDemand,

    Continuous,
}

#[derive(Clone)]
pub struct Accelerator {
    pub key: Key,

    pub modifiers: Modifiers,
    action: Rc<dyn Fn(&AcceleratorCtx)>,
    when: Option<Rc<dyn Fn() -> bool>>,
}

#[derive(Clone)]
pub struct AcceleratorCtx {
    pub window: WindowId,

    pub size: Size,

    pub scale: f32,

    pub environment: Environment,
}

impl Accelerator {
    pub fn new(key: Key, modifiers: Modifiers, action: impl Fn(&AcceleratorCtx) + 'static) -> Self {
        Self {
            key,
            modifiers,
            action: Rc::new(action),
            when: None,
        }
    }

    pub fn simple(key: Key, modifiers: Modifiers, action: impl Fn() + 'static) -> Self {
        Self::new(key, modifiers, move |_| action())
    }

    pub fn with_when(mut self, when: Rc<dyn Fn() -> bool>) -> Self {
        self.when = Some(when);
        self
    }

    pub fn should_fire(&self) -> bool {
        self.when.as_ref().is_none_or(|w| w())
    }

    pub fn fire(&self, ctx: &AcceleratorCtx) {
        (self.action)(ctx);
    }
}

pub fn not_editing(editing: sequoia_reactive::Signal<bool>) -> Rc<dyn Fn() -> bool> {
    Rc::new(move || !editing.get_untracked())
}

pub struct App {
    windows: Vec<WindowSpec>,
    theme: Theme,
    backend: GpuBackend,
    redraw_mode: RedrawMode,
    assets: AssetRegistry,
    worker_threads: usize,
    accelerators: Vec<Accelerator>,
    global_hotkeys: Vec<(Key, Modifiers, Rc<dyn Fn()>)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            windows: vec![WindowSpec::untitled()],
            theme: Theme::dark(),
            backend: GpuBackend::Auto,
            redraw_mode: RedrawMode::OnDemand,
            assets: AssetRegistry::with_builtin_icons(),
            worker_threads: executor::default_threads(),
            accelerators: Vec::new(),
            global_hotkeys: Vec::new(),
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    fn primary_mut(&mut self) -> &mut WindowSpec {
        &mut self.windows[0]
    }

    pub fn window(mut self, spec: WindowSpec) -> Self {
        self.windows.push(spec);
        self
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.primary_mut().title = title.into();
        self
    }

    pub fn size(mut self, width: f64, height: f64) -> Self {
        let p = self.primary_mut();
        p.width = width;
        p.height = height;
        self
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn backend(mut self, backend: GpuBackend) -> Self {
        self.backend = backend;
        self
    }

    pub fn root(mut self, root: impl View + 'static) -> Self {
        self.primary_mut().root = Some(AnyView::new(root));
        self
    }

    pub fn redraw_mode(mut self, mode: RedrawMode) -> Self {
        self.redraw_mode = mode;
        self
    }

    pub fn assets(mut self, assets: AssetRegistry) -> Self {
        self.assets = assets;
        self
    }

    pub fn icon(self, name: impl Into<String>, svg: &str) -> Self {
        let name = name.into();
        if let Err(e) = self.assets.register_icon(&name, svg) {
            log::error!("sequoia-platform: invalid SVG for icon `{name}`: {e} — icon skipped");
        }
        self
    }

    pub fn try_icon(
        self,
        name: impl Into<String>,
        svg: &str,
    ) -> Result<Self, sequoia_assets::SvgError> {
        let name = name.into();
        self.assets.register_icon(&name, svg)?;
        Ok(self)
    }

    pub fn font(self, name: impl Into<String>, data: Vec<u8>) -> Self {
        self.assets.register_font(name, data);
        self
    }

    pub fn default_font(self, name: impl Into<String>) -> Self {
        self.assets.set_default_font(name);
        self
    }

    pub fn on_close_request(mut self, f: impl Fn() -> bool + 'static) -> Self {
        self.primary_mut().on_close_request = Some(std::rc::Rc::new(f));
        self
    }

    pub fn accelerator(
        mut self,
        key: Key,
        modifiers: Modifiers,
        action: impl Fn() + 'static,
    ) -> Self {
        self.accelerators
            .push(Accelerator::simple(key, modifiers, action));
        self
    }

    pub fn accelerator_ctx(
        mut self,
        key: Key,
        modifiers: Modifiers,
        action: impl Fn(&AcceleratorCtx) + 'static,
    ) -> Self {
        self.accelerators
            .push(Accelerator::new(key, modifiers, action));
        self
    }

    pub fn accelerator_when(
        mut self,
        key: Key,
        modifiers: Modifiers,
        when: Rc<dyn Fn() -> bool>,
        action: impl Fn() + 'static,
    ) -> Self {
        self.accelerators
            .push(Accelerator::simple(key, modifiers, action).with_when(when));
        self
    }

    pub fn accelerator_ctx_when(
        mut self,
        key: Key,
        modifiers: Modifiers,
        when: Rc<dyn Fn() -> bool>,
        action: impl Fn(&AcceleratorCtx) + 'static,
    ) -> Self {
        self.accelerators
            .push(Accelerator::new(key, modifiers, action).with_when(when));
        self
    }

    pub fn global_hotkey(
        mut self,
        key: Key,
        modifiers: Modifiers,
        action: impl Fn() + 'static,
    ) -> Self {
        self.global_hotkeys.push((key, modifiers, Rc::new(action)));
        self
    }

    pub fn min_size(mut self, width: f64, height: f64) -> Self {
        self.primary_mut().settings.min_size = Some((width, height));
        self
    }

    pub fn max_size(mut self, width: f64, height: f64) -> Self {
        self.primary_mut().settings.max_size = Some((width, height));
        self
    }

    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.primary_mut().settings.position = Some((x, y));
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.primary_mut().settings.resizable = resizable;
        self
    }

    pub fn maximized(mut self, maximized: bool) -> Self {
        self.primary_mut().settings.maximized = maximized;
        self
    }

    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.primary_mut().settings.fullscreen = fullscreen;
        self
    }

    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.primary_mut().settings.always_on_top = always_on_top;
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.primary_mut().settings.decorations = decorations;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.primary_mut().settings.transparent = transparent;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.primary_mut().settings.visible = visible;
        self
    }

    pub fn window_icon(mut self, name: impl Into<String>) -> Self {
        self.primary_mut().settings.icon = Some(name.into());
        self
    }

    pub fn window_icon_tint(mut self, tint: Color) -> Self {
        self.primary_mut().settings.icon_tint = Some(tint);
        self
    }

    pub fn present_mode(mut self, mode: GpuPresentMode) -> Self {
        self.primary_mut().settings.present_mode = mode;
        self
    }

    pub fn power_preference(mut self, power: GpuPowerPreference) -> Self {
        self.primary_mut().settings.power_preference = power;
        self
    }

    pub fn worker_threads(mut self, threads: usize) -> Self {
        self.worker_threads = threads;
        self
    }

    pub fn fps_limit(mut self, fps: u32) -> Self {
        self.primary_mut().settings.fps_limit = Some(fps.max(1));
        self
    }

    pub fn show_fps(mut self, show: bool) -> Self {
        self.primary_mut().settings.show_fps = show;
        self
    }

    pub fn scale_factor(mut self, scale: f64) -> Self {
        self.primary_mut().settings.scale_override = Some(scale);
        self
    }

    pub fn cursor_visible(mut self, visible: bool) -> Self {
        self.primary_mut().settings.cursor_visible = visible;
        self
    }

    pub fn cursor_grab(mut self, grab: bool) -> Self {
        self.primary_mut().settings.cursor_grab = grab;
        self
    }

    pub fn run(self) -> Result<(), AppError> {
        let event_loop = EventLoop::<JobWake>::with_user_event()
            .build()
            .map_err(|e| AppError::EventLoop(e.to_string()))?;

        let windows = Windows::new();
        let mut initial = Vec::with_capacity(self.windows.len());
        for spec in self.windows {
            initial.push(windows.push_initial(spec));
        }

        let executor = Executor::new(Some(event_loop.create_proxy()), self.worker_threads);

        let env = Environment::new();
        env.provide(self.theme);
        env.provide(self.assets.clone());
        if let Some(clipboard) = clipboard::OsClipboard::new() {
            env.provide(SharedClipboard(std::rc::Rc::new(clipboard)));
        }

        env.provide(SharedDataClipboard(std::rc::Rc::new(
            sequoia_elements::DataClipboard::new(),
        )));
        env.provide(windows.clone());
        env.provide(executor.clone());

        let mut global_actions = HashMap::new();
        let global_hotkeys = if self.global_hotkeys.is_empty() {
            None
        } else {
            match GlobalHotkeys::new() {
                Ok(mut mgr) => {
                    for (key, mods, action) in self.global_hotkeys {
                        match mgr.register(key, mods) {
                            Ok(id) => {
                                global_actions.insert(id, action);
                            }
                            Err(e) => log::warn!("sequoia-platform: global hash rejected ({e})"),
                        }
                    }
                    Some(mgr)
                }
                Err(e) => {
                    log::warn!("sequoia-platform: no global shortcuts ({e})");
                    None
                }
            }
        };
        let mut handler = PlatformApp {
            theme: self.theme,
            backend: self.backend,
            redraw_mode: self.redraw_mode,
            assets: self.assets,
            accelerators: self.accelerators,
            global_hotkeys,
            global_actions,
            env,
            windows,
            executor,
            initial,
            live: HashMap::new(),
            id_map: HashMap::new(),
            error: None,
        };
        event_loop
            .run_app(&mut handler)
            .map_err(|e| AppError::EventLoop(e.to_string()))?;
        if let Some(e) = handler.take_error() {
            return Err(e);
        }
        Ok(())
    }
}

const WINDOW_ICON_PX: u32 = 64;

fn tint_rgba(alpha: &[u8], tint: Color) -> Vec<u8> {
    let ch = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let (r, g, b, a) = (ch(tint.r), ch(tint.g), ch(tint.b), tint.a.clamp(0.0, 1.0));
    let mut out = Vec::with_capacity(alpha.len() * 4);
    for cov in alpha {
        out.push(r);
        out.push(g);
        out.push(b);
        out.push((f32::from(*cov) * a).round() as u8);
    }
    out
}

fn window_icon(assets: &AssetRegistry, name: &str, tint: Color) -> Option<Icon> {
    let sprite = assets.rasterize_icon(name, WINDOW_ICON_PX).ok()?;
    if sprite.is_blank() {
        log::warn!("sequoia-platform: window icon `{name}` is blank (all-zero coverage)");
        return None;
    }
    let rgba = tint_rgba(&sprite.alpha, tint);
    Icon::from_rgba(rgba, sprite.width, sprite.height).ok()
}

fn grab_cursor(window: &Window) -> Result<(), winit::error::ExternalError> {
    match window.set_cursor_grab(CursorGrabMode::Confined) {
        Ok(()) => {
            log::debug!("sequoia-platform: cursor grab Confined");
            Ok(())
        }
        Err(confined) => window
            .set_cursor_grab(CursorGrabMode::Locked)
            .map(|_| {
                log::debug!(
                    "sequoia-platform: Confined unavailable (Wayland/macOS?) - using Locked"
                );
            })
            .map_err(|_| confined),
    }
}

fn sanitize_scale(scale: f64) -> f32 {
    let s = scale as f32;
    if s.is_finite() && s > 0.0 { s } else { 1.0 }
}

struct WindowState {
    id: WindowId,
    window: Arc<Window>,
    rt: Runtime,
    gpu: GpuRenderer,
    text: TextSystem,
    cursor: Point,
    mods: Modifiers,

    scale: f32,

    last_cursor: Option<winit::window::CursorIcon>,

    settings: WindowSettings,

    next_frame: Option<Instant>,

    fps_base: String,
    fps_frames: std::collections::VecDeque<(Instant, f64)>,
    fps_last_push: Option<Instant>,

    continuous_paced: bool,

    list: RenderList,
}

impl WindowState {
    fn draw_frame(&mut self, wid: WinitWindowId) {
        let gap_ms = frame_gap_ms(wid);
        let mut t = Instant::now();
        self.rt.tick();
        let tick_ms = t.elapsed().as_secs_f64() * 1000.0;
        t = Instant::now();
        let size = self.window.inner_size();
        let s = self.scale;
        self.rt.set_scale(s);
        let bounds = Rect::from_xywh(0.0, 0.0, size.width as f32 / s, size.height as f32 / s);
        self.rt.layout(bounds, &mut self.text);
        let layout_ms = t.elapsed().as_secs_f64() * 1000.0;
        t = Instant::now();
        self.list.clear();
        self.rt.paint(&mut self.list, &mut self.text);
        let surfaces = self.rt.take_surface_draws();
        let paint_ms = t.elapsed().as_secs_f64() * 1000.0;
        t = Instant::now();
        if let Err(e) = self
            .gpu
            .render(&self.list, self.text.atlas_mut(), s, &surfaces)
        {
            log::warn!("sequoia-platform: render error: {e}");
        }
        let render_ms = t.elapsed().as_secs_f64() * 1000.0;
        note_frame(gap_ms, tick_ms, layout_ms, paint_ms, render_ms);
        if self.settings.show_fps {
            self.push_fps_title(tick_ms + layout_ms + paint_ms + render_ms);
        }

        match self.gpu.take_frame_skip() {
            Some(FrameSkip::Timeout) => self.window.request_redraw(),
            Some(FrameSkip::Occluded) | None => {}

            Some(_) => {}
        }

        if self.gpu.take_redraw_request() {
            self.window.request_redraw();
        }

        sync_cursor(&self.window, &mut self.last_cursor, &self.rt);
        sync_ime_cursor(&self.window, self.scale, &self.rt);
    }
}

const FPS_WINDOW: Duration = Duration::from_millis(1000);

const FPS_WINDOW_CAP: usize = 600;

const FPS_PUSH_EVERY: Duration = Duration::from_millis(500);

fn fps_title_detailed(base: &str, fps: u32, p50_ms: f64, p95_ms: f64) -> String {
    format!("{base} — {fps} FPS · p50 {p50_ms:.0}ms p95 {p95_ms:.0}ms")
}

fn fps_title_idle(base: &str) -> String {
    format!("{base} — IDLE")
}

fn fps_title_idle_cost(base: &str, p50_ms: f64) -> String {
    if p50_ms.is_finite() && p50_ms > 0.05 {
        let theo = (1000.0 / p50_ms).round().max(1.0) as u32;
        format!("{base} — IDLE · p50 {p50_ms:.0}ms (~{theo} FPS)")
    } else {
        fps_title_idle(base)
    }
}

const FPS_IDLE_MAX_COUNT: usize = 10;

fn fps_sample_is_idle(count: usize, p95_ms: f64) -> bool {
    count < FPS_IDLE_MAX_COUNT && p95_ms < SLOW_FRAME_MS
}

fn fps_title_for_sample(base: &str, count: u32, p50_ms: f64, p95_ms: f64) -> String {
    if count == 0 {
        fps_title_idle(base)
    } else if fps_sample_is_idle(count as usize, p95_ms) {
        fps_title_idle_cost(base, p50_ms)
    } else {
        fps_title_detailed(base, count, p50_ms, p95_ms)
    }
}

fn percentile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let q = q.clamp(0.0, 1.0);
    let idx = ((sorted.len() as f64 - 1.0) * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

impl WindowState {
    fn fps_sample_stats(&self) -> Option<(u32, f64, f64)> {
        if self.fps_frames.is_empty() {
            return None;
        }
        let mut times: Vec<f64> = self.fps_frames.iter().map(|(_, ms)| *ms).collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some((
            self.fps_frames.len() as u32,
            percentile_sorted(&times, 0.5),
            percentile_sorted(&times, 0.95),
        ))
    }

    fn fps_push_title(&self, title: String) {
        self.window.set_title(title.as_str());
    }

    fn push_fps_title(&mut self, total_ms: f64) {
        let now = Instant::now();
        if total_ms.is_finite() && total_ms >= 0.0 {
            self.fps_frames.push_back((now, total_ms));
            while self.fps_frames.len() > FPS_WINDOW_CAP {
                self.fps_frames.pop_front();
            }
        }
        self.evict_old_fps_frames(now);
        let due = self
            .fps_last_push
            .map(|t| now.duration_since(t) >= FPS_PUSH_EVERY)
            .unwrap_or(true);
        if !due {
            return;
        }
        self.fps_last_push = Some(now);
        let title = match self.fps_sample_stats() {
            Some((count, p50, p95)) if self.continuous_paced && count > 0 => {
                fps_title_detailed(&self.fps_base, count, p50, p95)
            }
            Some((count, p50, p95)) => fps_title_for_sample(&self.fps_base, count, p50, p95),
            None => fps_title_idle(&self.fps_base),
        };
        self.fps_push_title(title);
    }

    fn evict_old_fps_frames(&mut self, now: Instant) {
        while self
            .fps_frames
            .front()
            .is_some_and(|(t, _)| now.duration_since(*t) > FPS_WINDOW)
        {
            self.fps_frames.pop_front();
        }
    }

    fn refresh_fps_title(&mut self) -> bool {
        if !self.settings.show_fps {
            return false;
        }
        if self.fps_frames.is_empty() {
            return false;
        }
        let now = Instant::now();
        self.evict_old_fps_frames(now);
        if self.fps_frames.is_empty() {
            self.fps_last_push = Some(now);
            let title = fps_title_idle(&self.fps_base);
            self.fps_push_title(title);
            return true;
        }
        let due = self
            .fps_last_push
            .map(|t| now.duration_since(t) >= FPS_PUSH_EVERY)
            .unwrap_or(true);
        if !due {
            return false;
        }

        let Some((count, p50, p95)) = self.fps_sample_stats() else {
            return false;
        };
        if self.continuous_paced || !fps_sample_is_idle(count as usize, p95) {
            return false;
        }
        self.fps_last_push = Some(now);
        self.fps_push_title(fps_title_idle_cost(&self.fps_base, p50));
        true
    }
}

fn map_cursor(icon: CursorIcon) -> winit::window::CursorIcon {
    use winit::window::CursorIcon as W;
    match icon {
        CursorIcon::Default => W::Default,
        CursorIcon::Pointer => W::Pointer,
        CursorIcon::Grab => W::Grab,
        CursorIcon::Grabbing => W::Grabbing,
        CursorIcon::Text => W::Text,
        CursorIcon::ColResize => W::ColResize,
        CursorIcon::RowResize => W::RowResize,
        CursorIcon::Crosshair => W::Crosshair,

        _ => W::Default,
    }
}

fn sync_cursor(window: &Window, last_cursor: &mut Option<winit::window::CursorIcon>, rt: &Runtime) {
    let icon = map_cursor(rt.cursor_icon());
    if *last_cursor != Some(icon) {
        window.set_cursor(icon);
        *last_cursor = Some(icon);
    }
}

const SLOW_FRAME_MS: f64 = 50.0;

const SLOW_FRAME_QUIET: Duration = Duration::from_secs(2);

static FRAME_ENDS: OnceLock<Mutex<HashMap<WinitWindowId, Instant>>> = OnceLock::new();

static LAST_SLOW_LOG: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

fn forget_frame_stats(wid: WinitWindowId) {
    let Some(map) = FRAME_ENDS.get() else {
        return;
    };
    let mut ends = map.lock().unwrap_or_else(|e| e.into_inner());
    ends.remove(&wid);
}

fn frame_gap_ms(wid: WinitWindowId) -> f64 {
    let mut ends = FRAME_ENDS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let gap = ends
        .get(&wid)
        .map(|t| t.elapsed().as_secs_f64() * 1000.0)
        .unwrap_or(0.0);
    ends.insert(wid, Instant::now());
    gap
}

fn note_frame(gap_ms: f64, tick_ms: f64, layout_ms: f64, paint_ms: f64, render_ms: f64) {
    let total_ms = tick_ms + layout_ms + paint_ms + render_ms;
    if total_ms < SLOW_FRAME_MS {
        return;
    }
    let now = Instant::now();
    {
        let mut last = LAST_SLOW_LOG
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if last
            .map(|t| now.duration_since(t) < SLOW_FRAME_QUIET)
            .unwrap_or(false)
        {
            return;
        }
        *last = Some(now);
    }
    log::warn!(
        "sequoia-platform: slow frame {total_ms:.1}ms (gap {gap_ms:.0}ms; tick {tick_ms:.1}ms, layout {layout_ms:.1}ms, paint {paint_ms:.1}ms, render+present {render_ms:.1}ms)"
    );
}

struct PlatformApp {
    theme: Theme,
    backend: GpuBackend,
    redraw_mode: RedrawMode,
    assets: AssetRegistry,

    accelerators: Vec<Accelerator>,

    global_hotkeys: Option<GlobalHotkeys>,

    global_actions: HashMap<HotkeyId, Rc<dyn Fn()>>,

    env: Environment,

    windows: Windows,

    executor: Executor,

    initial: Vec<WindowId>,

    live: HashMap<WinitWindowId, WindowState>,

    id_map: HashMap<WindowId, WinitWindowId>,
    error: Option<AppError>,
}

impl PlatformApp {
    fn take_error(&mut self) -> Option<AppError> {
        self.error.take()
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: AppError) {
        log::error!("sequoia-platform: {error}");
        self.error = Some(error);
        event_loop.exit();
    }

    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        spec: &WindowSpec,
    ) -> Result<(), AppError> {
        let s = &spec.settings;

        let show_after_first_frame = s.visible;
        let mut attrs = Window::default_attributes()
            .with_title(spec.title.clone())
            .with_resizable(s.resizable)
            .with_maximized(s.maximized)
            .with_decorations(s.decorations)
            .with_transparent(s.transparent)
            .with_visible(false);
        if let Some(scale) = s.scale_override.map(sanitize_scale) {
            let scale = f64::from(scale);
            attrs = attrs.with_inner_size(PhysicalSize::new(
                (spec.width * scale).round(),
                (spec.height * scale).round(),
            ));
            if let Some((w, h)) = s.min_size {
                attrs = attrs.with_min_inner_size(PhysicalSize::new(
                    (w * scale).round(),
                    (h * scale).round(),
                ));
            }
            if let Some((w, h)) = s.max_size {
                attrs = attrs.with_max_inner_size(PhysicalSize::new(
                    (w * scale).round(),
                    (h * scale).round(),
                ));
            }
            if let Some((x, y)) = s.position {
                attrs = attrs.with_position(PhysicalPosition::new(x * scale, y * scale));
            }
        } else {
            attrs = attrs.with_inner_size(LogicalSize::new(spec.width, spec.height));
            if let Some((w, h)) = s.min_size {
                attrs = attrs.with_min_inner_size(LogicalSize::new(w, h));
            }
            if let Some((w, h)) = s.max_size {
                attrs = attrs.with_max_inner_size(LogicalSize::new(w, h));
            }
            if let Some((x, y)) = s.position {
                attrs = attrs.with_position(LogicalPosition::new(x, y));
            }
        }
        if s.always_on_top {
            attrs = attrs.with_window_level(WindowLevel::AlwaysOnTop);
        }
        if s.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        if let Some(name) = &s.icon {
            let tint = s.icon_tint.unwrap_or(self.theme.accent);
            match window_icon(&self.assets, name, tint) {
                Some(icon) => attrs = attrs.with_window_icon(Some(icon)),
                None => log::warn!(
                    "sequoia-platform: cannot set window icon `{name}` (missing from registry?)"
                ),
            }
        }

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .map_err(|e| AppError::Window(format!("{id}: {e}")))?,
        );

        window.set_ime_allowed(true);

        window.set_cursor_visible(s.cursor_visible);
        if s.cursor_grab
            && let Err(e) = grab_cursor(&window)
        {
            log::warn!("sequoia-platform: failed to confine cursor: {e}");
        }

        let size = window.inner_size();
        let mut gpu = GpuRenderer::new_with_options(
            window.clone(),
            size.width,
            size.height,
            GpuOptions::with_backend(self.backend)
                .present_mode(s.present_mode)
                .power_preference(s.power_preference),
        )
        .map_err(|e| AppError::Gpu(format!("backend {} ({id}): {e}", self.backend.name())))?;
        gpu.set_clear_color(window_clear_color(self.theme, s.transparent));

        {
            let info = gpu.adapter_info();
            eprintln!(
                "sequoia-platform: GPU `{}` (wgpu {:?}, present {})",
                info.name,
                info.backend,
                s.present_mode.name()
            );
        }

        let scale = s
            .scale_override
            .map(sanitize_scale)
            .unwrap_or_else(|| sanitize_scale(window.scale_factor()));
        let mut text = TextSystem::new();
        text.set_scale_factor(scale);
        install_fonts(&self.assets, &mut text);

        let rt = Runtime::new();
        rt.set_window_id(id);
        rt.set_root_fill(true);
        if let Some(root) = spec.root.clone() {
            rt.mount_any(root);
        }
        rt.set_environment(self.env.clone());

        {
            let window = window.clone();
            rt.set_redraw_hook(move || window.request_redraw());
        }

        let wid = window.id();
        self.id_map.insert(id, wid);
        let continuous_paced = self.redraw_mode == RedrawMode::Continuous && s.fps_limit.is_some();
        let mut state = WindowState {
            id,
            window,
            rt,
            gpu,
            text,
            cursor: Point::ZERO,
            mods: Modifiers::NONE,
            scale,
            last_cursor: None,
            settings: s.clone(),
            next_frame: initial_next_frame(continuous_paced, s.fps_limit),
            fps_base: spec.title.clone(),
            fps_frames: std::collections::VecDeque::new(),
            fps_last_push: None,
            continuous_paced,
            list: RenderList::new(),
        };

        state.draw_frame(wid);
        if show_after_first_frame {
            state.window.set_visible(true);
        }
        let redraw = state.window.clone();
        self.live.insert(wid, state);
        redraw.request_redraw();
        log::debug!("sequoia-platform: created window {id} (winit {wid:?})");
        Ok(())
    }

    fn destroy_window(&mut self, event_loop: &ActiveEventLoop, id: WindowId) {
        let Some(wid) = self.id_map.remove(&id) else {
            return;
        };
        self.live.remove(&wid);
        forget_frame_stats(wid);
        log::debug!("sequoia-platform: closed window {id}");
        if self.windows.count() == 0 {
            event_loop.exit();
        }
    }

    fn drain_requests(&mut self, event_loop: &ActiveEventLoop) {
        for req in self.windows.take_requests() {
            match req {
                window::WindowRequest::Open(id) => {
                    let Some(spec) = self.windows.spec_of(id) else {
                        continue;
                    };
                    if self.id_map.contains_key(&id) {
                        continue;
                    }
                    if let Err(e) = self.create_window(event_loop, id, &spec) {
                        self.fail(event_loop, e);
                        return;
                    }
                }
                window::WindowRequest::Close(id) => {
                    self.destroy_window(event_loop, id);
                }
                window::WindowRequest::SetTitle(id, title) => {
                    if let Some(wid) = self.id_map.get(&id)
                        && let Some(ws) = self.live.get_mut(wid)
                    {
                        ws.fps_base = title.clone();
                        if ws.settings.show_fps {
                            match ws.fps_sample_stats() {
                                Some((count, p50, p95)) => {
                                    let t = if ws.continuous_paced {
                                        fps_title_detailed(&ws.fps_base, count, p50, p95)
                                    } else {
                                        fps_title_for_sample(&ws.fps_base, count, p50, p95)
                                    };
                                    ws.fps_last_push = Some(Instant::now());
                                    ws.fps_push_title(t);
                                }
                                None => {
                                    ws.fps_last_push = Some(Instant::now());
                                    let base = ws.fps_base.clone();
                                    ws.fps_push_title(fps_title_idle(&base));
                                }
                            }
                        } else {
                            ws.window.set_title(&title);
                        }
                    }
                }
                window::WindowRequest::Minimize(id) => {
                    if let Some(wid) = self.id_map.get(&id)
                        && let Some(ws) = self.live.get_mut(wid)
                    {
                        ws.window.set_minimized(true);
                    }
                }
                window::WindowRequest::ToggleMaximize(id) => {
                    if let Some(wid) = self.id_map.get(&id)
                        && let Some(ws) = self.live.get_mut(wid)
                    {
                        let maximize = !ws.window.is_maximized();
                        ws.window.set_maximized(maximize);
                    }
                }
                window::WindowRequest::BeginDrag(id) => {
                    if let Some(wid) = self.id_map.get(&id)
                        && let Some(ws) = self.live.get_mut(wid)
                        && let Err(e) = ws.window.drag_window()
                    {
                        log::warn!("sequoia-platform: window drag failed: {e}");
                    }
                }
            }
        }
    }

    fn earliest_frame_deadline(&self) -> Option<Instant> {
        self.live.values().filter_map(|ws| ws.next_frame).min()
    }

    fn fps_refresh_deadline(&self) -> Option<Instant> {
        if self
            .live
            .values()
            .any(|ws| ws.settings.show_fps && !ws.fps_frames.is_empty())
        {
            Some(Instant::now() + FPS_PUSH_EVERY)
        } else {
            None
        }
    }

    fn update_control_flow(&self, event_loop: &ActiveEventLoop) {
        if self.redraw_mode == RedrawMode::Continuous
            && self.live.values().any(|ws| ws.settings.fps_limit.is_some())
        {
            let deadline = match (self.earliest_frame_deadline(), self.fps_refresh_deadline()) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
            event_loop.set_control_flow(pacing_control_flow(deadline));
        } else if let Some(deadline) = self.fps_refresh_deadline() {
            event_loop.set_control_flow(pacing_control_flow(Some(deadline)));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn close_requested(&mut self, event_loop: &ActiveEventLoop, wid: WinitWindowId) {
        let Some(ws) = self.live.get(&wid) else {
            return;
        };
        let id = ws.id;
        let allow = self
            .windows
            .spec_of(id)
            .and_then(|s| s.on_close_request.clone())
            .is_none_or(|f| f());
        if allow {
            self.windows.close(id);
            self.drain_requests(event_loop);
        }
    }
}

fn sync_ime_cursor(window: &Window, scale: f32, rt: &Runtime) {
    let Some(rect) = rt.ime_cursor_rect() else {
        return;
    };
    let s = scale;
    let x = (rect.left() * s) as f64;
    let y = (rect.bottom() * s) as f64;
    window.set_ime_cursor_area(
        winit::dpi::PhysicalPosition::new(x, y),
        winit::dpi::PhysicalSize::new(1u32, 1u32),
    );
}

fn install_fonts(assets: &AssetRegistry, text: &mut TextSystem) {
    let mut families: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (alias, bytes) in assets.fonts() {
        match text.load_font(bytes.to_vec()) {
            Some(family) => {
                log::info!("sequoia-platform: loaded font '{alias}' as family '{family}'");
                text.register_alias(alias.clone(), family.clone());
                families.insert(alias, family);
            }
            None => {
                log::warn!("sequoia-platform: failed to load font '{alias}' (invalid font data)");
            }
        }
    }
    match assets.default_font() {
        Some(alias) => {
            if let Some(family) = families.get(&alias) {
                log::info!("sequoia-platform: default font set to '{alias}' (family '{family}')");
                text.set_default_family(Some(family.clone()));
            } else if text.font_families().iter().any(|f| f == &alias) {
                log::info!(
                    "sequoia-platform: default font set to system family '{alias}' (no bundled data needed)"
                );
                text.set_default_family(Some(alias));
            } else {
                log::warn!(
                    "sequoia-platform: default font alias '{alias}' not found among registered fonts nor system families - keeping '{}'",
                    text.default_family().unwrap_or("Inter")
                );
            }
        }
        None => {
            log::debug!("sequoia-platform: no custom default font, using built-in Inter");
        }
    }
}

fn map_key(key: &WinitKey) -> Option<Key> {
    match key {
        WinitKey::Named(NamedKey::Tab) => Some(Key::TAB),
        WinitKey::Named(NamedKey::Enter) => Some(Key::ENTER),
        WinitKey::Named(NamedKey::Escape) => Some(Key::ESCAPE),
        WinitKey::Named(NamedKey::Space) => Some(Key::SPACE),
        WinitKey::Named(NamedKey::Backspace) => Some(Key::BACKSPACE),
        WinitKey::Named(NamedKey::Delete) => Some(Key::DELETE),
        WinitKey::Named(NamedKey::Insert) => Some(Key::INSERT),
        WinitKey::Named(NamedKey::ArrowUp) => Some(Key::ARROW_UP),
        WinitKey::Named(NamedKey::ArrowDown) => Some(Key::ARROW_DOWN),
        WinitKey::Named(NamedKey::ArrowLeft) => Some(Key::ARROW_LEFT),
        WinitKey::Named(NamedKey::ArrowRight) => Some(Key::ARROW_RIGHT),
        WinitKey::Named(NamedKey::Home) => Some(Key::HOME),
        WinitKey::Named(NamedKey::End) => Some(Key::END),
        WinitKey::Named(NamedKey::PageUp) => Some(Key::PAGE_UP),
        WinitKey::Named(NamedKey::PageDown) => Some(Key::PAGE_DOWN),
        WinitKey::Named(NamedKey::F1) => Some(Key::F1),
        WinitKey::Named(NamedKey::F2) => Some(Key::F2),
        WinitKey::Named(NamedKey::F3) => Some(Key::F3),
        WinitKey::Named(NamedKey::F4) => Some(Key::F4),
        WinitKey::Named(NamedKey::F5) => Some(Key::F5),
        WinitKey::Named(NamedKey::F6) => Some(Key::F6),
        WinitKey::Named(NamedKey::F7) => Some(Key::F7),
        WinitKey::Named(NamedKey::F8) => Some(Key::F8),
        WinitKey::Named(NamedKey::F9) => Some(Key::F9),
        WinitKey::Named(NamedKey::F10) => Some(Key::F10),
        WinitKey::Named(NamedKey::F11) => Some(Key::F11),
        WinitKey::Named(NamedKey::F12) => Some(Key::F12),
        WinitKey::Character(s) => s.chars().next().map(Key::from_char),
        _ => None,
    }
}

impl ApplicationHandler<JobWake> for PlatformApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        for id in std::mem::take(&mut self.initial) {
            if self.id_map.contains_key(&id) {
                continue;
            }
            let Some(spec) = self.windows.spec_of(id) else {
                continue;
            };
            if let Err(e) = self.create_window(event_loop, id, &spec) {
                self.fail(event_loop, e);
                return;
            }
        }
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        wid: WinitWindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) {
            self.close_requested(event_loop, wid);
            return;
        }

        let redraw_mode = self.redraw_mode;
        {
            let Some(ws) = self.live.get_mut(&wid) else {
                return;
            };
            match event {
                WindowEvent::Resized(size) => {
                    ws.gpu.resize(size.width, size.height);
                    ws.window.request_redraw();
                }

                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    if ws.settings.scale_override.is_none() {
                        ws.scale = sanitize_scale(scale_factor);
                        ws.text.set_scale_factor(ws.scale);
                        ws.window.request_redraw();
                    }
                }

                WindowEvent::Occluded(occluded) => {
                    if !occluded {
                        ws.window.request_redraw();
                    }
                }

                WindowEvent::ModifiersChanged(m) => {
                    let mstate = m.state();
                    let mut mods = Modifiers::NONE;
                    if mstate.shift_key() {
                        mods |= Modifiers::SHIFT;
                    }
                    if mstate.control_key() {
                        mods |= Modifiers::CONTROL;
                    }
                    if mstate.alt_key() {
                        mods |= Modifiers::ALT;
                    }
                    if mstate.super_key() {
                        mods |= Modifiers::SUPER;
                    }
                    ws.mods = mods;
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let s = ws.scale;
                    ws.cursor = Point::new(position.x as f32 / s, position.y as f32 / s);

                    ws.rt.pointer_move(ws.cursor, &mut ws.text);
                    sync_cursor(&ws.window, &mut ws.last_cursor, &ws.rt);
                }

                WindowEvent::MouseInput {
                    state: button_state,
                    button,
                    ..
                } => {
                    let pb = match button {
                        MouseButton::Left => PointerButton::Left,
                        MouseButton::Right => PointerButton::Right,
                        MouseButton::Middle => PointerButton::Middle,
                        _ => return,
                    };
                    match button_state {
                        ElementState::Pressed => {
                            ws.rt.pointer_down(ws.cursor, pb, ws.mods, &mut ws.text);
                        }
                        ElementState::Released => {
                            ws.rt.pointer_up(ws.cursor, pb, ws.mods, &mut ws.text);
                        }
                    }

                    sync_cursor(&ws.window, &mut ws.last_cursor, &ws.rt);
                }

                WindowEvent::KeyboardInput { event: kev, .. } => {
                    if kev.state == ElementState::Pressed
                        && let Some(k) = map_key(&kev.logical_key)
                        && (!kev.repeat || k.allows_repeat())
                    {
                        let mut hit = false;
                        let physical = ws.window.inner_size();
                        let ctx = AcceleratorCtx {
                            window: ws.id,
                            size: Size::new(
                                physical.width as f32 / ws.scale,
                                physical.height as f32 / ws.scale,
                            ),
                            scale: ws.scale,
                            environment: self.env.clone(),
                        };
                        for acc in &self.accelerators {
                            if acc.key == k && acc.modifiers == ws.mods && acc.should_fire() {
                                acc.fire(&ctx);
                                hit = true;
                                break;
                            }
                        }
                        if hit {
                            ws.window.request_redraw();
                        } else {
                            ws.rt.key(k, ws.mods, &mut ws.text);
                        }
                    }
                    sync_ime_cursor(&ws.window, ws.scale, &ws.rt);
                }

                WindowEvent::Ime(ime) => {
                    use winit::event::Ime as Wime;
                    let ev = match ime {
                        Wime::Enabled => Some(ImeEvent::Enabled),
                        Wime::Preedit(value, cursor) => Some(ImeEvent::Preedit { value, cursor }),
                        Wime::Commit(value) => Some(ImeEvent::Commit(value)),
                        Wime::Disabled => Some(ImeEvent::Disabled),
                    };
                    if let Some(ev) = ev {
                        ws.rt.ime(ev, &mut ws.text);
                        sync_ime_cursor(&ws.window, ws.scale, &ws.rt);
                    }
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let s = ws.scale;
                    let (dx, dy) = match delta {
                        winit::event::MouseScrollDelta::LineDelta(x, y) => (x * 40.0, y * 40.0),

                        winit::event::MouseScrollDelta::PixelDelta(p) => {
                            (p.x as f32 / s, p.y as f32 / s)
                        }
                    };
                    ws.rt.wheel(
                        ws.cursor,
                        sequoia_core::Vec2::new(dx, dy),
                        ws.mods,
                        &mut ws.text,
                    );
                }

                WindowEvent::RedrawRequested => {
                    ws.draw_frame(wid);
                    if redraw_mode == RedrawMode::Continuous && !ws.continuous_paced {
                        ws.window.request_redraw();
                    }
                }

                WindowEvent::Focused(false) => {
                    ws.rt.pointer_cancel(&mut ws.text);
                }

                WindowEvent::CursorLeft { .. } => {
                    ws.rt.pointer_cancel(&mut ws.text);
                }

                WindowEvent::Touch(touch) => {
                    use winit::event::TouchPhase;
                    let s = ws.scale;
                    let pos = Point::new(touch.location.x as f32 / s, touch.location.y as f32 / s);
                    ws.cursor = pos;

                    let force = touch.force.as_ref().map(|f| f.normalized() as f32);
                    match touch.phase {
                        TouchPhase::Started => {
                            ws.rt
                                .touch_down(touch.id, pos, force, ws.mods, &mut ws.text);
                        }
                        TouchPhase::Moved => {
                            ws.rt.touch_move(touch.id, pos, force, &mut ws.text);
                        }
                        TouchPhase::Ended => {
                            ws.rt.touch_up(touch.id, pos, ws.mods, &mut ws.text);
                        }
                        TouchPhase::Cancelled => {
                            ws.rt.touch_up(touch.id, pos, ws.mods, &mut ws.text);
                        }
                    }
                    sync_cursor(&ws.window, &mut ws.last_cursor, &ws.rt);
                }

                WindowEvent::PinchGesture { delta, .. } => {
                    ws.rt.pinch(ws.cursor, delta as f32, ws.mods, &mut ws.text);
                }

                WindowEvent::PanGesture { delta, .. } => {
                    let s = ws.scale;
                    ws.rt.wheel(
                        ws.cursor,
                        sequoia_core::Vec2::new(delta.x / s, delta.y / s),
                        ws.mods,
                        &mut ws.text,
                    );
                }

                WindowEvent::DroppedFile(path) => {
                    ws.rt.files_dropped(vec![path], ws.cursor, &mut ws.text);
                }
                WindowEvent::HoveredFile(path) => {
                    ws.rt.files_hovered(vec![path], ws.cursor, &mut ws.text);
                }
                WindowEvent::HoveredFileCancelled => {
                    ws.rt.files_cancelled(&mut ws.text);
                }

                WindowEvent::AxisMotion { axis, value, .. } => {
                    let delta = match axis {
                        0 => sequoia_core::Vec2::new(value as f32 * 20.0, 0.0),
                        1 => sequoia_core::Vec2::new(0.0, value as f32 * 20.0),
                        _ => sequoia_core::Vec2::new(0.0, 0.0),
                    };
                    if delta.x != 0.0 || delta.y != 0.0 {
                        ws.rt.wheel(ws.cursor, delta, ws.mods, &mut ws.text);
                    }
                }

                _ => {}
            }
        }

        self.drain_requests(event_loop);
        self.update_control_flow(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.drain_requests(event_loop);

        if let Some(mgr) = self.global_hotkeys.as_ref() {
            let fired = mgr.drain();
            if !fired.is_empty() {
                for id in fired {
                    if let Some(action) = self.global_actions.get(&id) {
                        action();
                    }
                }
                for ws in self.live.values() {
                    ws.window.request_redraw();
                }
            }
        }
        if self.live.is_empty() {
            return;
        }

        for ws in self.live.values_mut() {
            ws.refresh_fps_title();
        }
        if self.redraw_mode != RedrawMode::Continuous {
            self.update_control_flow(event_loop);
            return;
        }
        let now = Instant::now();
        for ws in self.live.values_mut() {
            let Some(fps) = ws.settings.fps_limit else {
                continue;
            };
            let Some(deadline) = ws.next_frame else {
                continue;
            };
            if now >= deadline {
                ws.next_frame = Some(frame_deadline(fps));
                ws.window.request_redraw();
            }
        }
        self.update_control_flow(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: JobWake) {
        if self.executor.drain() == 0 {
            return;
        }
        for ws in self.live.values() {
            ws.window.request_redraw();
        }
        self.update_control_flow(event_loop);
    }
}

fn frame_deadline(fps: u32) -> Instant {
    Instant::now() + Duration::from_secs_f64(1.0 / f64::from(fps.max(1)))
}

fn initial_next_frame(continuous_paced: bool, fps_limit: Option<u32>) -> Option<Instant> {
    if continuous_paced {
        fps_limit.map(frame_deadline)
    } else {
        None
    }
}

fn window_clear_color(theme: Theme, transparent: bool) -> Color {
    let mut color = theme.background;
    if transparent {
        color.a = 0.0;
    }
    color
}

fn pacing_control_flow(deadline: Option<Instant>) -> ControlFlow {
    match deadline {
        Some(deadline) if deadline > Instant::now() => ControlFlow::WaitUntil(deadline),
        _ => ControlFlow::Wait,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_view::Unit;

    #[test]
    fn app_builder_targets_primary_and_counts_windows() {
        let app = App::new()
            .title("Main")
            .size(800.0, 600.0)
            .window(WindowSpec::new("Second", Unit).size(400.0, 300.0))
            .window(WindowSpec::new("Third", Unit));
        assert_eq!(app.window_count(), 3);
        assert_eq!(app.windows[0].title, "Main");
        assert_eq!(
            (app.windows[0].width, app.windows[0].height),
            (800.0, 600.0)
        );
        assert_eq!(app.windows[1].title, "Second");
        assert_eq!(
            (app.windows[1].width, app.windows[1].height),
            (400.0, 300.0)
        );
        assert_eq!(app.windows[2].title, "Third");
    }

    #[test]
    fn app_default_is_single_untitled_window() {
        let app = App::new();
        assert_eq!(app.window_count(), 1);
        assert_eq!(app.windows[0].title, "Sequoia");
        assert!(app.windows[0].root.is_none());
    }

    #[test]
    fn window_tint_scales_coverage_into_alpha() {
        let rgba = tint_rgba(&[0, 128, 255], Color::rgb8(0, 168, 232));
        assert_eq!(rgba.len(), 12, "RGBA per mask pixel");
        assert_eq!(
            &rgba[0..4],
            &[0, 168, 232, 0],
            "first: color + full coverage"
        );
        assert_eq!(&rgba[4..8], &[0, 168, 232, 128], "middle: half coverage");
        assert_eq!(&rgba[8..12], &[0, 168, 232, 255]);

        let half = Color {
            a: 0.5,
            ..Color::rgb8(255, 255, 255)
        };
        assert_eq!(tint_rgba(&[255], half)[3], 128);
    }

    #[test]
    fn fps_flag_reaches_window_settings() {
        let app = App::new().show_fps(true);
        assert!(app.windows[0].settings.show_fps);
    }

    #[test]
    fn fps_detailed_shows_throughput_and_tail() {
        assert_eq!(
            fps_title_detailed("Notka", 60, 16.0, 17.0),
            "Notka — 60 FPS · p50 16ms p95 17ms"
        );
        assert_eq!(
            fps_title_detailed("Notka", 4, 25.0, 250.0),
            "Notka — 4 FPS · p50 25ms p95 250ms",
            "hitch is visible in p95, not in average"
        );
        assert_eq!(fps_title_idle("Notka"), "Notka — IDLE");
    }

    #[test]
    fn fps_sparse_fast_frames_are_idle_not_low_fps() {
        assert!(fps_sample_is_idle(3, 2.0));
        assert!(fps_sample_is_idle(3, 17.0));
        assert_eq!(
            fps_title_for_sample("App", 3, 2.0, 2.5),
            "App — IDLE · p50 2ms (~500 FPS)"
        );
        assert_eq!(fps_title_for_sample("App", 0, 0.0, 0.0), "App — IDLE");
    }

    #[test]
    fn fps_sparse_slow_frames_stay_low_fps() {
        assert!(!fps_sample_is_idle(4, 250.0));
        assert!(!fps_sample_is_idle(5, 50.0));
        assert_eq!(
            fps_title_for_sample("App", 4, 25.0, 250.0),
            "App — 4 FPS · p50 25ms p95 250ms"
        );
    }

    #[test]
    fn fps_busy_fast_stream_shows_real_fps() {
        assert!(!fps_sample_is_idle(60, 16.0));
        assert_eq!(
            fps_title_for_sample("App", 60, 16.0, 17.0),
            "App — 60 FPS · p50 16ms p95 17ms"
        );
    }

    #[test]
    fn percentile_sorted_picks_rank() {
        assert_eq!(percentile_sorted(&[], 0.5), 0.0);
        let v = vec![10.0, 20.0, 30.0, 40.0];
        assert_eq!(percentile_sorted(&v, 0.0), 10.0);
        assert_eq!(percentile_sorted(&v, 0.5), 30.0, "rank round((4-1)*0.5)=2");
        assert_eq!(percentile_sorted(&v, 1.0), 40.0);
        assert_eq!(percentile_sorted(&v, 5.0), 40.0, "clamp do max");
    }

    #[test]
    fn window_icon_comes_from_the_asset_registry() {
        let assets = AssetRegistry::with_builtin_icons();
        assert!(window_icon(&assets, "star", Color::WHITE).is_some());
        assert!(window_icon(&assets, "no-such-icon", Color::WHITE).is_none());
    }

    #[test]
    fn scale_override_is_sanitized_and_stored() {
        assert_eq!(sanitize_scale(1.25), 1.25);
        assert_eq!(sanitize_scale(0.0), 1.0);
        assert_eq!(sanitize_scale(-2.0), 1.0);
        assert_eq!(sanitize_scale(f64::NAN), 1.0);
        assert_eq!(sanitize_scale(f64::INFINITY), 1.0);

        let app = App::new().scale_factor(1.0);
        assert_eq!(app.windows[0].settings.scale_override, Some(1.0));
    }

    #[test]
    fn window_settings_default_to_system_behaviour() {
        let s = WindowSettings::default();
        assert!(
            s.resizable && s.decorations && s.visible,
            "window as before"
        );
        assert!(!s.transparent && !s.always_on_top && !s.fullscreen && !s.maximized);
        assert!(s.cursor_visible && !s.cursor_grab);
        assert!(s.scale_override.is_none() && s.fps_limit.is_none());
        assert!(!s.show_fps);
        assert!(s.min_size.is_none() && s.max_size.is_none() && s.position.is_none());
        assert_eq!(s.present_mode, GpuPresentMode::Vsync);
        assert_eq!(s.power_preference, GpuPowerPreference::HighPerformance);
    }

    #[test]
    fn try_icon_registers_valid_svg_and_reports_broken() {
        let ok = App::new()
            .try_icon(
                "dot",
                r#"<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"/></svg>"#,
            )
            .expect("valid svg registers");
        assert!(ok.assets.icon("dot").is_some());

        let err = App::new().try_icon("broken", "<svg><path d='M0 0'/></svg>");
        assert!(err.is_err(), "broken svg must not panic, only error");
    }

    #[test]
    fn frame_pacing_waits_until_deadline_or_stays_idle() {
        assert!(matches!(
            pacing_control_flow(Some(Instant::now() + Duration::from_millis(16))),
            ControlFlow::WaitUntil(_)
        ));
        assert!(matches!(
            pacing_control_flow(Some(Instant::now() - Duration::from_millis(1))),
            ControlFlow::Wait
        ));
        assert!(matches!(pacing_control_flow(None), ControlFlow::Wait));
        assert!(frame_deadline(60) > Instant::now());
    }

    #[test]
    fn paced_windows_seed_first_frame_deadline() {
        let start = Instant::now();
        let deadline = initial_next_frame(true, Some(60)).expect("paced window has a deadline");
        assert!(deadline > start);
        assert!(initial_next_frame(false, Some(60)).is_none());
        assert!(initial_next_frame(true, None).is_none());
    }

    #[test]
    fn window_clear_color_follows_theme() {
        let theme = Theme::dark();
        assert_eq!(window_clear_color(theme, false), theme.background);
        assert_eq!(window_clear_color(theme, true).a, 0.0);
    }
}
