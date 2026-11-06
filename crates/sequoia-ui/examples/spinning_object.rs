//! A rotating object rendered by a `WgpuSurface` without any input events.
//!
//! Run with:
//!     cargo run --example spinning_object -p sequoia-ui
//!
//! The renderer calls `WgpuFrame::request_frame` on every draw. The platform
//! forwards that request into the runtime, so `paint_if_dirty` keeps painting
//! instead of skipping once the first frame is on screen.

use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
use sequoia_ui::prelude::*;
use sequoia_ui::wgpu;

const PANEL_BG: Color = Color::hex(0x090B10);
const PANEL_BORDER: Color = Color::hex(0x232A33);
const TEXT: Color = Color::hex(0xF3F3F5);
const MUTED: Color = Color::hex(0x93939F);

const SHADER: &str = r#"
struct Uniforms {
    // x: rotation angle (radians), y: surface aspect ratio (width / height)
    data: vec4<f32>,
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
    let center = vec2<f32>(0.5, 0.5);

    // Centered coordinates, corrected so the object stays square.
    var p = in.uv - center;
    p.x = p.x * u.data.y;

    let c = cos(u.data.x);
    let s = sin(u.data.x);
    let r = vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);

    // Rounded rectangle (signed distance field) that rotates with r.
    let extent = vec2<f32>(0.24, 0.15);
    let corner = 0.04;
    let q = abs(r) - extent + vec2<f32>(corner);
    let dist = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - corner;

    let aa = 0.003;
    let fill = 1.0 - smoothstep(-aa, aa, dist);

    let vignette = 1.0 - 1.4 * length(in.uv - center);
    var color = mix(
        vec3<f32>(0.015, 0.020, 0.035),
        vec3<f32>(0.050, 0.070, 0.120),
        clamp(vignette, 0.0, 1.0),
    );

    // A faint orbit ring behind the object.
    let d = length(p);
    let ring = smoothstep(0.300, 0.306, d) * (1.0 - smoothstep(0.306, 0.312, d));
    color = color + vec3<f32>(0.08, 0.13, 0.22) * ring;

    let gradient = clamp(0.5 + r.x * 2.5, 0.0, 1.0);
    var face = mix(vec3<f32>(0.16, 0.52, 0.98), vec3<f32>(0.98, 0.32, 0.52), gradient);

    // Bright dot on the local +x axis makes the rotation unmistakable.
    let marker = 1.0 - smoothstep(0.0, 0.022, length(r - vec2<f32>(0.165, 0.0)));
    face = face + vec3<f32>(0.9, 0.85, 0.7) * marker;

    let edge = 1.0 - smoothstep(0.0, 0.020, abs(dist));
    let result = mix(color, face, fill) + vec3<f32>(0.35) * edge * fill;
    return vec4<f32>(result, 1.0);
}
"#;

struct Rotor {
    pipeline: Option<wgpu::RenderPipeline>,
    uniform: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    angle: f32,
}

impl Rotor {
    fn new() -> Self {
        Self {
            pipeline: None,
            uniform: None,
            bind_group: None,
            angle: 0.0,
        }
    }

    fn ensure_gpu(&mut self, frame: &WgpuFrame<'_>) {
        if self.pipeline.is_some() {
            return;
        }

        let layout = frame
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rotor-bgl"),
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
            label: Some("rotor-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rotor-bind-group"),
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
                    label: Some("rotor-pipeline-layout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });

        let module = frame
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("rotor-shader"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });

        let pipeline = frame
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("rotor-pipeline"),
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

impl WgpuRenderer for Rotor {
    fn render(&mut self, frame: &mut WgpuFrame) {
        self.angle = (self.angle + frame.delta.as_secs_f32() * 1.1) % std::f32::consts::TAU;

        // Ask the host for the next frame; without it the window returns to
        // idle after the first paint.
        frame.request_frame();

        self.ensure_gpu(frame);

        let aspect = if frame.size.1 == 0 {
            1.0
        } else {
            frame.size.0 as f32 / frame.size.1 as f32
        };
        let uniform = self.uniform.as_ref().expect("uniform buffer");
        frame
            .queue
            .write_buffer(uniform, 0, &uniform_bytes(self.angle, aspect));

        let pipeline = self.pipeline.as_ref().expect("pipeline");
        let bind_group = self.bind_group.as_ref().expect("bind group");
        let region = frame.region;

        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rotor-pass"),
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

fn uniform_bytes(angle: f32, aspect: f32) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[..4].copy_from_slice(&angle.to_le_bytes());
    bytes[4..8].copy_from_slice(&aspect.to_le_bytes());
    bytes
}

fn main() {
    let surface = WgpuSurface::new(Rotor::new())
        .background(PANEL_BG)
        .border(1.0, PANEL_BORDER)
        .radius(12.0)
        .expand();

    let header = Row::new()
        .fill_width()
        .align_items(Align::Center)
        .gap(12.0)
        .child(Text::new("Spinning object").size(20.0).bold().color(TEXT))
        .child(Spacer::flex(1.0))
        .child(
            Text::new("request_frame() keeps the window painting")
                .size(12.0)
                .color(MUTED),
        );

    let view = Column::new()
        .fill_size()
        .padding(16.0)
        .gap(12.0)
        .child(header)
        .child(surface);

    App::new()
        .title("Spinning object")
        .size(880.0, 620.0)
        .min_size(520.0, 380.0)
        .root(view)
        .run()
        .expect("event loop failed");
}
