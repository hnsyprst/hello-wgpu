use crate::mesh::Mesh;

pub mod cuboid;
pub mod sphere;

pub trait Meshable {
    fn get_vecs(
        &self,
        resolution: i32,
    ) -> (
        Vec<[f32; 3]>,
        Vec<[f32; 3]>,
        Vec<[f32; 2]>,
        Vec<u32>,
    );

    fn build_mesh(
        &self,
        device: &wgpu::Device,
        positions: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<u32>,
        resolution: i32,
    ) -> Mesh;
}