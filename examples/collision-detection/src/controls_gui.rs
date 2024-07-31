
use cgmath::Vector3;
use hello_wgpu::gui::SendAny;

use hello_wgpu::gui::windows::GuiWindow;

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ColliderChoice {
    AABB,
    Sphere,
} 

pub struct ControlsEvent {
    pub position: Vector3<f32>,
}

pub struct CollisionEvent {
    pub is_colliding: bool,
}

pub struct StateEvent {
    pub position: Vector3<f32>,
    pub is_colliding: bool,
    pub moving_collider: ColliderChoice,
}


pub struct ControlsWindow {
    pub position: Vector3<f32>,
    pub is_colliding: bool,
    moving_collider: ColliderChoice,
}

impl ControlsWindow {
    pub fn new(
        position: Vector3<f32>,
    ) -> Self {
        Self {
            position,
            is_colliding: false,
            moving_collider: ColliderChoice::AABB,
        }
    }
}

impl GuiWindow for ControlsWindow {
    fn show(
        &mut self,
        ctx: &egui::Context,
    ) {
        egui::Window::new("⚙ Controls")
            .resizable(true)
            .vscroll(true)
            .default_open(true)
            .show(&ctx, |ui| {
                egui::Grid::new("controls_grid")
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

                        ui.label("Moving Collider");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.moving_collider, ColliderChoice::AABB, "AABB");
                            ui.selectable_value(&mut self.moving_collider, ColliderChoice::Sphere, "Sphere");
                        });
                        ui.end_row();

                        ui.label("Colliding");
                        ui.label(format!("{}", &self.is_colliding));
                        ui.end_row();
                    });
            });
    }

    fn update(
        &mut self,
        event: &SendAny,
    ) {
        if let Some(controls_event) = event.downcast_ref::<ControlsEvent>() {
            self.position = controls_event.position;
        }
        else if let Some(collision_event) = event.downcast_ref::<CollisionEvent>() {
            self.is_colliding = collision_event.is_colliding;
        }
    }

    fn get_state_event(
        &self,
    ) -> Box<SendAny> {
        Box::new(
            StateEvent {
                position: self.position,
                is_colliding: self.is_colliding,
                moving_collider: self.moving_collider,
            }
        )
    }
}