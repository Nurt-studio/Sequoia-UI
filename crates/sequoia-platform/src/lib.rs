#![forbid(unsafe_code)]

use std::cell::RefCell;
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
use sequoia_view::{AnyView, CullCtx, FrameOutcome, Runtime, View};

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
use sequoia_render::{FrameSkip, RenderStats};
pub use sequoia_render::{
    GpuBackend, MemoryBudget, PowerPreference as GpuPowerPreference, PresentMode as GpuPresentMode,
    QualityLevel, Region, SharedGpu, WgpuFrame, WgpuRenderer, WgpuSurface, wgpu,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
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

/// Controls when the platform schedules frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RedrawMode {
    /// Paint only after a redraw request; idle windows wait for events.
    #[default]
    OnDemand,

    /// Keep scheduling frames (optionally paced by `fps_limit`).
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

/// Plain-key accelerators must not fire while a text input has focus.
///
/// Modified shortcuts (Ctrl/Alt/Super) still pass through, so commands like
/// Ctrl+S remain usable while typing.
fn accelerator_allowed(modifiers: Modifiers, text_input_focused: bool) -> bool {
    !text_input_focused
        || modifiers.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
}

#[derive(Clone, Copy, Default)]
struct BenchSample {
    gap_ms: f64,
    tick_ms: f64,
    layout_ms: f64,
    paint_ms: f64,
    render_ms: f64,
    stats: RenderStats,
}

#[derive(Default)]
struct Bench {
    remaining: u32,
    samples: Vec<BenchSample>,
    done: bool,
    reported: bool,
}

impl Bench {
    fn new(frames: u32) -> Self {
        Self {
            remaining: frames.max(1),
            samples: Vec::with_capacity(frames as usize),
            done: false,
            reported: false,
        }
    }

    fn sample(&mut self, sample: BenchSample) {
        if self.done {
            return;
        }
        self.samples.push(sample);
        self.remaining = self.remaining.saturating_sub(1);
        self.done = self.remaining == 0;
    }

    fn print_report(&self) {
        let skip = (self.samples.len() / 10).min(self.samples.len().saturating_sub(1));
        let samples = &self.samples[skip..];
        if samples.is_empty() {
            return;
        }
        let n = samples.len() as f64;
        let mean = |f: fn(&BenchSample) -> f64| samples.iter().map(f).sum::<f64>() / n;
        let mut render: Vec<f64> = samples.iter().map(|s| s.render_ms).collect();
        render.sort_by(f64::total_cmp);
        let total_draws: u64 = samples.iter().map(|s| s.stats.draw_calls).sum();
        let total_passes: u64 = samples.iter().map(|s| s.stats.passes).sum();
        let total_quads: u64 = samples.iter().map(|s| s.stats.quad_instances).sum();
        let total_glyphs: u64 = samples.iter().map(|s| s.stats.glyph_instances).sum();
        let total_images: u64 = samples.iter().map(|s| s.stats.image_instances).sum();
        println!(
            "[stress] frames={} avg(gap={:.2} tick={:.2} layout={:.2} paint={:.2} render={:.2}ms) \
             p50_render={:.2}ms p95_render={:.2}ms draws={:.1} passes={:.1} \
             quads={:.1} glyphs={:.1} images={:.1}",
            samples.len(),
            mean(|s| s.gap_ms),
            mean(|s| s.tick_ms),
            mean(|s| s.layout_ms),
            mean(|s| s.paint_ms),
            mean(|s| s.render_ms),
            render[render.len() / 2],
            render[((render.len() as f64 * 0.95) as usize).min(render.len() - 1)],
            total_draws as f64 / n,
            total_passes as f64 / n,
            total_quads as f64 / n,
            total_glyphs as f64 / n,
            total_images as f64 / n,
        );
    }
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
    shaders: Vec<(String, String)>,
    shader_watch: Option<std::path::PathBuf>,
    bench_frames: Option<u32>,
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
            shaders: sequoia_shader::BUILTINS
                .iter()
                .map(|(name, wgsl)| ((*name).to_string(), (*wgsl).to_string()))
                .collect(),
            shader_watch: None,
            bench_frames: None,
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

    pub fn icons_dir(self, dir: impl AsRef<std::path::Path>) -> Self {
        match self.assets.register_icon_dir(dir) {
            Ok(report) => {
                for (path, error) in report.failed {
                    log::warn!(
                        "sequoia-platform: invalid SVG `{}`: {error} - icon skipped",
                        path.display()
                    );
                }
                log::debug!(
                    "sequoia-platform: registered {} icons from directory",
                    report.registered.len()
                );
            }
            Err(error) => {
                log::error!("sequoia-platform: failed to read icon directory: {error}");
            }
        }
        self
    }

    /// Registers a custom shader by name with a complete WGSL module.
    ///
    /// The module must expose `vs_main` and `fs_main` entry points matching the
    /// engine ABI. Use [`sequoia_render::compose_user_shader`] to wrap a bare
    /// `fs_user` fragment function when no custom vertex stage is needed.
    ///
    /// Registration happens when the window's GPU renderer is created; errors
    /// are logged and the shader is skipped.
    #[must_use]
    pub fn shader(mut self, name: impl Into<String>, wgsl: impl Into<String>) -> Self {
        let name = name.into();
        let wgsl = wgsl.into();
        if let Some(existing) = self.shaders.iter_mut().find(|(entry, _)| *entry == name) {
            existing.1 = wgsl;
        } else {
            self.shaders.push((name, wgsl));
        }
        self
    }

    /// Registers every source collected in a [`sequoia_shader::ShaderRegistry`].
    ///
    /// Later sources replace earlier ones with the same name, matching
    /// [`App::shader`].
    #[must_use]
    pub fn shader_registry(mut self, registry: sequoia_shader::ShaderRegistry) -> Self {
        for (name, wgsl) in registry.into_entries() {
            self = self.shader(name, wgsl);
        }
        self
    }

    /// Registers a compile-time shader generated by [`sequoia_shader::shader`].
    #[must_use]
    pub fn shader_type<S: sequoia_shader::SequoiShader>(self) -> Self {
        self.shader(S::NAME, S::WGSL)
    }

    /// Reads a WGSL shader file and registers it under `name`.
    #[must_use]
    pub fn shader_file(self, name: impl Into<String>, path: impl AsRef<std::path::Path>) -> Self {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(wgsl) => self.shader(name, wgsl),
            Err(e) => {
                log::error!(
                    "sequoia-platform: failed to read shader file `{}`: {e}",
                    path.display()
                );
                self
            }
        }
    }

    /// Recursively registers every `.wgsl` file in `dir`.
    ///
    /// The shader name is the relative path from `dir` without the `.wgsl`
    /// extension, using `/` as separator (e.g. `effects/invert`).
    #[must_use]
    pub fn shader_dir(mut self, dir: impl AsRef<std::path::Path>) -> Self {
        let dir = dir.as_ref();
        match collect_shaders_dir(dir, dir, &mut self.shaders) {
            Ok(()) => {}
            Err(e) => {
                log::error!(
                    "sequoia-platform: failed to read shader directory `{}`: {e}",
                    dir.display()
                );
            }
        }
        self
    }

    /// Loads `assets/shaders` and hot-reloads every `.wgsl` file edited while
    /// the app runs.
    ///
    /// This is [`App::watch_shader_dir`] with the conventional project path.
    #[must_use]
    pub fn watch_shaders(self) -> Self {
        self.watch_shader_dir("assets/shaders")
    }

    /// Loads `dir` and hot-reloads every `.wgsl` file edited while the app
    /// runs.
    ///
    /// The directory is loaded immediately with [`App::shader_dir`]; afterwards
    /// the event loop polls it a few times per second. A changed file is read,
    /// registered on every open window, and the window repaints. Shader names
    /// stay stable, so a reload replaces the effect in place. Compile errors
    /// are logged and the previous shader keeps rendering.
    #[must_use]
    pub fn watch_shader_dir(mut self, dir: impl AsRef<std::path::Path>) -> Self {
        let dir = dir.as_ref().to_path_buf();
        self = self.shader_dir(&dir);
        self.shader_watch = Some(dir);
        self
    }

    /// Returns the registered shaders as `(name, wgsl)` pairs.
    #[must_use]
    pub fn shaders(&self) -> &[(String, String)] {
        &self.shaders
    }

    /// Names of registered shaders in registration order.
    pub fn shader_names(&self) -> impl Iterator<Item = &str> {
        self.shaders.iter().map(|(name, _)| name.as_str())
    }

    /// Whether a shader with `name` is registered.
    #[must_use]
    pub fn has_shader(&self, name: &str) -> bool {
        self.shaders.iter().any(|(entry, _)| entry == name)
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

    /// Requests a swapchain format for the primary window.
    pub fn surface_format(mut self, format: sequoia_render::wgpu::TextureFormat) -> Self {
        self.primary_mut().settings.surface_format = Some(format);
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

    /// Runs `frames` frames and prints aggregated frame and renderer timings.
    ///
    /// The first 10% of frames are treated as warmup. The application exits
    /// automatically after the sample window, which makes this suitable for
    /// before/after performance comparisons from a terminal.
    pub fn benchmark(mut self, frames: u32) -> Self {
        self.bench_frames = Some(frames.max(1));
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
        let bench = self
            .bench_frames
            .map(|frames| Rc::new(RefCell::new(Bench::new(frames))));
        let mut handler = PlatformApp {
            theme: self.theme,
            backend: self.backend,
            bench: bench.clone(),
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
            shared_gpu: None,
            last_input: None,
            id_map: HashMap::new(),
            error: None,
            shaders: self.shaders,
            shader_watch: self.shader_watch.map(ShaderWatch::new),
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

#[derive(Default)]
struct PaintGate {
    requested: bool,
    redraw_pending: bool,
    skip_next_redraw: bool,
}

impl PaintGate {
    fn request(&mut self) {
        self.requested = true;
    }

    fn has_request(&self) -> bool {
        self.requested
    }

    fn clear_request(&mut self) {
        self.requested = false;
    }

    fn schedule(&mut self) {
        self.redraw_pending = true;
        self.skip_next_redraw = false;
    }

    fn on_redraw(&mut self) -> bool {
        self.redraw_pending = false;
        if self.skip_next_redraw && !self.requested {
            self.skip_next_redraw = false;
            false
        } else {
            self.skip_next_redraw = false;
            true
        }
    }

    fn begin_paint(&mut self) -> bool {
        self.requested = false;
        if self.redraw_pending {
            self.redraw_pending = false;
            self.skip_next_redraw = true;
            true
        } else {
            false
        }
    }

    fn end_paint(&mut self, superseded: bool) -> bool {
        if !std::mem::take(&mut self.requested) {
            return false;
        }
        if superseded {
            self.skip_next_redraw = false;
            false
        } else {
            self.redraw_pending = true;
            true
        }
    }
}

/// The wheel reaches the application via two paths: `WindowEvent::MouseWheel`
/// (with the cursor position) and `DeviceEvent::MouseWheel` from raw input.
///
/// On Windows both carry the same event, and precise touchpads can report
/// horizontal movement only through raw input, so neither path can simply be
/// disabled. Instead we count each axis once: through the path that reported it
/// first, discarding the copy from the other path.
///
/// A rejected copy does not refresh the timestamp: otherwise, with a continuous
/// stream from both paths, they would keep silencing each other forever and, for
/// example, horizontal touchpad gestures would stop reaching the application.
const WHEEL_DUPLICATE_MS: Duration = Duration::from_millis(50);

#[derive(Default)]
struct WheelDedupe {
    window: [Option<Instant>; 2],
    device: [Option<Instant>; 2],
}

impl WheelDedupe {
    fn fresh(seen: &[Option<Instant>; 2], axis: usize, now: Instant) -> bool {
        seen[axis].is_none_or(|t| now.duration_since(t) >= WHEEL_DUPLICATE_MS)
    }

    /// An axis is kept if the other path did not report it just now.
    /// The timestamp is set only for axes that pass through.
    fn take(
        seen: &mut [Option<Instant>; 2],
        other: &[Option<Instant>; 2],
        d: sequoia_core::Vec2,
        now: Instant,
    ) -> sequoia_core::Vec2 {
        let x = if d.x != 0.0 && Self::fresh(other, 0, now) {
            d.x
        } else {
            0.0
        };
        let y = if d.y != 0.0 && Self::fresh(other, 1, now) {
            d.y
        } else {
            0.0
        };
        if x != 0.0 {
            seen[0] = Some(now);
        }
        if y != 0.0 {
            seen[1] = Some(now);
        }
        sequoia_core::Vec2::new(x, y)
    }

    fn window(&mut self, d: sequoia_core::Vec2, now: Instant) -> sequoia_core::Vec2 {
        Self::take(&mut self.window, &self.device, d, now)
    }

    fn device(&mut self, d: sequoia_core::Vec2, now: Instant) -> sequoia_core::Vec2 {
        Self::take(&mut self.device, &self.window, d, now)
    }
}

/// Converts a wheel delta to logical pixels (one line is 40 px).
fn wheel_points(delta: winit::event::MouseScrollDelta, scale: f32) -> sequoia_core::Vec2 {
    match delta {
        winit::event::MouseScrollDelta::LineDelta(x, y) => {
            sequoia_core::Vec2::new(x * 40.0, y * 40.0)
        }
        winit::event::MouseScrollDelta::PixelDelta(p) => {
            sequoia_core::Vec2::new(p.x as f32 / scale, p.y as f32 / scale)
        }
    }
}

struct WindowState {
    id: WindowId,
    window: Arc<Window>,
    rt: Runtime,
    gpu: GpuRenderer,
    text: TextSystem,
    cursor: Point,
    mods: Modifiers,
    wheel_seen: WheelDedupe,

    scale: f32,

    last_cursor: Option<winit::window::CursorIcon>,

    ime_allowed: bool,

    settings: WindowSettings,

    next_frame: Option<Instant>,

    fps_base: String,
    fps_frames: std::collections::VecDeque<(Instant, f64)>,
    fps_last_push: Option<Instant>,

    continuous_paced: bool,

    gate: Rc<RefCell<PaintGate>>,

    list: RenderList,

    bench: Option<Rc<RefCell<Bench>>>,
}

impl WindowState {
    fn request_redraw(&mut self) {
        self.gate.borrow_mut().schedule();
        self.window.request_redraw();
    }

    fn paint_if_requested(&mut self) {
        if !self.gate.borrow().has_request() {
            return;
        }
        let wid = self.window.id();
        self.draw_frame(wid);
        if self.continuous_paced
            && let Some(fps) = self.settings.fps_limit
        {
            self.next_frame = Some(frame_deadline(fps));
        }
    }

    fn draw_frame(&mut self, wid: WinitWindowId) {
        let superseded = self.gate.borrow_mut().begin_paint();
        let gap_ms = frame_gap_ms(wid);
        if self.bench.is_some() {
            // A benchmark measures whole-frame paint/render cost, so opt out
            // of damage-based skipping for every sampled frame.
            self.rt.request_frame();
        }
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
        let mut epoch = self.text.atlas().epoch();
        let mut passes = 1;
        // `paint_if_dirty` clears the tracker, so capture the regions first.
        let damage = self.rt.damage_regions();
        let outcome = self.rt.paint_if_dirty(&mut self.list, &mut self.text);
        // The glyph atlas can grow, repack or clear while we paint. Entries
        // pushed before that carry UVs for the previous atlas layout, so those
        // glyphs would sample the wrong pixels. Repaint until the atlas is
        // stable, exactly like offscreen export does.
        while outcome == FrameOutcome::Painted
            && self.text.atlas().epoch() != epoch
            && passes < ATLAS_REPAINT_LIMIT
        {
            epoch = self.text.atlas().epoch();
            self.list.clear();
            self.rt
                .paint_region(&mut self.list, &mut self.text, CullCtx::ENABLED);
            passes += 1;
        }
        let surfaces = self.rt.take_surface_draws();
        let paint_ms = t.elapsed().as_secs_f64() * 1000.0;
        t = Instant::now();
        if outcome == FrameOutcome::Painted
            && let Err(e) = self.gpu.render_with_damage_regions(
                &self.list,
                self.text.atlas_mut(),
                s,
                &surfaces,
                damage.as_deref(),
            )
        {
            log::warn!("sequoia-platform: render error: {e}");
        }
        let render_ms = t.elapsed().as_secs_f64() * 1000.0;
        note_frame(gap_ms, tick_ms, layout_ms, paint_ms, render_ms);
        if let Some(bench) = &self.bench {
            let stats = self.gpu.take_stats();
            bench.borrow_mut().sample(BenchSample {
                gap_ms,
                tick_ms,
                layout_ms,
                paint_ms,
                render_ms,
                stats,
            });
        }
        if self.settings.show_fps {
            self.push_fps_title(tick_ms + layout_ms + paint_ms + render_ms);
        }

        if outcome == FrameOutcome::Painted {
            match self.gpu.take_frame_skip() {
                Some(FrameSkip::Timeout) => self.gate.borrow_mut().request(),
                Some(FrameSkip::Occluded) | None => {}

                Some(_) => {}
            }
        }

        // A surface drew during `render_with_damage_regions` and asked for
        // another frame through `WgpuFrame::request_frame`. Feeding that into
        // the runtime marks motion, so the next `paint_if_dirty` paints instead
        // of skipping and the animation keeps running without input events.
        if self.gpu.take_redraw_request() {
            self.rt.request_frame();
        }

        sync_cursor(&self.window, &mut self.last_cursor, &self.rt);
        sync_ime_allowed(&self.window, &mut self.ime_allowed, &self.rt);
        sync_ime_cursor(&self.window, self.scale, &self.rt);

        if self.gate.borrow_mut().end_paint(superseded) {
            self.window.request_redraw();
        }
    }
}

/// Upper bound on paint passes per frame; one extra pass is normally enough
/// to re-emit glyphs after the atlas grew mid-paint.
const ATLAS_REPAINT_LIMIT: usize = 8;

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

    /// Device shared by every window; keeps image uploads from being repeated.
    shared_gpu: Option<SharedGpu>,

    /// Last window that received input — the target for device events
    /// that do not carry a window identifier themselves.
    last_input: Option<WinitWindowId>,

    id_map: HashMap<WindowId, WinitWindowId>,
    error: Option<AppError>,
    shaders: Vec<(String, String)>,
    shader_watch: Option<ShaderWatch>,
    bench: Option<Rc<RefCell<Bench>>>,
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

    /// Re-reads watched shader files and hot-reloads anything that changed.
    fn poll_shader_watch(&mut self) {
        let changed = {
            let Some(watch) = self.shader_watch.as_mut() else {
                return;
            };
            let now = Instant::now();
            if now < watch.next_poll {
                return;
            }
            watch.next_poll = now + SHADER_WATCH_INTERVAL;
            watch.poll()
        };
        if changed.is_empty() {
            return;
        }
        for (name, wgsl) in &changed {
            if let Some(entry) = self.shaders.iter_mut().find(|(entry, _)| entry == name) {
                entry.1.clone_from(wgsl);
            } else {
                self.shaders.push((name.clone(), wgsl.clone()));
            }
        }
        for ws in self.live.values_mut() {
            let mut repaint = false;
            for (name, wgsl) in &changed {
                match ws.gpu.register_shader(name, wgsl) {
                    Ok(id) => {
                        ws.rt.register_shader_id_with(name.clone(), id);
                        repaint = true;
                        log::info!("sequoia-platform: hot-reloaded shader `{name}`");
                    }
                    Err(e) => {
                        log::error!("sequoia-platform: shader `{name}` hot reload failed: {e}");
                    }
                }
            }
            if repaint {
                ws.request_redraw();
            }
        }
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

        // Enabled per focus: platforms otherwise route every key through the
        // IME, which breaks plain-key accelerators in non-text widgets.
        window.set_ime_allowed(false);

        window.set_cursor_visible(s.cursor_visible);
        if s.cursor_grab
            && let Err(e) = grab_cursor(&window)
        {
            log::warn!("sequoia-platform: failed to confine cursor: {e}");
        }

        let size = window.inner_size();
        let mut options = GpuOptions::with_backend(self.backend)
            .present_mode(s.present_mode)
            .power_preference(s.power_preference);
        if let Some(format) = s.surface_format {
            options = options.surface_format(format);
        }
        let mut new_shared = None;
        let mut gpu = if let Some(shared) = self
            .shared_gpu
            .as_ref()
            .filter(|shared| shared.supports(self.backend))
        {
            match shared.renderer(window.clone(), size.width, size.height, options) {
                Ok(gpu) => gpu,
                Err(e) => {
                    log::warn!(
                        "sequoia-platform: window {id} cannot join the shared GPU ({e}) - using a private device"
                    );
                    GpuRenderer::new_with_options(window.clone(), size.width, size.height, options)
                        .map_err(|e| {
                            AppError::Gpu(format!("backend {} ({id}): {e}", self.backend.name()))
                        })?
                }
            }
        } else {
            match SharedGpu::new(window.clone(), options) {
                Ok(shared) => {
                    match shared.renderer(window.clone(), size.width, size.height, options) {
                        Ok(gpu) => {
                            new_shared = Some(shared);
                            gpu
                        }
                        Err(e) => {
                            log::warn!(
                                "sequoia-platform: shared GPU cannot render window {id} ({e}) - using a private device"
                            );
                            GpuRenderer::new_with_options(
                                window.clone(),
                                size.width,
                                size.height,
                                options,
                            )
                            .map_err(|e| {
                                AppError::Gpu(format!(
                                    "backend {} ({id}): {e}",
                                    self.backend.name()
                                ))
                            })?
                        }
                    }
                }
                Err(e) => {
                    log::info!(
                        "sequoia-platform: no shared GPU for window {id} ({e}) - using a private device"
                    );
                    GpuRenderer::new_with_options(window.clone(), size.width, size.height, options)
                        .map_err(|e| {
                            AppError::Gpu(format!("backend {} ({id}): {e}", self.backend.name()))
                        })?
                }
            }
        };
        if let Some(shared) = new_shared {
            self.shared_gpu = Some(shared);
        }
        gpu.set_clear_color(window_clear_color(self.theme, s.transparent));
        if let Some(budget) = s.memory_budget {
            gpu.set_memory_budget(budget);
        }
        for (name, wgsl) in &self.shaders {
            if let Err(e) = gpu.register_shader(name, wgsl) {
                log::error!(
                    "sequoia-platform: failed to register shader `{name}` on window {id}: {e}"
                );
            }
        }

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
        for (name, _) in &self.shaders {
            if let Some(shader_id) = gpu.shader_id(name) {
                rt.register_shader_id_with(name.clone(), shader_id);
            }
        }
        rt.set_window_id(id);
        rt.set_root_fill(true);
        rt.set_environment(self.env.clone());
        if let Some(root) = spec.root.clone() {
            rt.mount_any(root);
        }

        let gate = Rc::new(RefCell::new(PaintGate::default()));
        {
            let gate = gate.clone();
            rt.set_redraw_hook(move || gate.borrow_mut().request());
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
            wheel_seen: WheelDedupe::default(),
            scale,
            last_cursor: None,
            ime_allowed: false,
            settings: s.clone(),
            next_frame: initial_next_frame(continuous_paced, s.fps_limit),
            fps_base: spec.title.clone(),
            fps_frames: std::collections::VecDeque::new(),
            fps_last_push: None,
            continuous_paced,
            gate,
            list: RenderList::new(),
            bench: self.bench.clone(),
        };

        state.draw_frame(wid);
        state.gate.borrow_mut().clear_request();
        if show_after_first_frame {
            state.window.set_visible(true);
            state.rt.mark_full_damage();
        }
        state.request_redraw();
        self.live.insert(wid, state);
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
            // The last window is gone; drop the shared device explicitly so
            // its textures and buffers are released before the loop exits.
            self.shared_gpu = None;
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
        let mut deadline = None;
        if self.redraw_mode == RedrawMode::Continuous
            && self.live.values().any(|ws| ws.settings.fps_limit.is_some())
        {
            deadline = match (self.earliest_frame_deadline(), self.fps_refresh_deadline()) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
        } else if let Some(refresh) = self.fps_refresh_deadline() {
            deadline = Some(refresh);
        }
        if let Some(watch) = &self.shader_watch {
            deadline = Some(match deadline {
                Some(current) => current.min(watch.next_poll),
                None => watch.next_poll,
            });
        }
        match deadline {
            Some(_) => event_loop.set_control_flow(pacing_control_flow(deadline)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
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

/// Text carried by a key event that the key mapping cannot produce.
///
/// Windows resolves dead-key combinations to a `WM_CHAR` whose text differs
/// from the logical key (or has no logical character at all). Returning that
/// text lets the host deliver it as [`ImeEvent::Commit`] instead of dropping
/// the accent, while ordinary typing — where text matches the mapped
/// character — stays on the regular key path.
fn ime_commit_for_key(mapped: Option<char>, text: &str) -> Option<String> {
    if text.is_empty() || text.chars().any(|c| c.is_control()) {
        return None;
    }
    let mut chars = text.chars();
    if let (Some(mapped), Some(only), None) = (mapped, chars.next(), chars.next())
        && only == mapped
    {
        return None;
    }
    Some(text.to_owned())
}

#[cfg(test)]
mod ime_key_tests {
    use super::ime_commit_for_key;

    #[test]
    fn ordinary_typing_stays_on_the_key_path() {
        assert_eq!(ime_commit_for_key(Some('a'), "a"), None);
        assert_eq!(ime_commit_for_key(Some('A'), "A"), None);
        assert_eq!(ime_commit_for_key(Some('ł'), "ł"), None);
    }

    #[test]
    fn accent_text_becomes_a_commit() {
        assert_eq!(ime_commit_for_key(Some('e'), "é").as_deref(), Some("é"));
        assert_eq!(ime_commit_for_key(None, "´").as_deref(), Some("´"));
        assert_eq!(ime_commit_for_key(None, "に").as_deref(), Some("に"));
    }

    #[test]
    fn control_and_empty_text_is_ignored() {
        assert_eq!(ime_commit_for_key(Some('c'), "\u{3}"), None);
        assert_eq!(ime_commit_for_key(None, ""), None);
    }
}

/// Enables the platform IME only while a text input owns focus.
fn sync_ime_allowed(window: &Window, allowed: &mut bool, rt: &Runtime) {
    let want = rt.text_input_focused();
    if want != *allowed {
        window.set_ime_allowed(want);
        *allowed = want;
    }
}

/// Places the IME candidate window at the text caret.
fn sync_ime_cursor(window: &Window, scale: f32, rt: &Runtime) {
    let Some(rect) = rt.ime_cursor_rect() else {
        return;
    };
    let s = scale;
    let x = (rect.left() * s) as f64;
    let y = (rect.bottom() * s) as f64;
    let w = (rect.size.width * s).round().max(1.0) as u32;
    let h = (rect.size.height * s).round().max(1.0) as u32;
    window.set_ime_cursor_area(
        winit::dpi::PhysicalPosition::new(x, y),
        winit::dpi::PhysicalSize::new(w, h),
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
        if is_input_event(&event) {
            self.last_input = Some(wid);
        }

        let redraw_mode = self.redraw_mode;
        {
            let Some(ws) = self.live.get_mut(&wid) else {
                return;
            };
            if !ws.continuous_paced && is_input_event(&event) {
                ws.gate.borrow_mut().request();
            }
            match event {
                WindowEvent::Resized(size) => {
                    ws.gpu.resize(size.width, size.height);
                    ws.rt.mark_full_damage();
                    ws.request_redraw();
                }

                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    if ws.settings.scale_override.is_none() {
                        ws.scale = sanitize_scale(scale_factor);
                        ws.text.set_scale_factor(ws.scale);
                        ws.rt.mark_full_damage();
                        ws.request_redraw();
                    }
                }

                WindowEvent::Occluded(occluded) => {
                    if !occluded {
                        ws.rt.mark_full_damage();
                        ws.request_redraw();
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

                    ws.rt
                        .pointer_move_with_mods(ws.cursor, ws.mods, &mut ws.text);
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
                    let mapped = map_key(&kev.logical_key);
                    let commit = kev
                        .text
                        .as_deref()
                        .and_then(|text| ime_commit_for_key(mapped.and_then(Key::char), text));
                    if kev.state == ElementState::Pressed
                        && let Some(text) = commit
                    {
                        // The platform resolved this keystroke to text the key
                        // mapping cannot express (dead keys, accent
                        // composition). Treat it as a one-shot commit.
                        ws.rt.ime(ImeEvent::Commit(text), &mut ws.text);
                        ws.request_redraw();
                    } else if kev.state == ElementState::Pressed
                        && let Some(k) = mapped
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
                        let text_input = ws.rt.text_input_focused();
                        for acc in &self.accelerators {
                            if acc.key == k
                                && acc.modifiers == ws.mods
                                && acc.should_fire()
                                && accelerator_allowed(ws.mods, text_input)
                            {
                                acc.fire(&ctx);
                                hit = true;
                                break;
                            }
                        }
                        if hit {
                            ws.request_redraw();
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
                    let d = ws
                        .wheel_seen
                        .window(wheel_points(delta, ws.scale), Instant::now());
                    if d.x != 0.0 || d.y != 0.0 {
                        ws.rt.wheel(ws.cursor, d, ws.mods, &mut ws.text);
                    }
                }

                WindowEvent::RedrawRequested => {
                    if ws.gate.borrow_mut().on_redraw() {
                        ws.draw_frame(wid);
                    }
                    if redraw_mode == RedrawMode::Continuous && !ws.continuous_paced {
                        ws.request_redraw();
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
            // Focus may have moved in this event; keep the IME in sync before
            // the next keystroke arrives.
            sync_ime_allowed(&ws.window, &mut ws.ime_allowed, &ws.rt);
        }

        self.drain_requests(event_loop);
        self.update_control_flow(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.drain_requests(event_loop);
        self.poll_shader_watch();

        if let Some(mgr) = self.global_hotkeys.as_ref() {
            let fired = mgr.drain();
            if !fired.is_empty() {
                for id in fired {
                    if let Some(action) = self.global_actions.get(&id) {
                        action();
                    }
                }
                for ws in self.live.values() {
                    ws.gate.borrow_mut().request();
                }
            }
        }
        if self.live.is_empty() {
            return;
        }

        for ws in self.live.values_mut() {
            ws.paint_if_requested();
        }
        if let Some(bench) = &self.bench
            && bench.borrow().done
        {
            let mut bench = bench.borrow_mut();
            if !bench.reported {
                bench.reported = true;
                bench.print_report();
            }
            drop(bench);
            event_loop.exit();
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
                ws.request_redraw();
            }
        }
        self.update_control_flow(event_loop);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let DeviceEvent::MouseWheel { delta } = event else {
            return;
        };
        let Some(wid) = self.last_input else {
            return;
        };
        let now = Instant::now();
        let Some(ws) = self.live.get_mut(&wid) else {
            return;
        };
        // Raw input carries no cursor position - we use the last known one.
        let d = ws.wheel_seen.device(wheel_points(delta, ws.scale), now);
        if d.x == 0.0 && d.y == 0.0 {
            return;
        }
        ws.rt.wheel(ws.cursor, d, ws.mods, &mut ws.text);
        if !ws.continuous_paced {
            ws.gate.borrow_mut().request();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: JobWake) {
        if self.executor.drain() == 0 {
            return;
        }
        for ws in self.live.values() {
            ws.gate.borrow_mut().request();
        }
        self.update_control_flow(event_loop);
    }
}

fn is_input_event(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::CursorMoved { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::KeyboardInput { .. }
            | WindowEvent::Ime(_)
            | WindowEvent::Touch(_)
            | WindowEvent::PinchGesture { .. }
            | WindowEvent::PanGesture { .. }
            | WindowEvent::DroppedFile(_)
            | WindowEvent::HoveredFile(_)
            | WindowEvent::HoveredFileCancelled
            | WindowEvent::AxisMotion { .. }
    )
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

/// How often watched shader directories are rescanned while the app runs.
const SHADER_WATCH_INTERVAL: Duration = Duration::from_millis(200);

/// Polls one shader directory and returns files whose source changed.
struct ShaderWatch {
    dir: std::path::PathBuf,
    sources: HashMap<String, String>,
    next_poll: Instant,
}

impl ShaderWatch {
    fn new(dir: std::path::PathBuf) -> Self {
        let mut watch = Self {
            dir,
            sources: HashMap::new(),
            next_poll: Instant::now() + SHADER_WATCH_INTERVAL,
        };
        watch.scan(&mut Vec::new());
        watch
    }

    /// Returns `(name, wgsl)` for every new or edited file.
    fn poll(&mut self) -> Vec<(String, String)> {
        let mut changed = Vec::new();
        self.scan(&mut changed);
        changed
    }

    fn scan(&mut self, changed: &mut Vec<(String, String)>) {
        let mut paths = Vec::new();
        if collect_shader_paths(&self.dir, &self.dir, &mut paths).is_err() {
            return;
        }
        for (name, path) in paths {
            let Ok(wgsl) = std::fs::read_to_string(&path) else {
                continue;
            };
            if self.sources.get(&name) == Some(&wgsl) {
                continue;
            }
            self.sources.insert(name.clone(), wgsl.clone());
            changed.push((name, wgsl));
        }
    }
}

fn collect_shader_paths(
    root: &std::path::Path,
    current: &std::path::Path,
    out: &mut Vec<(String, std::path::PathBuf)>,
) -> std::io::Result<()> {
    if !current.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_shader_paths(root, &path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("wgsl")
            && let Ok(rel) = path.strip_prefix(root)
        {
            let name = rel.with_extension("").to_string_lossy().replace('\\', "/");
            out.push((name, path));
        }
    }
    Ok(())
}

fn collect_shaders_dir(
    root: &std::path::Path,
    current: &std::path::Path,
    out: &mut Vec<(String, String)>,
) -> std::io::Result<()> {
    let mut paths = Vec::new();
    collect_shader_paths(root, current, &mut paths)?;
    for (name, path) in paths {
        if let Ok(wgsl) = std::fs::read_to_string(path) {
            out.push((name, wgsl));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_view::Unit;

    #[test]
    fn shader_registration_dedupes_and_accepts_a_registry() {
        let app = App::new()
            .shader("custom", "first")
            .shader("custom", "second")
            .shader_registry(sequoia_shader::ShaderRegistry::new().with_raw("custom", "third"));

        let matches = app
            .shaders()
            .iter()
            .filter(|(name, _)| name == "custom")
            .count();
        assert_eq!(matches, 1);
        assert_eq!(
            app.shader_names().filter(|name| *name == "custom").count(),
            1
        );
        assert!(app.has_shader("custom"));
        assert_eq!(
            app.shaders()
                .iter()
                .find(|(name, _)| name == "custom")
                .map(|(_, wgsl)| wgsl.as_str()),
            Some("third")
        );
    }

    #[test]
    fn shader_file_and_directory_register_wgsl_sources() {
        let root = std::env::temp_dir().join(format!(
            "sequoia-shader-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("effects")).expect("create shader dir");
        std::fs::write(root.join("root.wgsl"), "@vertex fn vs_main() {}").expect("write root");
        std::fs::write(
            root.join("effects/invert.wgsl"),
            "@fragment fn fs_main() {}",
        )
        .expect("write nested");
        std::fs::write(root.join("effects/notes.txt"), "ignored").expect("write ignored");

        let file = root.join("effects/invert.wgsl");
        let app = App::new()
            .shader("inline", "@vertex fn vs_main() {}")
            .shader_file("from_file", &file)
            .shader_dir(&root);

        let shaders = app.shaders();
        assert!(
            shaders
                .iter()
                .any(|(name, wgsl)| name == "inline" && wgsl.contains("vs_main"))
        );
        assert!(
            shaders
                .iter()
                .any(|(name, wgsl)| name == "from_file" && wgsl.contains("fs_main"))
        );
        assert!(
            shaders
                .iter()
                .any(|(name, wgsl)| name == "root" && wgsl.contains("vs_main"))
        );
        assert!(
            shaders
                .iter()
                .any(|(name, wgsl)| name == "effects/invert" && wgsl.contains("fs_main"))
        );
        assert!(!shaders.iter().any(|(name, _)| name.ends_with("notes")));

        let _ = std::fs::remove_dir_all(&root);
    }

    fn temp_shader_dir(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "sequoia-shader-watch-{}-{:?}-{tag}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn shader_watch_reports_only_new_or_edited_files() {
        let root = temp_shader_dir("changes");
        std::fs::create_dir_all(root.join("effects")).expect("create shader dir");
        std::fs::write(root.join("base.wgsl"), "fn a() {}").expect("write base");

        let mut watch = ShaderWatch::new(root.clone());
        assert!(watch.poll().is_empty(), "unchanged files are not reported");

        std::fs::write(root.join("effects/new.wgsl"), "fn b() {}").expect("write new");
        let changed = watch.poll();
        assert_eq!(changed.len(), 1, "only the new file changes");
        assert_eq!(changed[0].0, "effects/new");
        assert_eq!(changed[0].1, "fn b() {}");

        std::fs::write(root.join("base.wgsl"), "fn a() { changed }").expect("edit base");
        let changed = watch.poll();
        assert_eq!(
            changed,
            vec![("base".to_string(), "fn a() { changed }".to_string())]
        );
        assert!(watch.poll().is_empty(), "reported edits are not repeated");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn watch_shader_dir_loads_initial_files_and_keeps_the_directory() {
        let root = temp_shader_dir("builder");
        std::fs::create_dir_all(&root).expect("create shader dir");
        std::fs::write(root.join("effect.wgsl"), "fn effect() {}").expect("write effect");

        let app = App::new().watch_shader_dir(&root);
        assert!(app.has_shader("effect"), "initial files are registered");
        assert_eq!(app.shader_watch.as_deref(), Some(root.as_path()));

        let app = App::new().watch_shaders();
        assert_eq!(
            app.shader_watch.as_deref(),
            Some(std::path::Path::new("assets/shaders")),
            "the default path is assets/shaders"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn editing_text_suppresses_plain_key_accelerators() {
        assert!(accelerator_allowed(Modifiers::NONE, false));
        assert!(!accelerator_allowed(Modifiers::NONE, true));
        assert!(!accelerator_allowed(Modifiers::SHIFT, true));
        assert!(accelerator_allowed(Modifiers::CONTROL, true));
        assert!(accelerator_allowed(Modifiers::ALT, true));
        assert!(accelerator_allowed(Modifiers::SUPER, true));
    }

    #[test]
    fn the_same_wheel_turn_is_counted_once() {
        let t0 = Instant::now();
        let d = sequoia_core::Vec2::new(-40.0, -40.0);
        let after = Duration::from_millis(1);

        let mut device_first = WheelDedupe::default();
        let a = device_first.device(d, t0);
        let b = device_first.window(d, t0 + after);
        assert_eq!((a.x, a.y), (-40.0, -40.0), "pierwsza droga liczy całość");
        assert_eq!((b.x, b.y), (0.0, 0.0), "kopia z okna odrzucona");

        let mut window_first = WheelDedupe::default();
        let a = window_first.window(d, t0);
        let b = window_first.device(d, t0 + after);
        assert_eq!((a.x, a.y), (-40.0, -40.0));
        assert_eq!((b.x, b.y), (0.0, 0.0), "kopia z raw input odrzucona");
    }

    #[test]
    fn raw_input_fills_an_axis_the_window_never_reports() {
        let t0 = Instant::now();
        let mut dedupe = WheelDedupe::default();
        let vertical = dedupe.window(sequoia_core::Vec2::new(0.0, -40.0), t0);
        let horizontal = dedupe.device(
            sequoia_core::Vec2::new(-40.0, 0.0),
            t0 + Duration::from_millis(2),
        );
        assert_eq!(vertical.y, -40.0);
        assert_eq!(horizontal.x, -40.0, "poziom tylko z raw input ma przejść");
        assert_eq!(horizontal.y, 0.0, "pion policzyło już okno");
    }

    #[test]
    fn repeated_window_wheel_steps_are_not_throttled() {
        let t0 = Instant::now();
        let mut dedupe = WheelDedupe::default();
        let a = dedupe.window(sequoia_core::Vec2::new(0.0, -40.0), t0);
        let b = dedupe.window(
            sequoia_core::Vec2::new(0.0, -40.0),
            t0 + Duration::from_millis(8),
        );
        assert_eq!(
            (a.y, b.y),
            (-40.0, -40.0),
            "kolejne kroki tej samej drogi nie giną"
        );
    }

    #[test]
    fn interleaved_wheel_paths_do_not_starve_each_other() {
        let t0 = Instant::now();
        let mut dedupe = WheelDedupe::default();
        let d = sequoia_core::Vec2::new(-40.0, 0.0);
        let mut applied = 0;
        for step in 0..20u64 {
            let at = t0 + Duration::from_millis(step * 8);
            let window = dedupe.window(d, at);
            let device = dedupe.device(d, at + Duration::from_millis(4));
            applied += u32::from(window.x != 0.0) + u32::from(device.x != 0.0);
        }
        assert!(
            applied >= 10,
            "jedna ścieżka musi pozostać aktywna, a dotarło {applied} kroków"
        );
    }

    #[test]
    fn quiet_window_hands_the_axis_to_raw_input() {
        let t0 = Instant::now();
        let mut dedupe = WheelDedupe::default();
        let d = sequoia_core::Vec2::new(-40.0, 0.0);
        assert_eq!(dedupe.window(d, t0).x, -40.0);
        assert_eq!(
            dedupe.device(d, t0 + Duration::from_millis(10)).x,
            0.0,
            "kopia tuż po zdarzeniu okna jest odrzucana"
        );
        assert_eq!(
            dedupe.device(d, t0 + Duration::from_millis(80)).x,
            -40.0,
            "po ciszy raw input przejmuje oś"
        );
    }

    #[test]
    fn wheel_points_matches_line_and_pixel_units() {
        let line = wheel_points(winit::event::MouseScrollDelta::LineDelta(1.0, -1.0), 1.0);
        assert_eq!((line.x, line.y), (40.0, -40.0));
        let pixel = wheel_points(
            winit::event::MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(
                20.0, -10.0,
            )),
            2.0,
        );
        assert_eq!((pixel.x, pixel.y), (10.0, -5.0));
    }

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
    fn accelerators_are_suppressed_while_typing() {
        use sequoia_input::Modifiers;
        assert!(accelerator_allowed(Modifiers::NONE, false));
        assert!(
            !accelerator_allowed(Modifiers::NONE, true),
            "plain Delete must reach the focused text field"
        );
        assert!(accelerator_allowed(Modifiers::CONTROL, true));
        assert!(accelerator_allowed(Modifiers::SUPER, true));
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
    fn surface_format_reaches_settings() {
        let app = App::new().surface_format(sequoia_render::wgpu::TextureFormat::Bgra8Unorm);
        assert_eq!(
            app.windows[0].settings.surface_format,
            Some(sequoia_render::wgpu::TextureFormat::Bgra8Unorm)
        );
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
        assert!(s.surface_format.is_none());
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

    #[test]
    fn paint_from_input_supersedes_the_queued_redraw() {
        let mut gate = PaintGate::default();
        gate.request();
        gate.schedule();
        assert!(
            gate.begin_paint(),
            "input arrived while a redraw was queued"
        );
        assert!(!gate.on_redraw(), "queued redraw would duplicate the frame");
    }

    #[test]
    fn paint_without_new_input_does_not_schedule_again() {
        let mut gate = PaintGate::default();
        gate.request();
        assert!(!gate.begin_paint());
        assert!(!gate.end_paint(false));
        assert!(!gate.redraw_pending && !gate.skip_next_redraw);
    }

    #[test]
    fn input_during_paint_schedules_the_next_frame() {
        let mut gate = PaintGate::default();
        gate.request();
        assert!(!gate.begin_paint());
        gate.request();
        assert!(gate.end_paint(false));
        assert!(gate.redraw_pending);
        assert!(gate.on_redraw());
    }

    #[test]
    fn input_during_paint_requests_another_frame_instead_of_skipping() {
        let mut gate = PaintGate::default();
        gate.request();
        gate.schedule();
        assert!(gate.begin_paint());
        gate.request();
        assert!(!gate.end_paint(true), "superseded paint stays coalesced");
        assert!(!gate.skip_next_redraw);
        assert!(gate.on_redraw());
    }

    #[test]
    fn input_after_a_skipped_redraw_is_not_swallowed() {
        let mut gate = PaintGate::default();
        gate.request();
        gate.schedule();
        assert!(gate.begin_paint());
        assert!(!gate.end_paint(true));
        gate.request();
        assert!(gate.on_redraw(), "fresh input must force a paint");
    }
}
