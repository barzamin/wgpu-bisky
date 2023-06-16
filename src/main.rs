use anyhow::Result;
use egui::FullOutput;
use log::debug;
use pollster::FutureExt;
use std::{fs, iter, mem, ops::Range, process::Output};

use wgpu::{
    Adapter, Backends, BlendState, ColorTargetState, ColorWrites, CommandEncoder,
    CommandEncoderDescriptor, CompositeAlphaMode, DepthStencilState, Device, DeviceDescriptor,
    Features, FragmentState, FrontFace, Instance, InstanceDescriptor, Limits, MultisampleState,
    PipelineLayoutDescriptor, PolygonMode, PresentMode, PrimitiveState, PrimitiveTopology, Queue,
    RenderPass, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

mod camera;
mod render;
mod ui;

use crate::ui::EguiState;
use crate::render::RenderCtx;

struct OutputSurface {
    surface: Surface,
    config: SurfaceConfiguration,
    // depth_buffer: Texture,
    format: TextureFormat,
    dims: PhysicalSize<u32>,
}

impl OutputSurface {
    // pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    // fn create_depthbuf(device: &Device, width: u32, height: u32) -> Texture {
    //     device.create_texture(&TextureDescriptor {
    //         dimension: TextureDimension::D2,
    //         label: Some("texture:depth"),
    //         size: Extent3d {
    //             width,
    //             height,
    //             depth_or_array_layers: 1,
    //         },
    //         mip_level_count: 1,
    //         sample_count: 1,
    //         format: Self::DEPTH_FORMAT,
    //         usage: TextureUsages::RENDER_ATTACHMENT,
    //         view_formats: &[],
    //     })
    // }

    pub fn from_window_surface(
        rctx: &RenderCtx,
        window_surface: Surface,
        dims: PhysicalSize<u32>,
    ) -> Self {
        // find a surface format
        let surface_format = window_surface
            .get_capabilities(&rctx.adapter)
            .formats
            .iter()
            .filter(|fmt| fmt.is_srgb())
            .copied()
            .next()
            .expect("no srgb surface format");
        log::debug!(
            "create_window_surface: surface_formats={:?}",
            surface_format
        );

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: dims.width,
            height: dims.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
        };

        window_surface.configure(&rctx.device, &config);

        Self {
            surface: window_surface,
            config,
            format: surface_format,
            dims,
        }
    }

    fn resize(&mut self, rctx: &RenderCtx, inner_size: PhysicalSize<u32>) {
        debug!("resizing OutputSurface to {:?}", inner_size);
        self.dims = inner_size;
        self.config.width = inner_size.width;
        self.config.height = inner_size.height;
        self.reconfigure(rctx);
    }

    fn reconfigure(&self, rctx: &RenderCtx) {
        self.surface.configure(&rctx.device, &self.config);
    }
}

struct BunnyObj {
    model: tobj::Model,
    // pipeline: RenderPipeline,
}

impl BunnyObj {
    pub fn new() -> Result<Self> {
        let (mut models, _) = tobj::load_obj("./bunny.obj", &tobj::LoadOptions::default())?;
        let model = models
            .pop()
            .expect("should have at least 1 mesh in bunny.obj");

        // let shader_module = rctx.device.create_shader_module(ShaderModuleDescriptor {
        //     label: Some("pipe0:shader"),
        //     source: wgpu::ShaderSource::Wgsl(fs::read_to_string("src/shaders/pipe0.wgsl")?.into()),
        // });

        // let pipelayout = rctx
        //     .device
        //     .create_pipeline_layout(&PipelineLayoutDescriptor {
        //         label: Some("pipe-layout:0"),
        //         bind_group_layouts: &[],
        //         push_constant_ranges: &[],
        //     });

        // let pipeline = rctx
        //     .device
        //     .create_render_pipeline(&RenderPipelineDescriptor {
        //         label: Some("pipe:0"),
        //         layout: Some(&pipelayout),
        //         vertex: VertexState {
        //             module: &shader_module,
        //             entry_point: "vert_main",
        //             buffers: &[VertexBufferLayout {
        //                 // slot 0
        //                 array_stride: (mem::size_of::<f32>() * 3) as u64,
        //                 step_mode: wgpu::VertexStepMode::Vertex,
        //                 attributes: &[VertexAttribute {
        //                     format: VertexFormat::Float32x3,
        //                     offset: 0,
        //                     shader_location: 0,
        //                 }],
        //             }],
        //         },
        //         fragment: Some(FragmentState {
        //             module: &shader_module,
        //             entry_point: "frag_main",
        //             targets: &[Some(ColorTargetState {
        //                 format: output_surface.format,
        //                 blend: Some(BlendState::REPLACE),
        //                 write_mask: ColorWrites::ALL,
        //             })],
        //         }),
        //         primitive: PrimitiveState {
        //             topology: PrimitiveTopology::TriangleList,
        //             strip_index_format: None,
        //             polygon_mode: PolygonMode::Fill,
        //             conservative: false,
        //             front_face: FrontFace::Ccw, // rhs
        //             cull_mode: Some(wgpu::Face::Back),
        //             unclipped_depth: false,
        //         },
        //         depth_stencil: None,
        //         multisample: MultisampleState {
        //             count: 1,
        //             mask: !0,
        //             alpha_to_coverage_enabled: false,
        //         },
        //         multiview: None,
        //     });

        Ok(Self { model })
    }
}

async fn run() -> Result<()> {
    env_logger::init();
    let evloop = EventLoop::new();
    let window = WindowBuilder::new().build(&evloop)?;
    let (mut rctx, window_surface) = RenderCtx::with_window(&window).await?;
    let mut output_surface =
        OutputSurface::from_window_surface(&rctx, window_surface, window.inner_size());

    let mut egui_state = EguiState::new(&window, &rctx.device, output_surface.format);
    let mut demo_app = egui_demo_lib::DemoWindows::default();

    // let bunny = BunnyObj::new(&rctx)?;

    evloop.run(move |ev, tgt, flow| {
        flow.set_poll();
        match ev {
            Event::WindowEvent {
                window_id,
                event: window_event,
            } => {
                let response = egui_state.winit.on_event(&egui_state.ctx, &window_event);

                if !response.consumed {
                    match window_event {
                        WindowEvent::CloseRequested => flow.set_exit(),
                        WindowEvent::Resized(_) => {
                            output_surface.resize(&rctx, window.inner_size());
                        }
                        _ => (),
                    }
                }
            }

            Event::MainEventsCleared => {
                window.request_redraw();
            }

            Event::RedrawRequested(_) => {
                let output_tex = output_surface.surface.get_current_texture().expect("uhh");
                let output_view = output_tex.texture.create_view(&TextureViewDescriptor {
                    label: Some("view:output-color"),
                    ..Default::default()
                });

                // -- ui

                // -- render !!
                let mut encoder = rctx
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("encoder0"),
                    });

                let (texdelta, paint_jobs) = egui_state.run(&window, |ctx| {
                    demo_app.ui(ctx);
                });

                egui_state.render(
                    &rctx,
                    &mut encoder,
                    &output_view,
                    window.inner_size(),
                    window.scale_factor() as f32,
                    texdelta,
                    &paint_jobs,
                );

                rctx.queue.submit(iter::once(encoder.finish()));
                output_tex.present();
            }

            _ => (),
        }
    });

    Ok(())
}

fn main() -> Result<()> {
    run().block_on()
}
