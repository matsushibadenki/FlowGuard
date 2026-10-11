//! Dense-map native GPU backend. All buffers and the rendered texture stay in Rust/wgpu.
//! Native-surface presentation is not connected to the WebView's compact UI map yet.
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}
#[derive(Clone, Copy)]
pub struct Route {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub color: [f32; 4],
}
pub struct NativeRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
impl NativeRenderer {
    pub async fn new(width: u32, height: u32) -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| e.to_string())?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("FlowGuard native renderer"),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FlowGuard route shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("routes.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FlowGuard routes"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x4],
                }],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let (width, height) = (width.clamp(1, 4096), height.clamp(1, 4096));
        let texture = Self::texture(&device, width, height);
        let view = texture.create_view(&Default::default());
        Ok(Self {
            device,
            queue,
            pipeline,
            texture,
            view,
            width,
            height,
        })
    }
    fn texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FlowGuard GPU-owned map"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        let (width, height) = (width.clamp(1, 4096), height.clamp(1, 4096));
        if (width, height) == (self.width, self.height) {
            return;
        }
        self.texture = Self::texture(&self.device, width, height);
        self.view = self.texture.create_view(&Default::default());
        self.width = width;
        self.height = height;
    }
    /// Routes use normalized [0,1] positions. This method never returns a pixel buffer.
    pub fn render(&self, routes: &[Route]) -> Result<(), String> {
        if routes.len() > 10_000 {
            return Err("scene_limit".into());
        }
        let mut vertices = Vec::with_capacity(routes.len() * 32 * 6);
        for route in routes {
            if route
                .from
                .iter()
                .chain(route.to.iter())
                .chain(route.color.iter())
                .any(|v| !v.is_finite())
            {
                return Err("invalid_scene".into());
            }
            append_route(&mut vertices, *route, self.width, self.height);
        }
        let buffer = if vertices.is_empty() {
            None
        } else {
            Some(
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("FlowGuard native route vertices"),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
            )
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FlowGuard render commands"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FlowGuard route pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.96,
                            g: 0.98,
                            b: 0.98,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(buffer) = buffer.as_ref() {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..vertices.len() as u32, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }
    /// Borrowed only by a future native compositor. No serialization or IPC readback API.
    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.view
    }
}
fn append_route(vertices: &mut Vec<Vertex>, route: Route, width: u32, height: u32) {
    let a = route.from;
    let b = route.to;
    let center = (a[0] + b[0]) * 0.5;
    let point = |t: f32| {
        let u = 1.0 - t;
        [
            u * u * u * a[0]
                + 3.0 * u * u * t * center
                + 3.0 * u * t * t * center
                + t * t * t * b[0],
            u * u * u * a[1] + 3.0 * u * u * t * a[1] + 3.0 * u * t * t * b[1] + t * t * t * b[1],
        ]
    };
    for i in 0..32 {
        let p = point(i as f32 / 32.0);
        let q = point((i + 1) as f32 / 32.0);
        let dx = (q[0] - p[0]) * width as f32;
        let dy = (q[1] - p[1]) * height as f32;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            continue;
        }
        let offset = [-dy / len / width as f32, dx / len / height as f32];
        let corners = [
            [p[0] + offset[0], p[1] + offset[1]],
            [p[0] - offset[0], p[1] - offset[1]],
            [q[0] + offset[0], q[1] + offset[1]],
            [q[0] - offset[0], q[1] - offset[1]],
        ];
        for index in [0, 1, 2, 2, 1, 3] {
            let c = corners[index];
            vertices.push(Vertex {
                position: [c[0] * 2.0 - 1.0, 1.0 - c[1] * 2.0],
                color: route.color,
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tessellation_remains_finite_and_bounded() {
        let mut vertices = vec![];
        append_route(
            &mut vertices,
            Route {
                from: [0.1, 0.2],
                to: [0.9, 0.8],
                color: [0.1, 0.4, 0.3, 1.0],
            },
            800,
            600,
        );
        assert_eq!(vertices.len(), 192);
        assert!(vertices
            .iter()
            .all(|v| v.position.iter().all(|n| n.is_finite() && n.abs() <= 1.0)));
    }
}
