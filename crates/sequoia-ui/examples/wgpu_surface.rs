//! A `WgpuSurface` embedded in a normal Sequoia layout.
//!
//! Run with:
//!     cargo run --example wgpu_surface -p sequoia-ui
//!
//! The window is a plain `Column`/`Row` layout. The viewport in the middle is
//! a leaf that owns a small `wgpu` pipeline. The side panel mutates the
//! renderer through `WgpuSurface::renderer()`.

use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
use sequoia_ui::prelude::*;
use sequoia_ui::wgpu;

const BG: Color = Color::hex(0x0E0E12);
const PANEL_BG: Color = Color::hex(0x141419);
const PANEL_BORDER: Color = Color::hex(0x2A2A34);
const TEXT: Color = Color::hex(0xF3F3F5);
const MUTED: Color = Color::hex(0x93939F);

const SHADER: &str = r#"
struct Uniforms {
    time: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VertexOut;
    out.clip = vec4<f32>(positions[index], 0.0, 1.0);
    out.uv = uvs[index];
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let t = u.time.x;
    let p = in.uv * 7.0;
    let grid = abs(fract(p + vec2<f32>(t * 0.10, t * 0.06)) - 0.5);
    let line = smoothstep(0.46, 0.50, max(grid.x, grid.y));
    let wave = 0.5 + 0.5 * sin(p.x * 1.3 + p.y * 0.8 + t * 1.5);
    let base = mix(vec3<f32>(0.04, 0.05, 0.08), vec3<f32>(0.10, 0.24, 0.40), wave);
    let glow = 0.25 * (1.0 - length(in.uv - vec2<f32>(0.5)));
    let accent = vec3<f32>(0.25, 0.40, 0.55) * line;
    return vec4<f32>(base + accent + glow, 1.0);
}
"#;

struct Field {
    pipeline: Option<wgpu::RenderPipeline>,
    uniform: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    time: f32,
    animating: bool,
}

impl Field {
    fn new() -> Self {
        Self {
            pipeline: None,
            uniform: None,
            bind_group: None,
            time: 0.0,
            animating: true,
        }
    }

    fn ensure_gpu(&mut self, frame: &WgpuFrame<'_>) {
        if self.pipeline.is_some() {
            return;
        }

        let layout = frame
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("field-bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                }],
            });

        let uniform = frame.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("field-time"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("field-bind-group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform,
                    offset: 0,
                    size: wgpu::BufferSize::new(16),
                }),
            }],
        });

        let pipeline_layout =
            frame
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("field-pipeline-layout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });

        let module = frame
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("field-shader"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });

        let pipeline = frame
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("field-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: frame.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        self.pipeline = Some(pipeline);
        self.uniform = Some(uniform);
        self.bind_group = Some(bind_group);
    }
}

impl WgpuRenderer for Field {
    fn render(&mut self, frame: &mut WgpuFrame) {
        if self.animating {
            self.time += frame.delta.as_secs_f32();
            frame.request_frame();
        }

        self.ensure_gpu(frame);

        let uniform = self.uniform.as_ref().expect("uniform buffer");
        frame.queue.write_buffer(uniform, 0, &time_bytes(self.time));

        let pipeline = self.pipeline.as_ref().expect("pipeline");
        let bind_group = self.bind_group.as_ref().expect("bind group");
        let region = frame.region;

        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("field-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // The host has already painted the rest of the UI.
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

        pass.set_viewport(
            region.x as f32,
            region.y as f32,
            region.width as f32,
            region.height as f32,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(region.x, region.y, region.width, region.height);
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn time_bytes(time: f32) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[..4].copy_from_slice(&time.to_le_bytes());
    bytes
}

fn main() {
    let viewport = WgpuSurface::new(Field::new())
        .background(Color::hex(0x090A0D))
        .border(1.0, PANEL_BORDER)
        .radius(10.0)
        .expand();

    let renderer = viewport.renderer();

    let pause = renderer.clone();
    let reset = renderer;

    let sidebar = Column::new()
        .width(220.0)
        .padding(14.0)
        .gap(10.0)
        .background(PANEL_BG)
        .radius(10.0)
        .child(Text::new("Surface controls").size(14.0).bold().color(TEXT))
        .child(
            Text::new("The panel mutates the renderer through surface.renderer().")
                .size(12.0)
                .color(MUTED),
        )
        .child(Button::new("Pause / play").width(180.0).on_click(move || {
            let mut field = pause.borrow_mut();
            field.animating = !field.animating;
        }))
        .child(Button::new("Reset time").width(180.0).on_click(move || {
            reset.borrow_mut().time = 0.0;
        }));

    let header = Row::new()
        .fill_width()
        .align_items(Align::Center)
        .gap(12.0)
        .child(Text::new("WgpuSurface").size(22.0).bold().color(TEXT))
        .child(Spacer::flex(1.0))
        .child(Text::new("a raw wgpu pipeline inside the layout").color(MUTED));

    let body = Row::new().expand().gap(12.0).child(sidebar).child(viewport);

    let view = Column::new()
        .fill_size()
        .background(BG)
        .padding(16.0)
        .gap(12.0)
        .child(header)
        .child(body);

    App::new()
        .title("WgpuSurface")
        .size(960.0, 600.0)
        .min_size(720.0, 420.0)
        .root(view)
        .run()
        .expect("event loop failed");
}
