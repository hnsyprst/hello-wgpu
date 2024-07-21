use wgpu::util::DeviceExt;
use crate::{mesh::Mesh, vertex};

pub trait Vertex {
    fn describe() -> wgpu::VertexBufferLayout<'static>;
}

// When adding a field here, remember to add it's corresponding wgpu::VertexAttribute to LineVertex::describe()
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
}

impl Vertex for LineVertex {
    fn describe() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

// Draw using render_pass::line::LinePass
pub struct Line {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
}

impl Line {
    pub fn new(
        name: &str,
        positions: Vec<[f32; 3]>,
        device: &wgpu::Device,
    ) -> Self {
        let vertices = positions.iter().map(|p| LineVertex { position: *p }).collect::<Vec<_>>();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            name: name.to_string(),
            vertex_buffer: vertex_buffer,
            num_vertices: vertices.len() as u32,
        }
    }
}