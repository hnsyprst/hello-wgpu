use std::sync::Arc;

use controls_gui::ColliderChoice;
use controls_gui::CollisionEvent;
use controls_gui::ControlsEvent;
use controls_gui::StateEvent;
use hello_wgpu::collision;
use hello_wgpu::collision::aabb;
use hello_wgpu::collision::aabb::AxisAlignedBoundingBoxCollider;
use hello_wgpu::collision::Collider;
use hello_wgpu::collision::ColliderEnum;
use hello_wgpu::debug::line::Line;
use hello_wgpu::debug::wireframe::Wireframe;
use hello_wgpu::model::Material;
use hello_wgpu::model::Model;
use hello_wgpu::object::Object;
use hello_wgpu::primitives;
use hello_wgpu::primitives::cuboid::Cuboid;
use hello_wgpu::primitives::sphere::Sphere;
use hello_wgpu::primitives::Meshable;
#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch="wasm32")]
use wasm_bindgen_futures::js_sys::Math::random;

mod controls_gui;

use hello_wgpu::{
    app,
    camera,
    debug,
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
    line_pass: render_pass::line::LinePass,
    objects: Vec<Object<Line>>,
    all_colliders: Vec<ColliderEnum>,
    depth_texture: texture::Texture,
    camera: camera::Camera,
    camera_controller: camera::CameraController,
    clear_color: Option<wgpu::Color>,
    moving_object_index: usize,
}

impl State {
    async fn new(
        app_data: &mut app::AppData,
    ) -> Self {
        // Set up camera
        let camera = camera::Camera::new(
            cgmath::Point3::new(0.0, 30.0, 30.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::unit_y(),
            app_data.config.width as f32 / app_data.config.height as f32,
            45.0,
            0.1,
            100.0,
        );
        
        let camera_controller = camera::CameraController::new(0.2);

        let line_pass = render_pass::line::LinePass::new(
            &app_data.device,
            &app_data.queue,
            &app_data.config,
            &camera,
        );

        // Set up instances for line pass
        let half_size = cgmath::Vector3::new(3.0, 3.0, 3.0);

        let cuboid = Cuboid::new("cube", half_size);
        let sphere = Sphere::new("sphere", 3.0, 16);

        let cuboid_wireframe = cuboid.to_wireframe("Cuboid Wireframe", &app_data.device);
        let sphere_wireframe = sphere.to_wireframe("Sphere Wireframe", &app_data.device);

        let mut objects = Vec::with_capacity(2);
        let mut all_colliders: Vec<ColliderEnum> = Vec::with_capacity(4);

        let instances = (0..2).flat_map(|i| {
            (0..2).map(move |j| {
                let position = cgmath::Vector3::new((i as f32 * 10.0) - 5.0, 1.0, (j as f32 * 10.0) - 5.0);
                let rotation = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0));
                Instance { position, rotation, rotation_speed: 0.0 }
            })
        }).collect::<Vec<_>>();

        for instance in instances[0..2].to_vec().iter() {
            all_colliders.push(ColliderEnum::AABB(collision::aabb::AxisAlignedBoundingBoxCollider::new(half_size, instance.position)));
        };
        for instance in instances[2..].to_vec().iter() {
            all_colliders.push(ColliderEnum::Sphere(collision::sphere::SphereCollider::new(half_size.x, instance.position)));
        };
        objects.push(
            Object { model: cuboid_wireframe, instances: instances[0..2].to_vec() },
        );
        objects.push(
            Object { model: sphere_wireframe, instances: instances[2..].to_vec() },
        );

        let depth_texture = texture::Texture::create_depth_texture(&app_data.device, &app_data.config, "Depth Texture");

        app_data.egui_renderer.add_gui_window("performance", Box::new(gui::windows::performance::PerformanceWindow::new()));
        app_data.egui_renderer.add_gui_window("stats", Box::new(gui::windows::stats::StatsWindow::new()));
        app_data.egui_renderer.add_gui_window("controls", Box::new(controls_gui::ControlsWindow::new(objects[0].instances[0].position)));
        let clear_color = Some(wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.5,
                a: 1.0,
        });

        let moving_object_index = 0;

        Self {
            line_pass,
            objects,
            all_colliders,
            depth_texture,
            camera,
            camera_controller,
            clear_color,
            moving_object_index,
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

fn check_for_collision(
    collider_index: usize,
    all_colliders: &Vec<ColliderEnum>,
) -> bool {
    let (left, right) = all_colliders.split_at(collider_index);
    let (collider, remaining) = right.split_first().unwrap();

    let collisions = left.iter().chain(remaining.iter()).map(|other_collider| {
        match other_collider {
            ColliderEnum::AABB(aabb) => aabb.is_colliding_with(collider),
            ColliderEnum::Sphere(sphere) => sphere.is_colliding_with(collider),
        }
    }).collect::<Vec<_>>();
    return collisions.into_iter().any(|collision| collision);
}

fn update(
    app_data: &mut app::AppData,
    state: &mut State,
) {
    // Move instances
    if let Some(controls_event) = app_data.egui_renderer.receive_event("controls").downcast_ref::<StateEvent>() {
        if state.moving_object_index != controls_event.moving_collider as usize {
            state.moving_object_index = controls_event.moving_collider as usize;
            app_data.egui_renderer.send_event(
                "controls", 
                &ControlsEvent { 
                    position: state.objects[state.moving_object_index].instances[0].position,
                 },
            );
        }
        else {
            let position = controls_event.position;
            state.objects[state.moving_object_index].instances[0] = Instance {
                position,
                rotation: state.objects[state.moving_object_index].instances[0].rotation,
                rotation_speed: state.objects[state.moving_object_index].instances[0].rotation_speed,
            }
        }
    };

    let all_instances = state.objects
    .iter()
    .flat_map(|object| {
        &object.instances
    }).collect::<Vec<_>>();
    
    let half_size = cgmath::Vector3 { x: 3.0, y: 3.0, z: 3.0 };
    all_instances
        .iter()
        .zip(&mut state.all_colliders)
        .for_each(|(instance, collider)| {
            let position = instance.position;
            match collider {
                ColliderEnum::AABB(aabb) => aabb.update(half_size, position),
                ColliderEnum::Sphere(sphere) => sphere.update(half_size.x, position),
            }
        });

    // Move camera
    state.camera_controller.update_camera(&mut state.camera);
    state.line_pass.camera_uniform.update_view_proj(&state.camera);

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
            num_instances: state.objects.iter().map(|object| object.instances.len() as u32).sum()
        }
    );

    let is_colliding = check_for_collision(
        state.moving_object_index * 2, // There are 2 colliders / instances for each collider type, always just get the first one from each group
        &state.all_colliders,
    );
        
    app_data.egui_renderer.send_event(
        "controls", 
        &CollisionEvent { is_colliding },
    );
}

fn render(
    app_data: &mut app::AppData,
    state: &mut State,
    view: wgpu::TextureView,
    mut encoder: wgpu::CommandEncoder,
) {
    encoder = state.line_pass.draw(
        app_data,
        &view,
        encoder,
        &state.objects,
        Some(&state.depth_texture),
        &state.clear_color,
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
