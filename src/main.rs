use std::{fs, iter};

use anyhow::Result;
use pollster::FutureExt;
use wgpu::{
    Adapter, Backends, BlendState, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    CompareFunction, CompositeAlphaMode, DepthBiasState, DepthStencilState, Device,
    DeviceDescriptor, Extent3d, Features, FragmentState, FrontFace, Instance, InstanceDescriptor,
    Limits, MultisampleState, PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PresentMode,
    PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, ShaderModule, ShaderModuleDescriptor,
    StencilState, Surface, SurfaceConfiguration, SurfaceTexture, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    VertexState,
};
use winit::{
    dpi::PhysicalSize,
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

    output: MainRenderTarget,
}

struct MainRenderTarget {
    width: u32,
    height: u32,

    surface: Surface,
    depth_buffer: Texture,
    surface_format: TextureFormat,
}

impl MainRenderTarget {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    fn create_depthbuf(device: &Device, width: u32, height: u32) -> Texture {
        device.create_texture(&TextureDescriptor {
            dimension: TextureDimension::D2,
            label: Some("texture:depth"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    fn config_surface(
        device: &Device,
        surface: &Surface,
        surface_format: TextureFormat,
        width: u32,
        height: u32,
    ) {
        surface.configure(
            device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: width,
                height: height,
                present_mode: PresentMode::Fifo,
                alpha_mode: CompositeAlphaMode::Opaque,
                view_formats: vec![],
            },
        );
    }

    pub fn from_window_surface(
        device: &Device,
        surface: Surface,
        surface_format: TextureFormat,
        dims: PhysicalSize<u32>,
    ) -> Self {
        Self::config_surface(device, &surface, surface_format, dims.width, dims.height);

        Self {
            width: dims.width,
            height: dims.height,

            surface,
            surface_format,
            depth_buffer: Self::create_depthbuf(device, dims.width, dims.height),
        }
    }

    fn resize(&mut self, device: &Device, inner_size: PhysicalSize<u32>) {
        Self::config_surface(
            device,
            &self.surface,
            self.surface_format,
            inner_size.width,
            inner_size.height,
        );
        self.depth_buffer = Self::create_depthbuf(device, inner_size.width, inner_size.height);
    }
}

impl Renderer {
    pub fn draw(&mut self) -> Result<()> {
        let window_tex = self.output.surface.get_current_texture()?;
        let window_view = window_tex.texture.create_view(&TextureViewDescriptor {
            label: Some("view:output-color"),
            ..Default::default()
        });

        let depth_view = self
            .output
            .depth_buffer
            .create_view(&TextureViewDescriptor {
                label: Some("view:output-depth"),
                ..Default::default()
            });

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("encoder0"),
            });

        {
            let _rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pass:clear"),
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
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            // rp.set_pipeline(&self.pipe);
            // rp.draw_indexe
        }

        {
            let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pass:0"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &window_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            rp.set_pipeline(&self.pipe);
            rp.draw(0..3, 0..1);
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

        let output = MainRenderTarget::from_window_surface(
            &device,
            window_surface,
            surf_format,
            window.inner_size(),
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
                targets: &[Some(ColorTargetState {
                    format: output.surface_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
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
            depth_stencil: Some(DepthStencilState {
                format: MainRenderTarget::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::LessEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
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
            output,
            pipe,
        })
    }

    fn resize(&mut self, inner_size: winit::dpi::PhysicalSize<u32>) {
        log::info!("resized to {:?}", inner_size);

        self.output.resize(&self.device, inner_size);
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
                event: window_event,
            } => match window_event {
                WindowEvent::CloseRequested => flow.set_exit(),
                WindowEvent::Resized(_) => {
                    renderer.resize(window.inner_size());
                }
                _ => (),
            },

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
