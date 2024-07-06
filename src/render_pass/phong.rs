use std::{
    collections::HashMap,
    iter,
};
use crate::{
    app::AppData, camera::{
        self,
        Camera,
        CameraUniform,
    }, instance, light::LightUniform, model::{
        self, DrawModel, Material, Vertex
    }, object::{self, Object}, texture::Texture, State
};
use super::{RenderPass};
use wgpu::{util::DeviceExt, BindGroupLayout, Device, Queue, Surface};

pub struct PhongPass {
    pub camera_uniform: CameraUniform,
    pub global_bind_group_layout: wgpu::BindGroupLayout,
    pub global_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub depth_texture: Texture,
    pub render_pipeline: wgpu::RenderPipeline,
    pub instance_buffers: HashMap<usize, wgpu::Buffer>,
}

impl PhongPass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        camera: &Camera,
        // TODO: Lights could be passed in here instead like the camera
    ) -> Self {
        let phong_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: Some("Phong Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/phong.wgsl").into()),
        });

        let global_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Phong Globals Layout"),
            entries: &[
                // Camera
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Lights
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }, 
            ]
        });
        // Set up camera and create buffer
        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(camera);
        let camera_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        // Set up lighting and create buffer
        // TODO: Lights could be passed in instead like the camera
        let light_uniform = LightUniform {
            position: [10.0, 10.0, 10.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };
        let light_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Phong Light Buffer"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Phong Globals"),
            layout: &global_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_uniform_buffer.as_entire_binding(),
                },
            ]
        });
        let texture_bind_group_layout = device.create_bind_group_layout(&Material::describe());

        // Set up render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
            label: Some("Phong Render Pipeline Layout"),
            bind_group_layouts: &[&global_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { 
            label: Some("Phong Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &phong_shader,
                entry_point: "vs_main",
                buffers: &[model::ModelVertex::describe(), instance::RawInstance::describe()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &phong_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // Every three vertices will correspond to one triangle
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // Tris are facing forward if vertices are arranged in counter-clockwise order
                cull_mode: Some(wgpu::Face::Back), // Tris not facing forward should be culled
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            // TODO: May need tgo be changed, see https://sotrh.github.io/learn-wgpu/intermediate/tutorial10-lighting/#seeing-the-light
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1, // No multisampling
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let depth_texture = Texture::create_depth_texture(&device, &config, "Phong Depth Texture");
        let instance_buffers = HashMap::new();

        Self {
            camera_uniform,
            global_bind_group_layout,
            global_bind_group,
            texture_bind_group_layout,
            depth_texture,
            render_pipeline,
            instance_buffers,
        }
    }
}

impl RenderPass for PhongPass {
    fn draw(
        &mut self,
        app_data: &AppData,
        view: wgpu::TextureView,
        mut encoder: wgpu::CommandEncoder,
        objects: &Vec<Object>,
    ) -> Result<(), wgpu::SurfaceError> {
        // Create a `RenderPass` to clear and render the frame
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view, // Render to the view created above (the output texture)
                resolve_target: None, // The same as `view` unless multisampling is enabled
                ops: wgpu::Operations {
                    // Load tells wgpu what to do with colours stored from the previous frame (here we're just clearing them to a specified colour)
                    load: wgpu::LoadOp::Clear(clear_color),
                    // Tells wgpu whether we want to store the rendered results to the `Texture` behind the `TextureView` in `view`
                    // In this case, that `Texture` is the `SurfaceTexture` and we do want to store the rendered results there
                    store: wgpu::StoreOp::Store, 
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);

        for (object_idx, object) in objects.iter().enumerate() {
            self.instance_buffers
                .entry(object_idx)
                .or_insert_with(|| {
                    let instance_data = object.instances.iter().map(instance::Instance::to_raw).collect::<Vec<_>>();
                    app_data.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Insatance Buffer"),
                            contents: bytemuck::cast_slice(&instance_data),
                            usage: wgpu::BufferUsages::VERTEX,
                    })
                });
        }

        for (object_idx, object) in objects.iter().enumerate() {
            render_pass.set_vertex_buffer(1, self.instance_buffers[&object_idx].slice(..));

            render_pass.draw_model_instanced(
                &object.model,
                0..object.instances.len() as u32,
                &self.global_bind_group,
            );
        }
        
        drop(render_pass); // Need to drop `render_pass` to release the mutable borrow of `encoder` so we can call `encoder.finish()`
        
        // let screen_descriptor = ScreenDescriptor {
        //     size_in_pixels: [app_data.config.width, app_data.config.height],
        //     pixels_per_point: self.window.scale_factor() as f32, // FIXME
        // };

        // `Queue.submit()` will accept anything that implements `IntoIter`, so we wrap `encoder.finish()` up in `std::iter::once`
        app_data.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}