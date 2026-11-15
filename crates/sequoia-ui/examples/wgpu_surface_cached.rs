//! A cached `WgpuSurface`: the host keeps an offscreen texture and only calls
//! `render` again when `is_dirty` reports a change. Buttons update the tint
//! through `surface.renderer()`.
//!
//! Run it:
//!     cargo run --example wgpu_surface_cached -p sequoia-ui

use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
use sequoia_ui::prelude::*;
use sequoia_ui::wgpu;

const BG: Color = Color::hex(0x0E1015);
const PANEL: Color = Color::hex(0x171A21);
const TEXT: Color = Color::hex(0xF2F4F8);
const MUTED: Color = Color::hex(0x8A93A6);

const SHADER: &str = r#"
struct Tint {
    color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> tint: Tint;

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
    let shade = 0.35 + 0.65 * (1.0 - length(in.uv - vec2<f32>(0.5)));
    return vec4<f32>(tint.color.rgb * shade, 1.0);
}
"#;

struct TintScene {
    pipeline: Option<wgpu::RenderPipeline>,
    uniform: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    color: [f32; 3],
    dirty: bool,
}

impl TintScene {
    fn new(color: [f32; 3]) -> Self {
        Self {
            pipeline: None,
            uniform: None,
            bind_group: None,
            color,
            dirty: true,
        }
    }

    fn set_color(&mut self, color: [f32; 3]) {
        self.color = color;
        self.dirty = true;
    }

    fn ensure_gpu(&mut self, frame: &WgpuFrame<'_>) {
        if self.pipeline.is_some() {
            return;
        }

        let layout = frame
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("tint-bgl"),
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
            label: Some("tint-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tint-bind-group"),
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
                    label: Some("tint-pipeline-layout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });

        let module = frame
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tint-shader"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });

        let pipeline = frame
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("tint-pipeline"),
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

impl WgpuRenderer for TintScene {
    fn render(&mut self, frame: &mut WgpuFrame) {
        self.ensure_gpu(frame);

        let mut bytes = [0u8; 16];
        for (lane, value) in self.color.iter().chain([&1.0]).enumerate() {
            let start = lane * 4;
            bytes[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
        let uniform = self.uniform.as_ref().expect("uniform");
        frame.queue.write_buffer(uniform, 0, &bytes);

        let region = frame.region;
        let pipeline = self.pipeline.as_ref().expect("pipeline");
        let bind_group = self.bind_group.as_ref().expect("bind group");
        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tint-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Cached mode clears the offscreen texture for us.
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

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

fn main() {
    let surface = WgpuSurface::new(TintScene::new([0.93, 0.55, 0.30]))
        .cached(true)
        .background(PANEL)
        .border(1.0, Color::hex(0x30363D))
        .radius(12.0)
        .expand();

    let scene = surface.renderer();
    let warm = scene.clone();
    let cool = scene.clone();
    let green = scene;

    let buttons = Row::new()
        .gap(10.0)
        .child(Button::new("Amber").on_click(move || {
            warm.borrow_mut().set_color([0.93, 0.55, 0.30]);
        }))
        .child(Button::new("Ocean").on_click(move || {
            cool.borrow_mut().set_color([0.20, 0.45, 0.85]);
        }))
        .child(Button::new("Forest").on_click(move || {
            green.borrow_mut().set_color([0.25, 0.70, 0.45]);
        }));

    let view = Column::new()
        .fill_size()
        .background(BG)
        .padding(20.0)
        .gap(12.0)
        .child(
            Text::new("Cached WgpuSurface")
                .size(22.0)
                .bold()
                .color(TEXT),
        )
        .child(
            Text::new("The offscreen texture is redrawn only when is_dirty is true.")
                .size(13.0)
                .color(MUTED),
        )
        .child(buttons)
        .child(surface);

    App::new()
        .title("Cached GPU surface")
        .size(760.0, 520.0)
        .root(view)
        .run()
        .expect("event loop failed");
}
