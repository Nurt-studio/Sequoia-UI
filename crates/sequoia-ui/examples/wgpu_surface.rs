//! A minimal `WgpuSurface`: a fullscreen triangle drawn by a raw `wgpu`
//! pipeline inside an ordinary layout.
//!
//! Run it:
//!     cargo run --example wgpu_surface -p sequoia-ui

use sequoia_ui::platform::{WgpuFrame, WgpuRenderer, WgpuSurface};
use sequoia_ui::prelude::*;
use sequoia_ui::wgpu;

const BG: Color = Color::hex(0x0E1015);
const PANEL: Color = Color::hex(0x171A21);
const TEXT: Color = Color::hex(0xF2F4F8);
const MUTED: Color = Color::hex(0x8A93A6);

const SHADER: &str = r#"
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
    let glow = 1.0 - length(in.uv - vec2<f32>(0.5));
    return vec4<f32>(0.10 + 0.60 * glow, 0.35, 0.90 - 0.40 * glow, 1.0);
}
"#;

struct Gradient {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Gradient {
    fn pipeline(&mut self, frame: &WgpuFrame<'_>) -> &wgpu::RenderPipeline {
        self.pipeline.get_or_insert_with(|| {
            let module = frame
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("surface-gradient"),
                    source: wgpu::ShaderSource::Wgsl(SHADER.into()),
                });
            frame
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("surface-gradient"),
                    layout: None,
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
                })
        })
    }
}

impl WgpuRenderer for Gradient {
    fn render(&mut self, frame: &mut WgpuFrame) {
        let region = frame.region;
        let pipeline = self.pipeline(frame);

        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("surface-gradient"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // The host has already painted the UI underneath.
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
        pass.draw(0..3, 0..1);
    }
}

fn main() {
    let viewport = WgpuSurface::new(Gradient { pipeline: None })
        .background(PANEL)
        .border(1.0, Color::hex(0x30363D))
        .radius(12.0)
        .expand();

    let view = Column::new()
        .fill_size()
        .background(BG)
        .padding(20.0)
        .gap(12.0)
        .child(
            Text::new("WgpuSurface")
                .size(22.0)
                .bold()
                .color(TEXT),
        )
        .child(
            Text::new("A raw wgpu pipeline inside the normal layout.")
                .size(13.0)
                .color(MUTED),
        )
        .child(viewport);

    App::new()
        .title("Embedded GPU surface")
        .size(760.0, 520.0)
        .root(view)
        .run()
        .expect("event loop failed");
}
