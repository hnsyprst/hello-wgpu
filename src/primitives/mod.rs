use wgpu::util::DeviceExt;
use crate::{mesh::Mesh, vertex};

pub trait Meshable {
    fn build_mesh(
        &self,
        device: &wgpu::Device
    ) -> Mesh;
}

pub struct Cuboid {
    name: String,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

// Build a cuboid, assuming that the cuboid is centered around the origin
impl Cuboid {
    pub fn new(
        name: &str,
        half_size: cgmath::Vector3<f32>, // Vector describing half the size of the cuboid
    ) -> Self {
    let min = -half_size;
    let max = half_size;
    
    // Faces arranged CCW
    let positions_uvs = vec![
        // Front face
        ([min.x, min.y, max.z], [0.0, 0.0]), // 0
        ([max.x, min.y, max.z], [1.0, 0.0]),
        ([max.x, max.y, max.z], [1.0, 1.0]),
        ([min.x, max.y, max.z], [0.0, 1.0]),
        // Back face
        ([min.x, max.y, min.z], [1.0, 0.0]), // 4
        ([max.x, max.y, min.z], [0.0, 0.0]),
        ([max.x, min.y, min.z], [0.0, 1.0]),
        ([min.x, min.y, min.z], [1.0, 1.0]),
        // Right face
        ([max.x, min.y, min.z], [0.0, 0.0]), // 8
        ([max.x, max.y, min.z], [1.0, 0.0]),
        ([max.x, max.y, max.z], [1.0, 1.0]),
        ([max.x, min.y, max.z], [0.0, 1.0]),
        // Left face
        ([min.x, min.y, max.z], [1.0, 0.0]), // 12
        ([min.x, max.y, max.z], [0.0, 0.0]),
        ([min.x, max.y, min.z], [0.0, 1.0]),
        ([min.x, min.y, min.z], [1.0, 1.0]),
        // Top face
        ([max.x, max.y, min.z], [1.0, 0.0]), // 16
        ([min.x, max.y, min.z], [0.0, 0.0]),
        ([min.x, max.y, max.z], [0.0, 1.0]),
        ([max.x, max.y, max.z], [1.0, 1.0]),
        // Bottom face
        ([max.x, min.y, max.z], [0.0, 0.0]), // 20
        ([min.x, min.y, max.z], [1.0, 0.0]),
        ([min.x, min.y, min.z], [1.0, 1.0]),
        ([max.x, min.y, min.z], [0.0, 1.0]),
    ];
    let (positions, uvs): (Vec<[f32; 3]>, Vec<[f32; 2]>) = positions_uvs.into_iter().unzip();
    let indices = vec![
        0, 1, 2, 2, 3, 0, // Front
        4, 5, 6, 6, 7, 4, // Back
        8, 9, 10, 10, 11, 8, // Right
        12, 13, 14, 14, 15, 12, // Left
        16, 17, 18, 18, 19, 16, // Top
        20, 21, 22, 22, 23, 20, // Bottom
    ];
    let face_normals = [
        [0.0, 0.0, 1.0],  // Front
        [0.0, 0.0, -1.0], // Back
        [1.0, 0.0, 0.0],  // Right
        [-1.0, 0.0, 0.0], // Left
        [0.0, 1.0, 0.0],  // Top
        [0.0, -1.0, 0.0], // Bottom
    ];
    let normals = (0..positions.len()).map(|i| {
        face_normals[i / 4]
    }).collect::<Vec<_>>();

    Self {
        name: name.to_string(),
        positions,
        normals,
        uvs,
        indices,
    }
    }
}

impl Meshable for Cuboid {
    fn build_mesh(
        &self,
        device: &wgpu::Device,
    ) -> Mesh {
        let vertices = vertex::vecs_to_model_vertices(
            &self.positions,
            &self.normals,
            &self.uvs,
            &self.indices);
    
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", self.name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", self.name)),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
    
        Mesh {
            name: self.name.to_string(),
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            num_elements: self.indices.len() as u32,
            material: 0,
        }
    }
}
