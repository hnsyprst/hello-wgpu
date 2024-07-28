use wgpu::util::DeviceExt;

use crate::primitives::Meshable;
use super::line::Line;

pub trait Wireframe {
    fn to_wireframe(
        &self,
        name: &str,
        device: &wgpu::Device,
    ) -> Line;
}

// Convert `TriangleList` format primitives into `LineList` compatible `debug::line::Line`
// TODO: Add indexing for Line to make this less costly
impl<T: Meshable> Wireframe for T{
    fn to_wireframe(
            &self,
            name: &str,
            device: &wgpu::Device,
        ) -> Line {
        let ( positions, normals, uvs, indices ) = self.get_vecs();
        
        let num_tris = indices.chunks(3).len();
        let mut strip_indices = Vec::with_capacity(num_tris * 6);

        for index in indices.chunks(3) {
            strip_indices.push(index[0]);
            strip_indices.push(index[1]);
            strip_indices.push(index[1]);
            strip_indices.push(index[2]);
            strip_indices.push(index[2]);
            strip_indices.push(index[0]);
        }

        let vertices = strip_indices.iter().map(|i| positions[*i as usize]).collect::<Vec<_>>();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Line {
            name: format!("{:?} Wireframe", name),
            vertex_buffer: vertex_buffer,
            num_vertices: vertices.len() as u32,
        }
    }
}

// TODO: impl Wireframe for Mesh