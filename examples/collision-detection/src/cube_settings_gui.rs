
use cgmath::Vector3;
use hello_wgpu::gui::SendAny;

use hello_wgpu::gui::windows::GuiWindow;

pub struct CubeSettingsEvent {
    pub position: Vector3<f32>,
}

pub struct CubeSettingsWindow {
    pub position: Vector3<f32>,
}

impl CubeSettingsWindow {
    pub fn new(
        position: Vector3<f32>,
    ) -> Self {
        Self {
            position,
        }
    }
}

impl GuiWindow for CubeSettingsWindow {
    fn show(
        &mut self,
        ctx: &egui::Context,
    ) {
        egui::Window::new("🧊 Cube")
            .resizable(true)
            .vscroll(true)
            .default_open(true)
            .show(&ctx, |ui| {
                egui::Grid::new("cube_settings_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("x");
                        ui.add(egui::DragValue::new(&mut self.position.x)
                            .speed(0.01)
                        );
                        ui.end_row();
                        ui.label("y");
                        ui.add(egui::DragValue::new(&mut self.position.y)
                            .speed(0.01)
                        );
                        ui.end_row();
                        ui.label("z");
                        ui.add(egui::DragValue::new(&mut self.position.z)
                            .speed(0.01)
                        );
                        ui.end_row();
                    });
            });
    }

    fn update(
        &mut self,
        event: &SendAny,
    ) {
        if let Some(cube_settings_event) = event.downcast_ref::<CubeSettingsEvent>() {
            self.position = cube_settings_event.position;
        }
    }

    fn get_state_event(
        &self,
    ) -> Box<SendAny> {
        Box::new(
            CubeSettingsEvent {
                position: self.position,
            }
        )
    }
}