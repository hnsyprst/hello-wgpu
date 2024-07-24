use wgpu::util::DeviceExt;
use crate::{mesh::Mesh, vertex};
use super::Meshable;

pub struct Sphere {
    pub name: String,
    pub radius: f32,
    pub resolution: i32,
}

// Build a sphere, assuming that the sphere is centered around the origin
impl Sphere {
    pub fn new(
        name: &str,
        radius: f32,
        resolution: i32,
    ) -> Self {
        Self {
            name: name.to_string(),
            radius,
            resolution,
        }
    }
}

impl Meshable for Sphere {
    fn get_vecs(
        &self,
        resolution: i32,
    ) -> (
        Vec<[f32; 3]>,
        Vec<[f32; 3]>,
        Vec<[f32; 2]>,
        Vec<u32>,
    ) {
        let delta_latitude = std::f32::consts::PI / resolution as f32;
        let delta_longitude = 2.0 * std::f32::consts::PI / resolution as f32;

        // TODO: Correct sizes?
        let mut positions = Vec::with_capacity(resolution as usize * resolution as usize);
        let mut normals = Vec::with_capacity(resolution as usize * resolution as usize);
        let mut uvs = Vec::with_capacity(resolution as usize * resolution as usize);
        // FIXME: Definitely not the correct size
        let mut indices = Vec::with_capacity(resolution as usize * resolution as usize);

        /*
        *  Indices
        *  i1 = index1, i2 = index2
        *  i1--i1+1
        *  |  / |
        *  | /  |
        *  i2--i2+1
        */
        for i in 0..=resolution {
            let latitude_angle = std::f32::consts::PI / 2.0 - i as f32 * delta_latitude;
            let xz = self.radius * f32::cos(latitude_angle);
            let y = self.radius * f32::sin(latitude_angle);

            let mut index_1 = i as u32 * (resolution as u32 + 1);
            let mut index_2 = index_1 + resolution as u32 + 1;

            for j in 0..=resolution {
                let longitude_angle = j as f32 * delta_longitude;
                let inverse_radius = 1.0 / self.radius;
                let position = [xz * f32::cos(longitude_angle), y, xz * f32::sin(longitude_angle)];
                let normal = [position[0] * inverse_radius, position[1] * inverse_radius, position[2] * inverse_radius];
                let uv = [j as f32 / resolution as f32, i as f32 / resolution as f32];

                positions.push(position);
                normals.push(normal);
                uvs.push(uv);

                if i < resolution {
                    if i != 0 {
                        indices.push(index_1);
                        indices.push(index_2);
                        indices.push(index_1 + 1);
                    }
                    if i != (resolution - 1) {
                        indices.push(index_1 + 1);
                        indices.push(index_2);
                        indices.push(index_2 + 1);
                    }
                    index_1 += 1;
                    index_2 += 1;
                }
                
            }
        }

        (
            positions,
            normals,
            uvs,
            indices
        )
    }

    fn build_mesh(
        &self,
        device: &wgpu::Device,
        positions: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<u32>,
        resolution: i32,
    ) -> Mesh {
        let vertices = vertex::vecs_to_model_vertices(
            &positions,
            &normals,
            &uvs,
            &indices,
        );
    
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", self.name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", self.name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
    
        Mesh {
            name: self.name.to_string(),
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            num_elements: indices.len() as u32,
            material: 0,
        }
    }
}