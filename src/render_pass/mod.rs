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
        clear_color: &Option<wgpu::Color>, // If not passed, will not clear view or the depth texture
    ) -> Result<wgpu::CommandEncoder, wgpu::SurfaceError>;
}