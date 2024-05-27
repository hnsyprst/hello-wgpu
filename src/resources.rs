// Serve resources in wasm

use std::{
    io::BufReader,
    io::Cursor,
};

use cfg_if::cfg_if;
use wgpu::util::DeviceExt;

use crate::{model, texture};

#[cfg(target_arch = "wasm32")]
fn format_url(
    file_name: &str,
) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    if !origin.ends_with("res") {
        origin = format!("{}/res", origin);
    }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_string(
    file_name: &str,
) -> anyhow::Result<String> {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(file_name);
            let txt = reqwest::get(url)
                .await?
                .text()
                .await?;
        } else {
            let path = std::path::Path::new(env!("OUT_DIR"))
                .join("res")
                .join(file_name);
            let txt = fs::read_to_string(path)?;
        }
    }

    Ok(txt)
}

pub async fn load_binary(
    file_name: &str,
) -> anyhow::Result<Vec<u8>> {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(file_name);
            let data = reqwest::get(url)
                .await?
                .bytes()
                .await?
                .to_vec();
        } else {
            let path = std::path::Path::new(env!("OUT_DIR"))
                .join("res")
                .join(file_name);
            let data = fs::read(path)?;
        }
    }

    Ok(data)
}

#[derive(serde::Deserialize, Debug)]
struct Response<T> {
    data: Vec<T>,
}

pub async fn load_json<T>(
    file_name: &str,
) -> anyhow::Result<Vec<T>> where T: serde::de::DeserializeOwned {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(file_name);
            let data: Response<T> = reqwest::get(url)
                .await?
                .json::<Response<T>>()
                .await?;
        } else {
            let path = std::path::Path::new(env!("OUT_DIR"))
                .join("res")
                .join(file_name);
            let data: Response<T> = serde_json::from_reader(BufReader::new(fs::File::open(path)?))?;
        }
    }

    Ok(data.data)
}

// TODO: Default texture if load texture_fails
pub async fn load_texture(
    file_name: &str,
    is_normal_map: bool,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name, is_normal_map)
}

pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&p).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    ).await?;

    let mut materials = Vec::new();
    for m in obj_materials? {
        let diffuse_texture = load_texture(&m.diffuse_texture, false, device, queue).await?;
        let normal_texture = load_texture(&m.normal_texture, true, device, queue).await?;

        materials.push(model::Material::new(
            device,
            &m.name,
            diffuse_texture,
            normal_texture,
            layout,
        ))
    }

    let meshes = models.into_iter().map(|m| {
        let mut vertices = (0..m.mesh.positions.len() / 3).map(|i| model::ModelVertex {
            position: [
                m.mesh.positions[i * 3],
                m.mesh.positions[i * 3 + 1],
                m.mesh.positions[i * 3 + 2],
            ],
            tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
            normal: [
                m.mesh.normals[i * 3],
                m.mesh.normals[i * 3 + 1],
                m.mesh.normals[i * 3 + 2],
            ],
            // We'll calculate these later, just set to 0 for now
            tangent: [0.0; 3],
            bitangent: [0.0; 3],
        }).collect::<Vec<_>>();

        let indices = &m.mesh.indices;
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", file_name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", file_name)),
            contents: bytemuck::cast_slice(&m.mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        model::Mesh {
            name: file_name.to_string(),
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
            num_elements: m.mesh.indices.len() as u32,
            material: m.mesh.material_id.unwrap_or(0),
        }
    }).collect::<Vec<_>>();

    Ok(model::Model {
        meshes,
        materials,
    })
}