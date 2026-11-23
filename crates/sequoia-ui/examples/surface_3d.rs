//! The same 3D surface rendered by both backends.
//!
//! The scene is a raymarched sphere on a plane with soft lighting. The WGSL
//! source is shared: wgpu consumes it directly, the ash backend compiles it to
//! SPIR-V with naga. Choose the backend with:
//!
//!     SEQUOIA_RENDER_BACKEND=wgpu:vulkan cargo run -p sequoia-ui --example surface_3d
//!     SEQUOIA_RENDER_BACKEND=ash          cargo run -p sequoia-ui --example surface_3d
//!
//! Set `SEQUOIA_BENCH_FRAMES=600` to run the built-in frame benchmark.
//!
//! The label at the bottom is painted *after* the surface and overlaps it.
//! Before the Faza 2 fix the ash backend drew direct surfaces after the whole
//! UI tree, so the surface overwrote that label; the wgpu backend composited
//! it in paint order. This example is the visual reproduction case (and a
//! golden-screenshot source) for that regression.

use sequoia_ui::platform::{
    AshFrame, AshRegion, AshSurface, AshSurfaceRenderer, RenderBackend, WgpuFrame, WgpuRenderer,
    WgpuSurface,
};
use sequoia_ui::prelude::*;
use sequoia_ui::render::GpuBackend;
use sequoia_ui::wgpu;

const BG: Color = Color::hex(0x0B0D12);
const PANEL: Color = Color::hex(0x151922);
const TEXT: Color = Color::hex(0xF2F4F8);
const MUTED: Color = Color::hex(0x8A93A6);

const SHADER: &str = r#"
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    let corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VsOut;
    out.clip = vec4<f32>(corners[index], 0.0, 1.0);
    out.uv = corners[index] * 0.5 + vec2<f32>(0.5);
    return out;
}

fn map_scene(p: vec3<f32>) -> f32 {
    let sphere = length(p - vec3<f32>(0.0, 0.05, 0.0)) - 0.85;
    let plane = p.y + 0.9;
    return min(sphere, plane);
}

fn normal_at(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.002, 0.0);
    return normalize(vec3<f32>(
        map_scene(p + e.xyy) - map_scene(p - e.xyy),
        map_scene(p + e.yxy) - map_scene(p - e.yxy),
        map_scene(p + e.yyx) - map_scene(p - e.yyx),
    ));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = (in.uv - vec2<f32>(0.5)) * vec2<f32>(1.6, 1.0);
    let origin = vec3<f32>(0.0, 0.55, 3.1);
    let dir = normalize(vec3<f32>(uv.x, uv.y, -1.5));

    var travel = 0.0;
    var hit = false;
    for (var step = 0; step < 56; step = step + 1) {
        let point = origin + dir * travel;
        let distance = map_scene(point);
        if (distance < 0.001) {
            hit = true;
            break;
        }
        if (travel > 12.0) {
            break;
        }
        travel = travel + distance * 0.85;
    }

    var color = vec3<f32>(0.05, 0.06, 0.10);
    if (hit) {
        let point = origin + dir * travel;
        let normal = normal_at(point);
        let light = normalize(vec3<f32>(0.55, 0.85, 0.45));
        let diffuse = max(dot(normal, light), 0.0);
        var base = vec3<f32>(0.24, 0.56, 0.96);
        if (point.y < -0.85) {
            base = vec3<f32>(0.15, 0.16, 0.22);
        }
        let specular = pow(max(dot(reflect(-light, normal), -dir), 0.0), 28.0);
        let rim = pow(1.0 - max(dot(normal, -dir), 0.0), 3.0);
        color = base * (0.18 + 0.82 * diffuse) + vec3<f32>(specular * 0.55 + rim * 0.12);
    }
    return vec4<f32>(color, 1.0);
}
"#;

const HEAVY_SHADER: &str = r#"
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    let corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VsOut;
    out.clip = vec4<f32>(corners[index], 0.0, 1.0);
    out.uv = corners[index] * 0.5 + vec2<f32>(0.5);
    return out;
}

fn sd_torus(p: vec3<f32>, radii: vec2<f32>) -> f32 {
    let q = vec2<f32>(length(p.xz) - radii.x, p.y);
    return length(q) - radii.y;
}

fn sd_box(p: vec3<f32>, half_size: vec3<f32>) -> f32 {
    let q = abs(p) - half_size;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn map_scene(p: vec3<f32>) -> f32 {
    let ball_a = length(p - vec3<f32>(-0.95, -0.3, 0.1)) - 0.58;
    let ball_b = length(p - vec3<f32>(0.85, -0.45, -0.5)) - 0.45;
    let ball_c = length(p - vec3<f32>(0.1, 0.75, 0.6)) - 0.35;
    let ring = sd_torus(p - vec3<f32>(0.0, 0.15, 0.0), vec2<f32>(0.95, 0.2));
    let outer_ring = sd_torus(p - vec3<f32>(0.0, 0.15, 0.0), vec2<f32>(1.35, 0.06));
    let block = sd_box(p - vec3<f32>(-1.5, -0.65, -1.1), vec3<f32>(0.32, 0.3, 0.32));
    let plane = p.y + 0.95;
    return min(min(min(ball_a, ball_b), min(ball_c, block)), min(min(ring, outer_ring), plane));
}

fn normal_at(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.0015, 0.0);
    return normalize(vec3<f32>(
        map_scene(p + e.xyy) - map_scene(p - e.xyy),
        map_scene(p + e.yxy) - map_scene(p - e.yxy),
        map_scene(p + e.yyx) - map_scene(p - e.yyx),
    ));
}

fn march(ro: vec3<f32>, rd: vec3<f32>, steps: i32) -> f32 {
    var travel = 0.05;
    for (var i = 0; i < steps; i = i + 1) {
        let distance = map_scene(ro + rd * travel);
        if (distance < 0.0008 || travel > 14.0) {
            break;
        }
        travel = travel + distance * 0.85;
    }
    return travel;
}

fn shadow(origin: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    var shade = 1.0;
    var travel = 0.06;
    for (var i = 0; i < 32; i = i + 1) {
        let distance = map_scene(origin + light_dir * travel);
        shade = min(shade, 10.0 * distance / travel);
        travel = travel + clamp(distance, 0.03, 0.4);
        if (distance < 0.001 || travel > 9.0) {
            break;
        }
    }
    return clamp(shade, 0.0, 1.0);
}

fn occlusion(point: vec3<f32>, normal: vec3<f32>) -> f32 {
    var occ = 0.0;
    var scale = 1.0;
    for (var i = 0; i < 8; i = i + 1) {
        let height = 0.02 + 0.07 * f32(i);
        let distance = map_scene(point + normal * height);
        occ = occ + (height - distance) * scale;
        scale = scale * 0.72;
    }
    return clamp(1.0 - 2.2 * occ, 0.0, 1.0);
}

fn surface_color(point: vec3<f32>, ray_dir: vec3<f32>, light_dir: vec3<f32>) -> vec3<f32> {
    let normal = normal_at(point);
    let diffuse = max(dot(normal, light_dir), 0.0);
    let shade = shadow(point + normal * 0.01, light_dir);
    let ao = occlusion(point, normal);
    var base = vec3<f32>(0.22, 0.52, 0.94);
    if (point.y < -0.9) {
        let checker = (floor(point.x * 2.0) + floor(point.z * 2.0)) % 2.0;
        base = mix(vec3<f32>(0.10, 0.11, 0.16), vec3<f32>(0.24, 0.26, 0.34), checker);
    }
    let specular = pow(max(dot(reflect(-light_dir, normal), -ray_dir), 0.0), 42.0);
    let rim = pow(1.0 - max(dot(normal, -ray_dir), 0.0), 3.0);
    return base * (0.12 + 0.88 * diffuse * shade) * ao + vec3<f32>(specular * shade * 0.7 + rim * 0.15);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = (in.uv - vec2<f32>(0.5)) * vec2<f32>(1.6, 1.0);
    let origin = vec3<f32>(0.0, 0.55, 3.1);
    let dir = normalize(vec3<f32>(uv.x, uv.y, -1.5));
    let light_dir = normalize(vec3<f32>(0.55, 0.85, 0.45));

    var color = vec3<f32>(0.04, 0.05, 0.09);
    let travel = march(origin, dir, 256);
    if (travel < 14.0) {
        let point = origin + dir * travel;
        let normal = normal_at(point);
        color = surface_color(point, dir, light_dir);

        let reflected = reflect(dir, normal);
        let bounce_origin = point + normal * 0.02;
        let bounce = march(bounce_origin, reflected, 96);
        if (bounce < 14.0) {
            let bounce_point = bounce_origin + reflected * bounce;
            let reflected_color = surface_color(bounce_point, reflected, light_dir);
            let bounce_normal = normal_at(bounce_point);
            let second = reflect(reflected, bounce_normal);
            let second_origin = bounce_point + bounce_normal * 0.02;
            let second_dist = march(second_origin, second, 48);
            var second_color = vec3<f32>(0.04, 0.05, 0.09);
            if (second_dist < 14.0) {
                second_color = surface_color(second_origin + second * second_dist, second, light_dir);
            }
            let fresnel = 0.04 + 0.55 * pow(1.0 - max(dot(normal, -dir), 0.0), 5.0);
            color = mix(color, mix(reflected_color, second_color, 0.4), fresnel);
        }
    }
    return vec4<f32>(color, 1.0);
}
"#;

fn shader_source() -> &'static str {
    if std::env::var_os("SEQUOIA_HEAVY_3D").is_some() {
        HEAVY_SHADER
    } else {
        SHADER
    }
}

fn spirv() -> Vec<u32> {
    let module = naga::front::wgsl::parse_str(shader_source()).expect("parse 3D shader");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .expect("validate 3D shader");
    naga::back::spv::write_vec(
        &module,
        &info,
        &naga::back::spv::Options {
            fake_missing_bindings: true,
            ..Default::default()
        },
        None,
    )
    .expect("compile 3D shader to SPIR-V")
}

struct WgpuScene3D {
    pipeline: Option<(wgpu::TextureFormat, wgpu::RenderPipeline)>,
}

impl WgpuRenderer for WgpuScene3D {
    fn render(&mut self, frame: &mut WgpuFrame<'_>) {
        let pipeline = match self.pipeline.as_ref() {
            Some((format, pipeline)) if *format == frame.format => pipeline,
            _ => {
                let module = frame
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("surface-3d"),
                        source: wgpu::ShaderSource::Wgsl(shader_source().into()),
                    });
                let pipeline =
                    frame
                        .device
                        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                            label: Some("surface-3d"),
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
                        });
                &self.pipeline.insert((frame.format, pipeline)).1
            }
        };

        let region = frame.region;
        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("surface-3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
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

struct AshCache {
    device: ash::Device,
    render_pass: ash::vk::RenderPass,
    format: ash::vk::Format,
    module: ash::vk::ShaderModule,
    pipeline: ash::vk::Pipeline,
}

impl Drop for AshCache {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_shader_module(self.module, None);
        }
    }
}

struct AshScene3D {
    cache: Option<AshCache>,
}

impl AshScene3D {
    fn ensure_pipeline(&mut self, frame: &AshFrame<'_>) -> &AshCache {
        if self.cache.as_ref().is_some_and(|cache| {
            cache.render_pass == frame.render_pass && cache.format == frame.format
        }) {
            return self.cache.as_ref().expect("checked above");
        }

        use ash::vk;
        let device = frame.device.clone();
        let words = spirv();
        let module = unsafe {
            device
                .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&words), None)
                .expect("create 3D shader module")
        };
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(module)
                .name(c"vs_main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(module)
                .name(c"fs_main"),
        ];
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let attachment = [vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)];
        let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&attachment);
        let dynamic = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization)
            .multisample_state(&multisample)
            .color_blend_state(&blend)
            .dynamic_state(&dynamic)
            .layout(frame.pipeline_layout)
            .render_pass(frame.render_pass)
            .subpass(0);
        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
                .expect("create 3D pipeline")
                .remove(0)
        };
        self.cache.insert(AshCache {
            device,
            render_pass: frame.render_pass,
            format: frame.format,
            module,
            pipeline,
        })
    }
}

impl AshSurfaceRenderer for AshScene3D {
    fn render(&mut self, frame: &mut AshFrame<'_>) {
        use ash::vk;
        let cache = self.ensure_pipeline(frame);
        let area = AshRegion {
            x: frame.region.x,
            y: frame.region.y,
            width: frame.region.width,
            height: frame.region.height,
        };
        unsafe {
            frame.device.cmd_set_viewport(
                frame.command_buffer,
                0,
                &[vk::Viewport::default()
                    .x(area.x as f32)
                    .y(area.y as f32)
                    .width(area.width as f32)
                    .height(area.height as f32)
                    .max_depth(1.0)],
            );
            frame.device.cmd_set_scissor(
                frame.command_buffer,
                0,
                &[vk::Rect2D::default()
                    .offset(vk::Offset2D {
                        x: area.x as i32,
                        y: area.y as i32,
                    })
                    .extent(vk::Extent2D {
                        width: area.width,
                        height: area.height,
                    })],
            );
            frame.device.cmd_bind_pipeline(
                frame.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                cache.pipeline,
            );
            frame.device.cmd_draw(frame.command_buffer, 3, 1, 0, 0);
        }
    }
}

fn main() {
    let ash = std::env::var("SEQUOIA_RENDER_BACKEND")
        .map(|value| value.to_ascii_lowercase().contains("ash"))
        .unwrap_or(false);
    let backend = if ash {
        RenderBackend::VulkanAsh
    } else {
        RenderBackend::Wgpu(GpuBackend::Vulkan)
    };

    if ash {
        run(
            backend,
            AshSurface::new(AshScene3D { cache: None })
                .background(PANEL)
                .border(1.0, Color::hex(0x30363D))
                .radius(12.0)
                .flex(1.0),
        );
    } else {
        run(
            backend,
            WgpuSurface::new(WgpuScene3D { pipeline: None })
                .background(PANEL)
                .border(1.0, Color::hex(0x30363D))
                .radius(12.0)
                .flex(1.0),
        );
    }
}

fn run<V: View + 'static>(backend: RenderBackend, surface: V) {
    let view = Column::new()
        .fill_size()
        .background(BG)
        .padding(20.0)
        .gap(12.0)
        .child(
            Text::new("Embedded 3D surface")
                .size(22.0)
                .bold()
                .color(TEXT),
        )
        .child(
            Text::new(format!("Raymarched sphere — {} backend", backend.name()))
                .size(13.0)
                .color(MUTED),
        )
        .child(
            Stack::new()
                .fill_size()
                .child_at(surface, StackPos::fill())
                .child_at(
                    Text::new("Overlay painted after the surface")
                        .size(18.0)
                        .bold()
                        .color(TEXT),
                    StackPos::bottom().offset(0.0, -12.0),
                ),
        );

    let app = App::new()
        .title(format!("3D surface — {}", backend.name()))
        .size(1200.0, 800.0)
        .render_backend(backend)
        .root(view);

    let app = if let Ok(frames) = std::env::var("SEQUOIA_BENCH_FRAMES") {
        app.benchmark(frames.parse().unwrap_or(300))
    } else {
        app
    };
    app.run().expect("event loop failed");
}
