use std::sync::Arc;

use cube_settings_gui::CubeSettingsEvent;
use hello_wgpu::model::Material;
use hello_wgpu::model::Model;
use hello_wgpu::primitives;
use hello_wgpu::primitives::Meshable;
#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch="wasm32")]
use wasm_bindgen_futures::js_sys::Math::random;

mod cube_settings_gui;

use hello_wgpu::{
    app,
    camera,
    texture,
    render_pass,
    object,
    resources,
    instance,
    gui,
};
use gui::windows::{performance::PerformanceEvent, stats::StatsEvent};
use instance::Instance;
use render_pass::RenderPass;

use cgmath::prelude::*;
use cgmath::Rotation3;
use winit::{event::WindowEvent, event_loop::EventLoop};

struct State {
    phong_pass: render_pass::phong::PhongPass,
    phong_objects: Vec<object::Object>,
    // wireframe_pass: render_pass::wireframe::WireframePass,
    // wireframe_objects: Vec<object::Object>,
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
            cgmath::Point3::new(0.0, 10.0, 20.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::unit_y(),
            app_data.config.width as f32 / app_data.config.height as f32,
            45.0,
            0.1,
            100.0,
        );
        
        let camera_controller = camera::CameraController::new(0.2);

        let phong_pass = render_pass::phong::PhongPass::new(&app_data.device, &app_data.queue, &app_data.config, &camera);
        // let wireframe_pass = render_pass::wireframe::WireframePass::new(&app_data.device, &app_data.queue, &app_data.config, &camera);

        // Load models
        let cube_model = resources::load_model("cube.obj", &app_data.device, &app_data.queue, &phong_pass.texture_bind_group_layout, Some(env!("OUT_DIR"))).await.unwrap();

        // Set up instances for phong pass
        const SPACE_BETWEEN: f32 = 5.0;
        let cube_instances = (0..2).map(|x| {
            let position = cgmath::Vector3 { x: x as f32 * SPACE_BETWEEN + 1.0 , y: 0.0, z: x as f32 * SPACE_BETWEEN + 1.0 };
            let rotation = cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0));
            Instance { position, rotation, rotation_speed: 0.0 }
        }).collect::<Vec<_>>();
        
        let wireframe_object = {
            let model = Model {
                meshes: vec![
                    primitives::Cuboid::new(
                        "cube",
                        cgmath::Vector3 { x: 3.0, y: 3.0, z: 3.0 }
                    ).build_mesh(&app_data.device),
                ],
                materials: vec![
                    Material::default(
                        &app_data.device,
                        &app_data.queue,
                        &phong_pass.texture_bind_group_layout,
                    )],
            };
            let instance = Instance { position: cgmath::Vector3 { x: 5.0, y: 5.0, z: 1.0 }, rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)), rotation_speed: 0.0 };
            object::Object{ model, instances: vec![instance] }
        };
        let phong_objects = vec![
            object::Object{ model: cube_model, instances: cube_instances },
            wireframe_object,
        ];
        let depth_texture = texture::Texture::create_depth_texture(&app_data.device, &app_data.config, "Depth Texture");

        app_data.egui_renderer.add_gui_window("performance", Box::new(gui::windows::performance::PerformanceWindow::new()));
        app_data.egui_renderer.add_gui_window("stats", Box::new(gui::windows::stats::StatsWindow::new()));
        app_data.egui_renderer.add_gui_window("cube_settings", Box::new(cube_settings_gui::CubeSettingsWindow::new(phong_objects[0].instances[0].position)));
        
        Self {
            phong_pass,
            phong_objects,
            // wireframe_pass,
            // wireframe_objects,
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
    if let Some(cube_settings_event) = app_data.egui_renderer.receive_event("cube_settings").downcast_ref::<CubeSettingsEvent>() {
        let position = cube_settings_event.position;
        state.phong_objects[0].instances[0] = Instance {
            position,
            rotation: state.phong_objects[0].instances[0].rotation,
            rotation_speed: state.phong_objects[0].instances[0].rotation_speed,
        }
    };

    // Move camera
    state.camera_controller.update_camera(&mut state.camera);
    state.phong_pass.camera_uniform.update_view_proj(&state.camera);

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
