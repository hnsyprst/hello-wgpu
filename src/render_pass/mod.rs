use wgpu::{Device, Queue, Surface};

use crate::{model::Model, object::Object, app::AppData, State};

pub mod phong;
pub mod basic;

pub trait RenderPass {
    fn draw(
        &mut self,
        app_data: &AppData,
        view: wgpu::TextureView,
        encoder: wgpu::CommandEncoder,
        objects: &Vec<Object>,
    ) -> Result<(), wgpu::SurfaceError>;
}