use crate::{app::AppData, texture::Texture};

pub mod basic;
pub mod phong;
pub mod line;
pub trait RenderPass<T> {
    fn draw(
        &mut self,
        app_data: &AppData,
        view: &wgpu::TextureView,
        encoder: wgpu::CommandEncoder,
        objects: &Vec<T>,
        depth_texture: Option<&Texture>,
    ) -> Result<wgpu::CommandEncoder, wgpu::SurfaceError>;
}