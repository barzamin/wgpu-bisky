use anyhow::Result;
use log::debug;
use pollster::FutureExt;
use std::{fs, iter, mem, ops::Range};

use wgpu::{
    Adapter, Backends, CompositeAlphaMode, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, PresentMode, Queue, RequestAdapterOptions, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages, CommandEncoderDescriptor, RenderPassDescriptor, RenderPassColorAttachment, TextureViewDescriptor,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

mod camera;

struct RenderCtx {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
}
/*
    pipe: RenderPipeline,

    output: MainRenderTarget,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    n_indices: usize,
    cam_bind_group: wgpu::BindGroup,
}
*/

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

impl RenderCtx {
    pub async fn with_window(window: &Window) -> Result<(Self, Surface)> {
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

        Ok((
            Self {
                instance,
                adapter,
                device,
                queue,
            },
            window_surface,
        ))
    }
}

struct EguiState {
    pub ctx: egui::Context,

    pub winit: egui_winit::State,
    pub renderer: egui_wgpu::Renderer,
}

impl EguiState {
    pub fn new(window: &Window, device: &Device, fb_format: TextureFormat) -> Self {
        debug!("init egui backends");

        let context = egui::Context::default();
        let mut winit_state = egui_winit::State::new(window);
        winit_state.set_pixels_per_point(window.scale_factor() as f32);
        let renderer = egui_wgpu::renderer::Renderer::new(device, fb_format, None, 1);

        Self {
            ctx: context,
            winit: winit_state,
            renderer,
        }
    }
}

async fn run() -> Result<()> {
    env_logger::init();
    let evloop = EventLoop::new();
    let window = WindowBuilder::new().build(&evloop)?;
    let (mut rctx, window_surface) = RenderCtx::with_window(&window).await?;
    let mut ctx = egui::Context::default();
    let mut output_surface =
        OutputSurface::from_window_surface(&rctx, window_surface, window.inner_size());

    let mut egui_state = EguiState::new(&window, &rctx.device, output_surface.format);
    let mut demo_app = egui_demo_lib::DemoWindows::default();

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
                            // egui_state.renderer.
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
                let input = egui_state.winit.take_egui_input(&window);
                egui_state.ctx.begin_frame(input);

                demo_app.ui(&egui_state.ctx);

                let full_output = egui_state.ctx.end_frame();
                let paint_jobs = egui_state.ctx.tessellate(full_output.shapes);
                egui_state.winit.handle_platform_output(
                    &window,
                    &egui_state.ctx,
                    full_output.platform_output,
                );

                // -- render !!
                let mut encoder = rctx
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("encoder0"),
                    });

                // upload changed textures
                let texdelta = full_output.textures_delta;
                for (tid, imgdelta) in texdelta.set {
                    egui_state.renderer.update_texture(&rctx.device, &rctx.queue, tid, &imgdelta);
                }

                // upload buffers
                let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
                    size_in_pixels: output_surface.dims.into(),
                    pixels_per_point: window.scale_factor() as f32,
                };
                egui_state.renderer.update_buffers(&rctx.device, &rctx.queue, &mut encoder, &paint_jobs, &screen_descriptor);

                {
                    let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("pass:egui"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &output_view,
                            resolve_target: None,
                            ops: wgpu::Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                    });

                    egui_state.renderer.render(&mut rp, &paint_jobs, &screen_descriptor);
                }

                rctx.queue.submit(iter::once(encoder.finish()));
                output_tex.present();

                for tid in texdelta.free {
                    egui_state.renderer.free_texture(&tid);
                }
            }

            _ => (),
        }
    });

    Ok(())
}

fn main() -> Result<()> {
    run().block_on()
}
