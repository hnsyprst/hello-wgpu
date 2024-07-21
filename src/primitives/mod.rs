use crate::mesh::Mesh;

pub mod cuboid;

pub trait Meshable {
    fn build_mesh(
        &self,
        device: &wgpu::Device
    ) -> Mesh;
}