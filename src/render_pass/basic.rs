use std::collections::HashMap;
use crate::{
    app::AppData,
    camera::{
        self,
        Camera,
        CameraUniform,
    },
    instance,
    light,
    model::{
        self,
        DrawLight,
    },
    object::Object,
    texture::Texture,
    vertex::{self, Vertex},
};
use super::RenderPass;
use wgpu::util::DeviceExt;

pub struct BasicPass {
    pub camera_uniform: CameraUniform,
    camera_uniform_buffer: wgpu::Buffer,
    pub light_uniform: light::LightUniform,
    light_uniform_buffer: wgpu::Buffer,
    pub global_bind_group_layout: wgpu::BindGroupLayout,
    pub global_bind_group: wgpu::BindGroup,
    pub render_pipeline: wgpu::RenderPipeline,
    pub instance_buffers: HashMap<usize, wgpu::Buffer>,
}

impl BasicPass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        camera: &Camera,
        // TODO: Lights could be passed in here instead like the camera
    ) -> Self {
        let basic_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: Some("Basic Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/basic.wgsl").into()),
        });

        let global_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Basic Globals Layout"),
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
        let light_uniform = light::LightUniform {
            position: [10.0, 10.0, 10.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };
        let light_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Basic Light Buffer"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Basic Globals"),
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
        // Set up render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
            label: Some("Basic Render Pipeline Layout"),
            bind_group_layouts: &[&global_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { 
            label: Some("Basic Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &basic_shader,
                entry_point: "vs_main",
                buffers: &[vertex::ModelVertex::describe(), instance::RawInstance::describe()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &basic_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.view_formats[0],
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

        let instance_buffers = HashMap::new();

        Self {
            camera_uniform,
            camera_uniform_buffer,
            light_uniform,
            light_uniform_buffer,
            global_bind_group_layout,
            global_bind_group,
            render_pipeline,
            instance_buffers,
        }
    }
}

impl RenderPass<Object> for BasicPass {
    fn draw(
        &mut self,
        app_data: &AppData,
        view: &wgpu::TextureView,
        mut encoder: wgpu::CommandEncoder,
        objects: &Vec<Object>,
        depth_texture: Option<&Texture>,
        clear_color: &Option<wgpu::Color>,
    ) -> Result<wgpu::CommandEncoder, wgpu::SurfaceError> {
        // Create a `RenderPass` to render the frame

        app_data.queue.write_buffer(&self.camera_uniform_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
        app_data.queue.write_buffer(&self.light_uniform_buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Basic Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view, // Render to the view created above (the output texture)
                resolve_target: None, // The same as `view` unless multisampling is enabled
                ops: wgpu::Operations {
                    // Load tells wgpu what to do with colours stored from the previous frame
                    load: match clear_color { 
                        Some(clear_color) => { wgpu::LoadOp::Clear(*clear_color) },
                        _ => { wgpu::LoadOp::Load },
                    },
                    // Tells wgpu whether we want to store the rendered results to the `Texture` behind the `TextureView` in `view`
                    // In this case, that `Texture` is the `SurfaceTexture` and we do want to store the rendered results there
                    store: wgpu::StoreOp::Store, 
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture.unwrap().view,
                depth_ops: Some(wgpu::Operations {
                    load: match clear_color {
                        Some(_) => { wgpu::LoadOp::Clear(1.0) },
                        _ => { wgpu::LoadOp::Load },
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);

        for (object_idx, object) in objects.iter().enumerate() {
            let create_instance_buffer = || {
                let instance_data = object.instances.iter().map(instance::Instance::to_raw).collect::<Vec<_>>();
                app_data.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Basic Instance Buffer"),
                        contents: bytemuck::cast_slice(&instance_data),
                        usage: wgpu::BufferUsages::VERTEX,
                })
            };
            self.instance_buffers
                .entry(object_idx)
                .and_modify(|value| {*value = create_instance_buffer()})
                .or_insert_with(create_instance_buffer);
        }

        for (object_idx, object) in objects.iter().enumerate() {
            render_pass.set_vertex_buffer(1, self.instance_buffers[&object_idx].slice(..));
            render_pass.draw_light_model_instanced(
                &object.model,
                0..object.instances.len() as u32,
                &self.global_bind_group,
            );
        }
        
        drop(render_pass); // Need to drop `render_pass` to release the mutable borrow of `encoder` so we can call `encoder.finish()`
        Ok(encoder)
    }
}