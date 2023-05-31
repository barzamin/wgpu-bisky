use std::{fs, iter};

use anyhow::Result;
use pollster::FutureExt;
use wgpu::{
    Adapter, Backends, Color, CommandEncoderDescriptor, CompositeAlphaMode, Device,
    DeviceDescriptor, Features, FragmentState, FrontFace, Instance, InstanceDescriptor, Limits,
    MultisampleState, PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PresentMode,
    PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModule,
    ShaderModuleDescriptor, Surface, SurfaceConfiguration, TextureUsages, TextureView,
    TextureViewDescriptor, VertexState, SurfaceTexture,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

struct Renderer {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    pipe: RenderPipeline,

    window_surface: Surface,
}

impl Renderer {
    pub fn draw(&mut self) -> Result<()> {
        let window_tex = self.window_surface.get_current_texture()?;
        let window_view = window_tex.texture.create_view(&TextureViewDescriptor {
            label: Some("surface:window:view"),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("encoder0"),
            });

        {
            let _rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pass:test"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &window_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.8,
                            g: 0.2,
                            b: 0.7,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        self.queue.submit(iter::once(encoder.finish()));
        window_tex.present();

        Ok(())
    }

    pub async fn new(window: &Window) -> Result<Self> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::DX12 | Backends::VULKAN | Backends::METAL,
            dx12_shader_compiler: wgpu::Dx12Compiler::Dxc {
                dxil_path: None,
                dxc_path: None,
            }, // todo
        });

        let window_surface = unsafe { instance.create_surface(window) }?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&window_surface),
            })
            .await
            .expect("adapter");

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("device0"),
                    features: Features::empty(),
                    limits: Limits::default(),
                },
                None,
            )
            .await?;

        let win_inner_size = window.inner_size();

        // find a surface format
        let surf_format = window_surface
            .get_capabilities(&adapter)
            .formats
            .iter()
            .filter(|fmt| fmt.is_srgb())
            .copied()
            .next()
            .expect("no srgb surface format");
        log::debug!("surf_formats={:?}", surf_format);

        window_surface.configure(
            &device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: surf_format,
                width: win_inner_size.width,
                height: win_inner_size.height,
                present_mode: PresentMode::Fifo,
                alpha_mode: CompositeAlphaMode::Opaque,
                view_formats: vec![],
            },
        );

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("pipe0:shader"),
            source: wgpu::ShaderSource::Wgsl(fs::read_to_string("src/shaders/pipe0.wgsl")?.into()),
        });

        let pipelayout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipe0:layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipe = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pipe0"),
            layout: Some(&pipelayout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vert_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "frag_main",
                targets: &[],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                front_face: FrontFace::Ccw, // rhs
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            window_surface,
            pipe,
        })
    }
}

async fn run() -> Result<()> {
    env_logger::init();
    let evloop = EventLoop::new();
    let window = WindowBuilder::new().build(&evloop)?;
    let mut renderer = Renderer::new(&window).await?;

    // let device = adapter.

    evloop.run(move |ev, tgt, flow| {
        flow.set_poll();
        match ev {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } => {
                flow.set_exit();
            }

            Event::MainEventsCleared => {
                window.request_redraw();
            }

            Event::RedrawRequested(_) => {
                // draw
                renderer.draw().expect("render error:");
            }

            _ => (),
        }
    });

    Ok(())
}

fn main() -> Result<()> {
    run().block_on()
}
