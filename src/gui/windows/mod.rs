use super::SendAny;

pub mod performance;
pub mod stats;

pub trait GuiWindow {
    fn show(
        &mut self,
        ctx: &egui::Context
    );

    fn update(
        &mut self,
        event: &SendAny,
    );
}