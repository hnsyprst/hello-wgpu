use std::ops::Range;

use wgpu::util::DeviceExt;
use crate::{mesh::Mesh, primitives::cuboid::Cuboid, vertex};

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

    pub fn from_cuboid(
        name: &str,
        cuboid: &Cuboid,
        device: &wgpu::Device,
    ) -> Self {
        let vertices = cuboid.indices.iter().map(|i| LineVertex { position: cuboid.positions[*i as usize] }).collect::<Vec<_>>();
        
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

pub trait DrawLine<'a> {
    fn draw_line(
        &mut self,
        line: &'a Line,
        global_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_line_instanced(
        &mut self,
        line: &'a Line,
        instances: Range<u32>,
        global_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLine<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_line(
        &mut self,
        line: &'b Line,
        global_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_line_instanced(line, 0..1, global_bind_group);
    }

    fn draw_line_instanced(
        &mut self,
        line: &'b Line,
        instances: Range<u32>,
        global_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, line.vertex_buffer.slice(..));
        self.set_bind_group(0, global_bind_group, &[]);
        self.draw(0..line.num_vertices, instances);
    }
}