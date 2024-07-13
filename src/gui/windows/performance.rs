
use crate::gui::SendAny;

use super::GuiWindow;

pub struct PerformanceEvent {
    pub fps: f64,
    pub render_time: f64,
    pub update_time: f64,
}

pub struct PerformanceWindow {
    pub fps: f64,
    pub render_time: f64,
    pub update_time: f64,
}

impl PerformanceWindow {
    pub fn new() -> Self {
        Self {
            fps: 0.0,
            render_time: 0.0,
            update_time: 0.0,
        }
    }
}

impl GuiWindow for PerformanceWindow {
    fn show(
        &mut self,
        ctx: &egui::Context,
    ) {
        egui::Window::new("🖥 Performance")
            .resizable(true)
            .vscroll(true)
            .default_open(true)
            .show(&ctx, |ui| {
                egui::Grid::new("performance_info_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("FPS");
                        ui.label(format!("{:.4}", &self.fps));
                        ui.end_row();
                        
                        ui.label("Render time");
                        ui.label(format!("{:.4}ms", &self.render_time * 1000.0));
                        ui.end_row();
                        
                        ui.label("Update time");
                        ui.label(format!("{:.4}ms", &self.update_time * 1000.0));
                        ui.end_row();
                    });
            });
    }

    fn update(
        &mut self,
        event: &SendAny,
    ) {
        if let Some(performance_event) = event.downcast_ref::<PerformanceEvent>() {
            self.fps = performance_event.fps;
            self.render_time = performance_event.render_time;
            self.update_time = performance_event.update_time;
        }
    }
}