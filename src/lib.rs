use egui_wgpu::renderer::ScreenDescriptor;
#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch="wasm32")]
use wasm_bindgen_futures::js_sys::Math::random;

mod camera;
mod texture;
mod model;
mod resources;
mod gui;

use model::Vertex;

use cgmath::prelude::*;
use egui::{Align2, Context};
use wgpu::{util::DeviceExt, Color, CommandEncoder};
use winit::{
    event::{self, *},
    event_loop::{self, ControlFlow, EventLoop},
    window::{self, Window, WindowBuilder},
};
use rand::Rng;

#[derive(serde::Deserialize, Debug)]
struct Song {
    path: String,
    tagged_genre: String,
    x: f32,
    y: f32,
    z: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RawInstance {
    model: [[f32; 4]; 4],
    normal: [[f32; 3]; 3],
}

impl RawInstance {
    fn describe() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<RawInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // model vec4s
                // A mat4x4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // normal vec3s
                // See above comment about mat4x4s, we'll have a similar situation in the shader here with mat3x3/vec3s.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location:11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
            }
    }
}

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    rotation_speed: f32,
}

impl Instance {
    fn to_raw(&self) -> RawInstance {
        RawInstance {
            model: (cgmath:: Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
            normal: cgmath::Matrix3::from(self.rotation).into(),
        }
    }
}


// TODO: Might be better to impl a constructor for uniforms in future so user doesn't have to think about stupid padding (cool)
// might also make it easier to forget about stupid padding (sad)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding: u32,
    color: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding2: u32,
}

// TODO: Add labels
fn create_render_pipeline_with_shader(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    shader: &wgpu::ShaderModule,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    bind_group_layouts: &[&wgpu::BindGroupLayout]
) -> wgpu::RenderPipeline {
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: bind_group_layouts,
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { 
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
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
            format: texture::Texture::DEPTH_FORMAT,
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
    })
}

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    clear_color: wgpu::Color,
    render_pipelines: [wgpu::RenderPipeline; 2],
    render_pipeline_index: usize,
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: texture::Texture,
    camera: camera::Camera,
    camera_uniform: camera::CameraUniform,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: camera::CameraController,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    obj_model: model::Model,
    light_uniform: LightUniform,
    light_uniform_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,
    egui_renderer: gui::EguiRenderer,
    window: Window,
}

impl State {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // The `instance` is a handle to our GPU. Its main purpose is to create `Adapter`s and `Surface`s.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(), // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
            ..Default::default()
        });
        
        // The `surface` is the part of the window that we are drawing to.
        let surface = unsafe { instance.create_surface(&window) }.unwrap(); // The surface needs to live as long as the window that created it. State owns the window, so this should be safe.
        
        // The `adapter` is a handle for our actual graphics card. We need it to create the `Device` and `Queue`.
        // `Adapter`s are locked to a specific backend (i.e., if you have two GPUs on windows you'll have 4 `Adapters` to chose from: 2 Vulkan and 2 DirectX).
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(), // `LowPower` will pick an adapter that favors battery life, such as an integrated GPU. 
                                                                    // `HighPerformance` will pick an adapter for more power-hungry yet more performant GPU's, such as a dedicated graphics card.
                                                                    // `default` will pick the first available adapter.
                compatible_surface: Some(&surface), // Find an adapter compatible with the supplied surface.
                force_fallback_adapter: false, // Forces the instance to pick an adapter compatible with all hardware (typically forces a "software" rendering backend for instead of using GPU hardware)
            },
        ).await.unwrap();

        // The `device` is responsible for the creation of most rendering and compute resources. These are used in commands passed to the `queue`.
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(), // Enable features not guaranteed to be supported. See docs for full list
                limits: if cfg!(target_arch = "wasm32") { // Describes the limits an adapter/device supports. Recommended to start with the most resticted limits and and manually increase to stay running on all hardware that supports the limits needed
                    wgpu::Limits::downlevel_webgl2_defaults() // Worth playing with this now that WebGPU is supported in Chrome---Limits::default() is guaranteed to support WebGPU
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None,
        ).await.unwrap();

        // Setting up `config`` defining how the surface creates `SurfaceTexture`s
        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0], // Play with this or enable runtime selection. PresentMode::Fifo is guaranteed to be supported on all platforms and is essentially VSync
                                                                 // For runtime selection, `let modes = &surface_caps.present_modes;` will get a list of all `PresentMode`s supported by the surface
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // Create a texture and load it
        let diffuse_bytes = include_bytes!("tex.png");
        let diffuse_texture = texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "tex.png").unwrap(); // TODO: Use a default texture with `unwrap_or` or some other `Err` handling
        
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { 
            label: Some("texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ]
        });
        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        // Load model
        let obj_model = resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout).await.unwrap();

        // Set up camera
        let camera = camera::Camera::new(
            cgmath::Point3::new(0.0, 1.0, 2.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::unit_y(),
            config.width as f32 / config.height as f32,
            45.0,
            0.1,
            100.0,
        );

        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        let camera_controller = camera::CameraController::new(0.2);

        // Set up a default screen clear colour
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        // Set up instance buffer
        // Note: if new instances are added at runtime, both `instance_buffer` and `camera_bind_group` must be recreated
        let songs: Vec<Song> = resources::load_json::<Song>("coords.json").await.unwrap();

        const SPACE_BETWEEN: f32 = 5.0;
        let mut rng = rand::thread_rng();
        let instances = songs.iter().map(|song| {
            // let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
            // let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

            let position = cgmath::Vector3 { x: song.x * SPACE_BETWEEN, y: song.y * SPACE_BETWEEN, z: song.z * SPACE_BETWEEN };
            let rotation = if position.is_zero() {
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            } else {
                cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
            };
            let rotation_speed: f32 = rng.gen_range(-0.5..0.5);
            Instance { position, rotation, rotation_speed }
        }).collect::<Vec<_>>();
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Insatance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
        });

        // Set up lighting
        let light_uniform = LightUniform {
            position: [10.0, 10.0, 10.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };
        let light_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light Buffer"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let light_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        });
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_uniform_buffer.as_entire_binding(),
            }],
            label: None,
        });

        // Set up the GUI
        let mut egui_renderer = gui::EguiRenderer::new(
            &device,
            config.format,
            None,
            1,
            &window,
        );

        // Set up the render pipeline
        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");
        let vertex_layouts = [model::ModelVertex::describe(), RawInstance::describe()];

        let render_pipeline_index: usize = 0;
        let render_pipelines = {
            let bind_group_layouts = [&texture_bind_group_layout, &camera_bind_group_layout, &light_bind_group_layout];
            let default_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });
            let colourful_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
                label: Some("Colourful Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("colourful_shader.wgsl").into()),
            });
            [
                create_render_pipeline_with_shader(&device, &config, &default_shader, &vertex_layouts, &bind_group_layouts),
                create_render_pipeline_with_shader(&device, &config, &colourful_shader, &vertex_layouts, &bind_group_layouts),
            ]
        };
        let light_render_pipeline = {
            let light_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("light_shader.wgsl").into()),
            });
            create_render_pipeline_with_shader(&device, &config, &light_shader, &[model::ModelVertex::describe()], &[&camera_bind_group_layout, &light_bind_group_layout])
        };

        Self {
            surface,
            device,
            queue,
            config,
            size,
            clear_color,
            render_pipelines,
            render_pipeline_index,
            diffuse_bind_group,
            diffuse_texture,
            camera,
            camera_uniform,
            camera_uniform_buffer,
            camera_bind_group,
            camera_controller,
            instances,
            instance_buffer,
            depth_texture,
            obj_model,
            light_uniform,
            light_uniform_buffer,
            light_bind_group,
            light_render_pipeline,
            egui_renderer,
            window,
        }
    }

    pub fn window(&self) -> &Window {
        // Get a reference to `self.window`.
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // Configures `self.surface` to match `new_size`.
        if new_size.width > 0 && new_size.height > 0 { // height or width being 0 may cause crashes
            // self.window.set_inner_size(new_size);
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
        self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        self.egui_renderer.handle_input(&event); // FIXME: This is being called every input (not great!) but im out of time atm
        // Returns a bool to indicate whether `event` has been fully processed.
        // May be used to instruct an event loop to not process `event` any further.
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.clear_color = wgpu::Color {
                    r: position.x / self.size.width as f64,
                    g: position.y / self.size.height as f64,
                    b: 1.0,
                    a: 1.0,
                };
                true
            },
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(VirtualKeyCode::Space),
                    ..
                },
                ..
            } => {
                self.render_pipeline_index = (self.render_pipeline_index + 1) % self.render_pipelines.len();
                true
            }
            _ => {
                self.camera_controller.process_events(event)
            }
        }
    }

    fn update(&mut self) {
        // Move instances
        self.instances = self.instances.iter().map(|instance| {
            let position = instance.position;
            let rotation_speed = instance.rotation_speed;
            let rotation = instance.rotation * cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(rotation_speed));
            Instance { position, rotation, rotation_speed }

        }).collect::<Vec<_>>();
        let instance_data = self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        self.instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Insatance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
        });
        // Move lights
        let light_position: cgmath::Vector3<_> = self.light_uniform.position.into();
        self.light_uniform.position = (
            cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(0.1)) * light_position
        ).into();
        self.queue.write_buffer(&self.light_uniform_buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));
        // Move camera
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        // Despite not explicitly using a staging buffer, this is still pretty performant (apparently) https://github.com/gfx-rs/wgpu/discussions/1438#discussioncomment-345473
        self.queue.write_buffer(&self.camera_uniform_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Get a frame to render to
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Create a `CommandEncoder` to create the store commands in a command buffer that will be sent to the GPU
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Create a `RenderPass` to clear and render the frame
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view, // Render to the view created above (the output texture)
                resolve_target: None, // The same as `view` unless multisampling is enabled
                ops: wgpu::Operations {
                    // Load tells wgpu what to do with colours stored from the previous frame (here we're just clearing them to a specified colour)
                    load: wgpu::LoadOp::Clear(self.clear_color),
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
        
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        use model::DrawLight;
        render_pass.set_pipeline(&self.light_render_pipeline);
        render_pass.draw_light_model(
            &self.obj_model,
            &self.camera_bind_group,
            &self.light_bind_group,
        );

        use model::DrawModel;
        render_pass.set_pipeline(&self.render_pipelines[self.render_pipeline_index]);
        render_pass.draw_model_instanced(
            &self.obj_model,
            0..self.instances.len() as u32,
            &self.camera_bind_group,
            &self.light_bind_group,
        );
        drop(render_pass); // Need to drop `render_pass` to release the mutable borrow of `encoder` so we can call `encoder.finish()`
        
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        self.egui_renderer.draw(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.window,
            &view,
            screen_descriptor,
            |ui| {
                egui::Window::new("Controls")
                    // .vscroll(true)
                    .default_open(true)
                    .max_width(1000.0)
                    .max_height(800.0)
                    .default_width(800.0)
                    .resizable(true)
                    .anchor(Align2::CENTER_TOP, [0.0, 0.0])
                    .show(&ui, |mut ui| {
                        if ui.add(egui::Button::new("Click me")).clicked() {
                            println!("PRESSED")
                        }
                        ui.label("Slider");
                        ui.end_row();
                    });
            },
        );

        // `Queue.submit()` will accept anything that implements `IntoIter`, so we wrap `encoder.finish()` up in `std::iter::once`
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() {
    // Set up logging (send logs to the JS console if we're targeting wasm)
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }
    
    let event_loop = EventLoop::new();
    let mut builder = WindowBuilder::new();

    // Create window in DOM if targeting wasm
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
            use winit::platform::web::WindowBuilderExtWebSys;
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("canvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            let width = canvas.client_width();
            let height = canvas.client_height();
            builder = builder.with_inner_size(winit::dpi::PhysicalSize::new(width, height)).with_canvas(Some(canvas));
    }
    builder = builder.with_title("main-canvas");
    let window = builder.build(&event_loop).unwrap();

    let mut state =  State::new(window).await;

    event_loop.run(move |event, _, control_flow|
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => if !state.input(event) { 
                match event {
                    WindowEvent::CloseRequested | 
                    WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        ..
                    } => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(physical_size) => {
                        log::info!("Resized to {:?}", physical_size);
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged{ new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                };
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // The surface is lost, so we need to reconfigure the surface
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is OOM, so let's just quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // Anything else should be resolved by the next frame, so print an error and move on
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once unless we manually request it.
                state.window().request_redraw();
            }
            _ => {}
        }
    );
}