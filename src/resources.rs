// Serve resources in wasm

use std::{
    io::BufReader,
    io::Cursor,
    fs,
};

use cfg_if::cfg_if;
use wgpu::util::DeviceExt;

use crate::{model, texture};

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    if !origin.ends_with("res") {
        origin = format!("{}/res", origin);
    }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
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

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
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

pub async fn load_json<T>(file_name: &str) -> anyhow::Result<Vec<T>> where T: serde::de::DeserializeOwned {
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
pub async fn load_texture(file_name: &str, device: &wgpu::Device, queue: &wgpu::Queue) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name)
}

pub async fn load_model(file_name: &str, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> anyhow::Result<model::Model> {
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
        let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
        let normal_texture = load_texture(&m.normal_texture, device, queue).await?;

        materials.push(model::Material::new(
            device,
            &m.name,
            diffuse_texture,
            normal_texture,
            layout,
        ))
    }

    let meshes = models.into_iter().map(|m| {
        let vertices = (0..m.mesh.positions.len() / 3).map(|i| model::ModelVertex {
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
        }).collect::<Vec<_>>();
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