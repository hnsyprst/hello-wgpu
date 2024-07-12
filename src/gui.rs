use std::sync::Arc;
use egui::*;
use egui_wgpu::{Renderer, ScreenDescriptor};
use wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView};
use winit::event::WindowEvent;
use winit::window::Window;

pub struct EguiRenderer {
    state: egui_winit::State,
    renderer: Renderer,
    window: Arc<Window>,
    pub screen_descriptor: ScreenDescriptor,
}

impl EguiRenderer {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: Arc<Window>,
        screen_descriptor: ScreenDescriptor,
    ) -> EguiRenderer {
        let egui_context = Context::default();

        let mut egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
        );
        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
        );

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
            window: window,
            screen_descriptor: screen_descriptor,
        }
    }
    
    pub fn context(
        &self,
    ) -> &Context {
        self.state.egui_ctx()
    }

    pub fn handle_input(
        &mut self,
        event: &WindowEvent,
    ) {
        let _ = self.state.on_window_event(&*self.window, &event);
    }

    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        run_ui: impl FnOnce(&Context),
    ) {
        self.state
            .egui_ctx()
            .set_pixels_per_point(self.screen_descriptor.pixels_per_point);

        let raw_input = self.state.take_egui_input(&self.window);
        let full_output = self.state.egui_ctx().run(raw_input, |ui| {
            run_ui(&self.state.egui_ctx());
        });

        self.state
            .handle_platform_output(&self.window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(&device, &queue, *id, &image_delta);
        }
        self.renderer
            .update_buffers(&device, &queue, encoder, &tris, &self.screen_descriptor);
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui main render pass"),
            occlusion_query_set: None,
        });
        self.renderer.render(&mut rpass, &tris, &self.screen_descriptor);
        drop(rpass);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }
    }
}