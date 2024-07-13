use std::sync::Arc;
use cgmath::Rotation3;
use gui::windows::{performance::PerformanceEvent, stats::StatsEvent};
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
mod render_pass;
mod object;

use instance::Instance;
use render_pass::RenderPass;

use cgmath::prelude::*;
use winit::{event::WindowEvent, event_loop::EventLoop};
use rand::Rng;

#[derive(serde::Deserialize, Debug)]
struct Song {
    path: String,
    tagged_genre: String,
    x: f32,
    y: f32,
    z: f32,
}

struct State {
    basic_pass: render_pass::basic::BasicPass,
    basic_objects: Vec<object::Object>,
    phong_pass: render_pass::phong::PhongPass,
    phong_objects: Vec<object::Object>,
    depth_texture: texture::Texture,
    camera: camera::Camera,
    camera_controller: camera::CameraController,
}

impl State {
    async fn new(
        app_data: &mut app::AppData,
    ) -> Self {
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
        
        let camera_controller = camera::CameraController::new(0.2);

        let basic_pass = render_pass::basic::BasicPass::new(&app_data.device, &app_data.queue, &app_data.config, &camera);
        let phong_pass = render_pass::phong::PhongPass::new(&app_data.device, &app_data.queue, &app_data.config, &camera);

        // Load models
        let light_model = resources::load_model("lightbulb_2.obj", &app_data.device, &app_data.queue, &phong_pass.texture_bind_group_layout).await.unwrap();
        let cube_model = resources::load_model("cube.obj", &app_data.device, &app_data.queue, &phong_pass.texture_bind_group_layout).await.unwrap();
        let ferris_model = resources::load_model("ferris.obj", &app_data.device, &app_data.queue, &phong_pass.texture_bind_group_layout).await.unwrap();
        
        // Set up instances for basic pass
        let light_instance = vec![{
            let position = cgmath::Vector3::from(basic_pass.light_uniform.position);
            let rotation = if position.is_zero() {
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            } else {
                cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
            };
            let rotation_speed: f32 = 0.0;
            Instance { position, rotation, rotation_speed }
        }];
        let basic_objects = vec![
            object::Object{ model: light_model, instances: light_instance },
        ];
        // Set up instances for phong pass
        // Note: if new instances are added at runtime, both `instance_buffer` and `camera_bind_group` must be recreated
        let songs: Vec<Song> = resources::load_json::<Song>("coords.json").await.unwrap();
        const SPACE_BETWEEN: f32 = 5.0;
        let mut rng = rand::thread_rng();
        let cube_instances = songs.iter().map(|song| {
            let position = cgmath::Vector3 { x: song.x * SPACE_BETWEEN, y: song.y * SPACE_BETWEEN, z: song.z * SPACE_BETWEEN };
            let rotation = if position.is_zero() {
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            } else {
                cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
            };
            let rotation_speed: f32 = rng.gen_range(-0.5..0.5);
            Instance { position, rotation, rotation_speed }
        }).collect::<Vec<_>>();
        let ferris_instance = vec![{
            let position = cgmath::Vector3 { x: 1.0, y: 1.0, z: 1.0 };
            let rotation = if position.is_zero() {
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            } else {
                cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
            };
            let rotation_speed: f32 = rng.gen_range(-0.5..0.5);
            Instance { position, rotation, rotation_speed }
        }];
        let phong_objects = vec![
            object::Object{ model: cube_model, instances: cube_instances },
            object::Object{ model: ferris_model, instances: ferris_instance },
        ];
        let depth_texture = texture::Texture::create_depth_texture(&app_data.device, &app_data.config, "Depth Texture");

        app_data.egui_renderer.add_gui_window("performance", Box::new(gui::windows::performance::PerformanceWindow::new()));
        app_data.egui_renderer.add_gui_window("stats", Box::new(gui::windows::stats::StatsWindow::new()));
        
        Self {
            basic_pass,
            basic_objects,
            phong_pass,
            phong_objects,
            depth_texture,
            camera,
            camera_controller,
        }
    }
}

fn window_event(
    app_data: &mut app::AppData,
    state: &mut State,
    window_event: &WindowEvent,
) {
    match window_event {
        _ => {
            state.camera_controller.process_events(window_event);
        }
    }
}

fn resize(
    app_data: &mut app::AppData,
    state: &mut State,
    size: (u32, u32),
) {
    state.depth_texture = texture::Texture::create_depth_texture(&app_data.device, &app_data.config, "depth_texture");
}

fn update(
    app_data: &mut app::AppData,
    state: &mut State,
) {
    // Move instances
    for object in state.phong_objects.iter_mut() {
        object.instances = object.instances.iter().map(|instance| {
            let position = instance.position;
            let rotation_speed = instance.rotation_speed;
            let rotation = instance.rotation * cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(rotation_speed));
            Instance { position, rotation, rotation_speed }
        }).collect::<Vec<_>>();
    }

    // Move lights
    let light_position: cgmath::Vector3<_> = state.phong_pass.light_uniform.position.into();
    state.phong_pass.light_uniform.position = (
        cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(5.0)) * light_position
    ).into();
    state.basic_pass.light_uniform.position = (
        cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(5.0)) * light_position
    ).into();

    // Move camera
    state.camera_controller.update_camera(&mut state.camera);
    state.phong_pass.camera_uniform.update_view_proj(&state.camera);
    state.basic_pass.camera_uniform.update_view_proj(&state.camera);

    // Update GUI
    app_data.egui_renderer.send_event(
        "performance", 
        &PerformanceEvent {
            fps: app_data.fps,
            render_time: app_data.render_time,
            update_time: app_data.update_time,
        }
    );
    app_data.egui_renderer.send_event(
        "stats", 
        &StatsEvent {
            num_instances: state.phong_objects.iter().map(|object| object.instances.len() as u32).sum()
        }
    );
}

fn render(
    app_data: &mut app::AppData,
    state: &mut State,
    view: wgpu::TextureView,
    mut encoder: wgpu::CommandEncoder,
) {
    encoder = state.phong_pass.draw(
        app_data,
        &view,
        encoder,
        &state.phong_objects,
        Some(&state.depth_texture),
    ).unwrap();
    encoder = state.basic_pass.draw(
        app_data,
        &view,
        encoder,
        &state.basic_objects,
        Some(&state.depth_texture),
    ).unwrap();

    app_data.egui_renderer.draw(
        &app_data.device,
        &app_data.queue,
        &mut encoder,
        &view,
    );

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
    
    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(app::create_window("cubes-app", &event_loop));
    let mut app_data = app::AppData::new(Arc::clone(&window)).await;
    let state = State::new(&mut app_data).await;
    let app = app::App::new(
        state,
        app_data,
        window_event,
        resize,
        update,
        render,
    ).await;
    app.run(window, event_loop);
}