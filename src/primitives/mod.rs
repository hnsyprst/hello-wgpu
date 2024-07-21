use wgpu::util::DeviceExt;
use crate::{mesh::Mesh, vertex};

pub struct Primitive {
    name: String,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

pub fn build_mesh(
    primitive: Primitive,
    device: &wgpu::Device,
) -> Mesh {
    let vertices = vertex::vecs_to_model_vertices(
        primitive.positions,
        primitive.normals,
        primitive.uvs,
        &primitive.indices);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{:?} Vertex Buffer", primitive.name)),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{:?} Index Buffer", primitive.name)),
        contents: bytemuck::cast_slice(&primitive.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Mesh {
        name: primitive.name,
        vertex_buffer: vertex_buffer,
        index_buffer: index_buffer,
        num_elements: primitive.indices.len() as u32,
        material: 0,
    }
}

// Build a cuboid, assuming that the cuboid is centered around the origin
pub fn create_cuboid(
    name: &str,
    half_size: cgmath::Vector3<f32>, // Vector describing half the size of the cuboid
) -> Primitive {
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

    Primitive {
        name: name.to_string(),
        positions,
        normals,
        uvs,
        indices,
    }
}