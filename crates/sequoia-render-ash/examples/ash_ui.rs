//! Experimental `ash` Vulkan renderer demo.
//!
//! Run with:
//!     cargo run -p sequoia-render-ash --example ash_ui

use std::sync::Arc;

use sequoia_core::prelude::*;
use sequoia_render_ash::{AshError, AshOptions, AshRenderer};
use sequoia_text::TextSystem;
use sequoia_view::prelude::*;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = Demo::default();
    event_loop.run_app(&mut app).expect("run event loop");
}

#[derive(Default)]
struct Demo {
    renderer: Option<AshRenderer>,
    window: Option<Arc<Window>>,
    runtime: Runtime,
    text: TextSystem,
    list: RenderList,
}

impl ApplicationHandler for Demo {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Sequoia — ash Vulkan backend")
                        .with_inner_size(LogicalSize::new(1040.0, 680.0)),
                )
                .expect("create window"),
        );
        let size = window.inner_size();
        match AshRenderer::with_options(
            window.clone(),
            size.width,
            size.height,
            AshOptions::default(),
        ) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => {
                report(&error);
                event_loop.exit();
                return;
            }
        }

        self.text.set_scale_factor(window.scale_factor() as f32);
        self.runtime
            .mount(Canvas::new().on_draw(paint_demo).fill_size());
        self.window = Some(window);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer
                    && let Err(error) = renderer.resize(size.width, size.height)
                {
                    report(&error);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    self.text.set_scale_factor(window.scale_factor() as f32);
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }
}

impl Demo {
    fn redraw(&mut self) {
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let scale = window.scale_factor() as f32;
        let bounds = Rect::from_xywh(
            0.0,
            0.0,
            size.width as f32 / scale,
            size.height as f32 / scale,
        );
        self.runtime.layout(bounds, &mut self.text);
        self.list.clear();
        self.runtime.paint(&mut self.list, &mut self.text);
        if let Err(error) = renderer.render(&self.list, self.text.atlas_mut(), scale) {
            report(&error);
        }
    }
}

fn report(error: &AshError) {
    eprintln!("sequoia-render-ash: {error}");
}

fn paint_demo(g: &mut Painter) {
    let rect = g.rect();
    g.fill(
        rect,
        Fill::linear(
            GradientStops::two(
                Color::rgba(0.055, 0.063, 0.082, 1.0),
                Color::rgba(0.105, 0.089, 0.157, 1.0),
            ),
            35.0,
        ),
    );

    let width = rect.size.width;
    let left = 56.0;
    let mut top = 56.0;

    let card = Rect::from_xywh(left, top, (width - 112.0).max(240.0), 250.0);
    g.shadow(
        card,
        22.0,
        36.0,
        0.0,
        Vec2::new(0.0, 16.0),
        Color::rgba(0.0, 0.0, 0.0, 0.42),
    );
    g.rounded(
        card,
        22.0,
        Fill::solid(Color::rgba(0.094, 0.102, 0.133, 0.94)),
    );

    g.text_tl(
        "Sequoia",
        Point::new(left + 32.0, top + 26.0),
        34.0,
        Color::rgba(0.95, 0.96, 1.0, 1.0),
    );
    g.text_tl(
        "experimental ash / Vulkan backend",
        Point::new(left + 34.0, top + 76.0),
        17.0,
        Color::rgba(0.62, 0.66, 0.78, 1.0),
    );

    let chip = Rect::from_xywh(left + 32.0, top + 118.0, 156.0, 34.0);
    g.rounded(
        chip,
        17.0,
        Fill::linear(
            GradientStops::two(
                Color::rgba(0.20, 0.52, 0.92, 1.0),
                Color::rgba(0.40, 0.26, 0.86, 1.0),
            ),
            0.0,
        ),
    );
    g.text_tl(
        "quads · text · images",
        Point::new(chip.left() + 14.0, chip.top() + 8.0),
        14.0,
        Color::rgba(1.0, 1.0, 1.0, 1.0),
    );

    top += 286.0;

    let image_size = 160.0;
    let image_rect = Rect::from_xywh(left, top, image_size, image_size);
    g.shadow(
        image_rect,
        18.0,
        24.0,
        0.0,
        Vec2::new(0.0, 8.0),
        Color::rgba(0.0, 0.0, 0.0, 0.38),
    );
    g.clipped_rounded(image_rect, 18.0, |g| {
        g.image(
            image_rect,
            [0.0, 0.0, 1.0, 1.0],
            ImageSource::Rgba {
                id: 1,
                width: 64,
                height: 64,
                data: checkerboard(),
            },
        );
    });

    let gradient_rect = Rect::from_xywh(left + image_size + 24.0, top, 260.0, image_size);
    g.clipped_rounded(gradient_rect, 18.0, |g| {
        g.fill(
            gradient_rect,
            Fill::radial(GradientStops::two(
                Color::rgba(0.99, 0.72, 0.28, 1.0),
                Color::rgba(0.72, 0.13, 0.28, 1.0),
            )),
        );
    });
    g.text_tl(
        "radial gradient",
        Point::new(gradient_rect.left() + 18.0, gradient_rect.top() + 18.0),
        16.0,
        Color::rgba(1.0, 1.0, 1.0, 0.92),
    );

    let text_left = gradient_rect.right() + 28.0;
    if text_left + 220.0 < width {
        g.text_tl(
            "Raw Vulkan, no wgpu",
            Point::new(text_left, top + 10.0),
            21.0,
            Color::rgba(0.92, 0.94, 1.0, 1.0),
        );
        g.text_tl(
            "One SPIR-V module drives",
            Point::new(text_left, top + 48.0),
            15.0,
            Color::rgba(0.62, 0.66, 0.78, 1.0),
        );
        g.text_tl(
            "all four pipelines:",
            Point::new(text_left, top + 70.0),
            15.0,
            Color::rgba(0.62, 0.66, 0.78, 1.0),
        );
        g.text_tl(
            "quads, glyphs, blits,",
            Point::new(text_left, top + 100.0),
            15.0,
            Color::rgba(0.42, 0.72, 0.98, 1.0),
        );
        g.text_tl(
            "gradients.",
            Point::new(text_left, top + 122.0),
            15.0,
            Color::rgba(0.42, 0.72, 0.98, 1.0),
        );
    }
}

fn checkerboard() -> Arc<Vec<u8>> {
    const SIZE: u32 = 64;
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let light = ((x / 8) + (y / 8)) % 2 == 0;
            let (r, g, b) = if light {
                (64u8, 190u8, 236u8)
            } else {
                (232u8, 102u8, 70u8)
            };
            let index = ((y * SIZE + x) * 4) as usize;
            pixels[index] = r;
            pixels[index + 1] = g;
            pixels[index + 2] = b;
            pixels[index + 3] = 255;
        }
    }
    Arc::new(pixels)
}
