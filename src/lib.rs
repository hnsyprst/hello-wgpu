use cgmath::Rotation3;
#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch="wasm32")]
use wasm_bindgen_futures::js_sys::Math::random;

mod app;
mod camera;
mod gui;
mod instance;
mod light;
mod model;
mod resources;
mod texture;

use light::LightUniform;
use instance::Instance;
use model::{Model, Vertex};

use cgmath::prelude::*;
use egui::{Align2, Context};
use wgpu::{util::DeviceExt, Color, CommandEncoder, RenderPipeline, SurfaceError};
use winit::{
    event::{self, *},
    event_loop::{self, ControlFlow, EventLoop},
    window::{self, Window, WindowBuilder},
};
use rand::Rng;

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

#[derive(serde::Deserialize, Debug)]
struct Song {
    path: String,
    tagged_genre: String,
    x: f32,
    y: f32,
    z: f32,
}

struct State {
    texture_bind_group_layout: wgpu::BindGroupLayout,
    obj_model: model::Model,
    clear_color: wgpu::Color,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    light_uniform: LightUniform,
    light_uniform_buffer: wgpu::Buffer,
    light_bind_group_layout: wgpu::BindGroupLayout,
    light_bind_group: wgpu::BindGroup,
    camera: camera::Camera,
    camera_uniform: camera::CameraUniform,
    camera_uniform_buffer: wgpu::Buffer,
    camera_controller: camera::CameraController,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
}

impl State {
    async fn new(
        app_data: &app::AppData,
    ) -> Self {
        // // Set up the GUI
        // let mut egui_renderer = gui::EguiRenderer::new(
        //     &app_data.device,
        //     app_data.config.format,
        //     None,
        //     1,
        //     &window,
        // );
        
        // Set up a default screen clear colour
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let texture_bind_group_layout = app_data.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { 
            label: Some("texture_bind_group_layout"),
            entries: &[
                // Diffuse
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
                // Normal map
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ]
        });

        // Load model
        let obj_model = resources::load_model("cube.obj", &app_data.device, &app_data.queue, &texture_bind_group_layout).await.unwrap();

        // Set up instances
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
        let instance_buffer = app_data.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Insatance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
        });

        // Set up camera
        let camera = camera::Camera::new(
            cgmath::Point3::new(0.0, 1.0, 2.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::unit_y(),
            app_data.config.width as f32 / app_data.config.height as f32,
            45.0,
            0.1,
            100.0,
        );
        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&camera);
        let camera_uniform_buffer = app_data.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        let camera_controller = camera::CameraController::new(0.2);

        // Set up lighting
        let light_uniform = LightUniform {
            position: [10.0, 10.0, 10.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };
        let light_uniform_buffer = app_data.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light Buffer"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );


        // TODO: Move all of this, it belongs in a specific Phong module
        let camera_bind_group_layout = app_data.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let camera_bind_group = app_data.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });
        let light_bind_group_layout = app_data.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let light_bind_group = app_data.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_uniform_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let depth_texture = texture::Texture::create_depth_texture(&app_data.device, &app_data.config, "depth_texture");

        Self {
            texture_bind_group_layout,
            obj_model,
            clear_color,
            instances,
            instance_buffer,
            light_uniform,
            light_uniform_buffer,
            light_bind_group_layout,
            light_bind_group,
            camera,
            camera_uniform,
            camera_uniform_buffer,
            camera_bind_group_layout,
            camera_bind_group,
            camera_controller,
            depth_texture,
        }
    }
}

fn window_event(
    app_data: &app::AppData,
    state: &mut State,
    window_event: &WindowEvent,
) {
    match window_event {
        WindowEvent::CursorMoved { position, .. } => {
            state.clear_color = wgpu::Color {
                r: position.x / app_data.size.width as f64,
                g: position.y / app_data.size.height as f64,
                b: 1.0,
                a: 1.0,
            };
        },
        // WindowEvent::KeyboardInput {
        //     input: KeyboardInput {
        //         state: ElementState::Pressed,
        //         virtual_keycode: Some(VirtualKeyCode::Space),
        //         ..
        //     },
        //     ..
        // } => {
        //     state.render_pipeline_index = (state.render_pipeline_index + 1) % app_data.render_pipelines.len();
        // }
        _ => {
            state.camera_controller.process_events(window_event);
        }
    }
}

fn resize(
    app_data: &app::AppData,
    state: &mut State,
    size: (u32, u32),
) {
    state.depth_texture = texture::Texture::create_depth_texture(&app_data.device, &app_data.config, "depth_texture");
}

fn init(
    app_data: &app::AppData,
    state: &mut State,
) -> Vec<RenderPipeline> {
    let vertex_layouts = [model::ModelVertex::describe(), instance::RawInstance::describe()];
    let bind_group_layouts = [&state.texture_bind_group_layout, &state.camera_bind_group_layout, &state.light_bind_group_layout];
    
    let default_render_pipeline = {
        let default_shader = app_data.device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        create_render_pipeline_with_shader(
            &app_data.device,
            &app_data.config,
            &default_shader,
            &vertex_layouts,
            &bind_group_layouts,
        )
    };
    // let colourful_render_pipeline = {
    //     let colourful_shader = app_data.device.create_shader_module(wgpu::ShaderModuleDescriptor { 
    //         label: Some("Colourful Shader"),
    //         source: wgpu::ShaderSource::Wgsl(include_str!("colourful_shader.wgsl").into()),
    //     });
    //     create_render_pipeline_with_shader(
    //         &app_data.device,
    //         &app_data.config,
    //         &colourful_shader,
    //         &vertex_layouts,
    //         &bind_group_layouts,
    //     )
    // };
    let light_render_pipeline = {
        let light_shader = app_data.device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: Some("Light Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("light_shader.wgsl").into()),
        });
        create_render_pipeline_with_shader(
            &app_data.device,
            &app_data.config,
            &light_shader,
            &[model::ModelVertex::describe()],
            &[&state.camera_bind_group_layout, &state.light_bind_group_layout],
        )
    };

    vec![
        default_render_pipeline,
        light_render_pipeline,
    ]
}

fn update(
    app_data: &app::AppData,
    state: &mut State,
) {
    // Move instances
    state.instances = state.instances.iter().map(|instance| {
        let position = instance.position;
        let rotation_speed = instance.rotation_speed;
        let rotation = instance.rotation * cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(rotation_speed));
        Instance { position, rotation, rotation_speed }
    }).collect::<Vec<_>>();
    let instance_data = state.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
    state.instance_buffer = app_data.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Insatance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
    });

    // Move lights
    let light_position: cgmath::Vector3<_> = state.light_uniform.position.into();
    state.light_uniform.position = (
        cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(0.1)) * light_position
    ).into();
    app_data.queue.write_buffer(&state.light_uniform_buffer, 0, bytemuck::cast_slice(&[state.light_uniform]));
    // Move camera
    state.camera_controller.update_camera(&mut state.camera);
    state.camera_uniform.update_view_proj(&state.camera);
    // Despite not explicitly using a staging buffer, this is still pretty performant (apparently) https://github.com/gfx-rs/wgpu/discussions/1438#discussioncomment-345473
    app_data.queue.write_buffer(&state.camera_uniform_buffer, 0, bytemuck::cast_slice(&[state.camera_uniform]));
}

fn render(
    app_data: &app::AppData,
    state: &mut State,
    view: wgpu::TextureView,
    mut encoder: wgpu::CommandEncoder,
) {
    // Create a `RenderPass` to clear and render the frame
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view, // Render to the view created above (the output texture)
            resolve_target: None, // The same as `view` unless multisampling is enabled
            ops: wgpu::Operations {
                // Load tells wgpu what to do with colours stored from the previous frame (here we're just clearing them to a specified colour)
                load: wgpu::LoadOp::Clear(state.clear_color),
                // Tells wgpu whether we want to store the rendered results to the `Texture` behind the `TextureView` in `view`
                // In this case, that `Texture` is the `SurfaceTexture` and we do want to store the rendered results there
                store: wgpu::StoreOp::Store, 
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &state.depth_texture.view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        occlusion_query_set: None,
        timestamp_writes: None,
    });
    
    render_pass.set_vertex_buffer(1, state.instance_buffer.slice(..));

    // Draw models
    use model::DrawLight;
    render_pass.set_pipeline(&app_data.render_pipelines[1]);
    render_pass.draw_light_model(
        &state.obj_model,
        &state.camera_bind_group,
        &state.light_bind_group,
    );
    use model::DrawModel;
    render_pass.set_pipeline(&app_data.render_pipelines[0]);
    render_pass.draw_model_instanced(
        &state.obj_model,
        0..state.instances.len() as u32,
        &state.camera_bind_group,
        &state.light_bind_group,
    );
    drop(render_pass); // Need to drop `render_pass` to release the mutable borrow of `encoder` so we can call `encoder.finish()`
    
    // let screen_descriptor = ScreenDescriptor {
    //     size_in_pixels: [app_data.config.width, app_data.config.height],
    //     pixels_per_point: self.window.scale_factor() as f32, // FIXME
    // };

    // `Queue.submit()` will accept anything that implements `IntoIter`, so we wrap `encoder.finish()` up in `std::iter::once`
    app_data.queue.submit(std::iter::once(encoder.finish()));
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
    let window = app::create_window("cubes-app", &event_loop);
    let app_data = app::AppData::new(&window).await;
    let mut state = State::new(&app_data).await;
    let mut app = app::App::new(
        state,
        app_data,
        window_event,
        resize,
        update,
        render,
        init,
    ).await;
    app.run(window, event_loop);
}