#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

use winit::{
    event::{self, *},
    event_loop::{self, ControlFlow, EventLoop},
    window::{self, WindowBuilder},
};

use winit::window::Window;

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
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

        Self {
            surface,
            device,
            queue,
            config,
            size,
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
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        // Returns a bool to indicate whether `event` has been fully processed.
        // May be used to instruct an event loop to not process `event` any further.
        false
    }

    fn update(&mut self) {
        // todo!()
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
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        drop(render_pass); // Need to drop `render_pass` to release the mutable borrow of `encoder` so we can call `encoder.finish()`

        // `Queue.submit()` will accept anything that implements `IntoIter``, so we wrap `encoder.finish()` up in `std::iter::once`
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
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Create window in DOM if targeting wasm
    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));
        
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

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
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged{ new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
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
        });
}