pub trait Vertex {
    fn describe() -> wgpu::VertexBufferLayout<'static>;
}

// When adding a field here, remember to add it's corresponding wgpu::VertexAttribute to ModelVertex::describe()
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
}

impl Vertex for ModelVertex {
    fn describe() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress, // `array_stride` defines how wide a vertex is; when the shader goes to read the next vertex, it will skip over `array_stride` bytes
            step_mode: wgpu::VertexStepMode::Vertex, // `step_mode` tells the pipeline whether each element of the array represents per-vertex or per-instance data
            attributes: &[ // The attributes that make up a single vertex
                wgpu::VertexAttribute {
                    offset: 0, // defines the offset in bytes from the start of the struct until this attribute begins
                    shader_location: 0, // which location in the shader to store this attribute (in this case, @location(0))
                    format: wgpu::VertexFormat::Float32x3, // `format` tells the shader the shape of the attribute: `Float32x3` corresponds to `vec3<f32>`
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ]
        }
    }
}

pub fn vecs_to_model_vertices(
    positions: &Vec<[f32; 3]>,
    normals: &Vec<[f32; 3]>,
    uvs: &Vec<[f32; 2]>,
    indices: &Vec<u32>,
) -> Vec<ModelVertex> {
    let mut vertices = positions
        .into_iter()
        .zip(normals.into_iter())
        .zip(uvs.into_iter())
        .map(|((position, normal), uv)| ModelVertex {
            position: *position,
            tex_coords: *uv,
            normal: *normal,
            // We'll calculate these later, just set to 0 for now
            tangent: [0.0; 3],
            bitangent: [0.0; 3],
        })
        .collect::<Vec<_>>();

    let mut triangles_included = vec![0; vertices.len()];

    // Using tris (by looping through indices in groups of 3), 
    // calculate tangents and bitangents
    // See: https://learnopengl.com/Advanced-Lighting/Normal-Mapping
    for tri in indices.chunks(3) {
        let vertex_0 = vertices[tri[0] as usize];
        let vertex_1 = vertices[tri[1] as usize];
        let vertex_2 = vertices[tri[2] as usize];

        let position_0: cgmath::Vector3<f32> = vertex_0.position.into();
        let position_1: cgmath::Vector3<f32> = vertex_1.position.into();
        let position_2: cgmath::Vector3<f32> = vertex_2.position.into();

        let uv_0: cgmath::Vector2<f32> = vertex_0.tex_coords.into();
        let uv_1: cgmath::Vector2<f32> = vertex_1.tex_coords.into();
        let uv_2: cgmath::Vector2<f32> = vertex_2.tex_coords.into();

        // Calculate the edges of the triangle
        let delta_position_1 = position_1 - position_0;
        let delta_position_2 = position_2 - position_0;

        // UV delta
        let delta_uv_1 = uv_1 - uv_0;
        let delta_uv_2 = uv_2 - uv_0;

        let inverse_determinant = 1.0 / (delta_uv_1.x * delta_uv_2.y - delta_uv_1.y * delta_uv_2.x);
        let tangent = inverse_determinant * (delta_position_1 * delta_uv_2.y - delta_position_2 * delta_uv_1.y);
        let bitangent = inverse_determinant * (delta_position_2 * delta_uv_1.x - delta_position_1 * delta_uv_2.x);
        for i in 0..2 {
            triangles_included[tri[i] as usize] += 1;
            vertices[tri[i] as usize].tangent = (tangent + cgmath::Vector3::from(vertices[tri[i] as usize].tangent)).into();
            vertices[tri[i] as usize].bitangent = (bitangent + cgmath::Vector3::from(vertices[tri[i] as usize].bitangent)).into();
        }
    }
    for (i, n) in triangles_included.into_iter().enumerate() {
        let denominator = 1.0 / n as f32;
        let mut vertex = &mut vertices[i];
        vertex.tangent = (cgmath::Vector3::from(vertex.tangent) * denominator).into();
        vertex.bitangent = (cgmath::Vector3::from(vertex.bitangent) * denominator).into();
    }

    vertices
}