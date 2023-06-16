use log::debug;
use wgpu::{TextureFormat, Device, RenderPassColorAttachment, RenderPassDescriptor, TextureView, CommandEncoder};
use winit::{window::Window, dpi::PhysicalSize};

use crate::render::RenderCtx;

pub struct EguiState {
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

    pub fn run<F>(
        &mut self,
        window: &Window,
        ui: F,
    ) -> (egui::TexturesDelta, Vec<egui::ClippedPrimitive>)
    where
        F: FnOnce(&egui::Context),
    {
        let input = self.winit.take_egui_input(&window);
        self.ctx.begin_frame(input);

        ui(&self.ctx);

        let full_output = self.ctx.end_frame();
        self.winit
            .handle_platform_output(&window, &self.ctx, full_output.platform_output);

        // tesselate
        (
            full_output.textures_delta,
            self.ctx.tessellate(full_output.shapes),
        )
    }

    pub fn render(
        &mut self,
        rctx: &RenderCtx,
        encoder: &mut CommandEncoder,
        output_view: &TextureView,
        dims: PhysicalSize<u32>,
        dpiscale: f32,

        texdelta: egui::TexturesDelta,
        paint_jobs: &[egui::ClippedPrimitive],
    ) {
        // upload changed textures
        for (tid, imgdelta) in texdelta.set {
            self.renderer
                .update_texture(&rctx.device, &rctx.queue, tid, &imgdelta);
        }

        // upload buffers
        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: dims.into(),
            pixels_per_point: dpiscale,
        };
        self.renderer.update_buffers(
            &rctx.device,
            &rctx.queue,
            encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pass:egui"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.renderer
                .render(&mut rp, &paint_jobs, &screen_descriptor);
        }

        for tid in texdelta.free {
            self.renderer.free_texture(&tid);
        }
    }
}
