use std::sync::Arc;
use std::collections::HashMap;

use egui::*;
use egui_wgpu::{Renderer, ScreenDescriptor};
use wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView};
use winit::event::WindowEvent;
use winit;

use super::windows::GuiWindow;
use super::SendAny;

pub struct EguiRenderer {
    state: egui_winit::State,
    renderer: Renderer,
    window: Arc<winit::window::Window>,
    pub screen_descriptor: ScreenDescriptor,
    gui_windows: HashMap<String, Box<dyn GuiWindow>>,
}

impl EguiRenderer {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: Arc<winit::window::Window>,
        screen_descriptor: ScreenDescriptor,
    ) -> EguiRenderer {
        let egui_context = Context::default();

        let egui_state = egui_winit::State::new(
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

        let gui_windows: HashMap<String, Box<dyn GuiWindow>> = HashMap::new();

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
            window: window,
            screen_descriptor: screen_descriptor,
            gui_windows,
        }
    }

    pub fn add_gui_window(
        &mut self,
        gui_window_name: &str,
        gui_window: Box<dyn GuiWindow>,
    ) {
        self.gui_windows.insert(gui_window_name.to_string(), gui_window);
    }

    pub fn send_event(
        &mut self,
        gui_window_name: &str,
        event: &SendAny,
    ) {
        self.gui_windows
            .get_mut(gui_window_name)
            .unwrap()
            .update(event);
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
    ) {
        self.state
            .egui_ctx()
            .set_pixels_per_point(self.screen_descriptor.pixels_per_point);

        let raw_input = self.state.take_egui_input(&self.window);
        let full_output = self.state.egui_ctx().run(raw_input, |_ui| {
            for (_, gui_window) in &mut self.gui_windows {
                gui_window.show(&self.state.egui_ctx());
            }
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