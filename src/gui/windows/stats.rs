
use crate::gui::SendAny;

use super::GuiWindow;

pub struct StatsEvent {
    pub num_instances: u32,
}

pub struct StatsWindow {
    pub num_instances: u32,
}

impl StatsWindow {
    pub fn new() -> Self {
        Self {
            num_instances: 0,
        }
    }
}

impl GuiWindow for StatsWindow {
    fn show(
        &mut self,
        ctx: &egui::Context,
    ) {
        egui::Window::new("📊 Stats")
            .resizable(true)
            .vscroll(true)
            .default_open(true)
            .show(&ctx, |ui| {
                egui::Grid::new("stats_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Num instances");
                        ui.label(format!("{}", &self.num_instances));
                        ui.end_row();
                    });
            });
    }

    fn update(
        &mut self,
        event: &SendAny,
    ) {
        if let Some(ev) = event.downcast_ref::<StatsEvent>() {
            self.num_instances = ev.num_instances;
        }
    }
}