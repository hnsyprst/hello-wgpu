use egui_wgpu::renderer::ScreenDescriptor;
use wgpu::RenderPipeline;
use winit::event::WindowEvent;
use winit::{
    event::{self, *},
    event_loop::{self, ControlFlow, EventLoop},
    window::{self, Window, WindowBuilder},
};

pub type WindowEventFn<T> = fn(app_data: &AppData, state: &mut T, window_event: &WindowEvent);
pub type ResizeFn<T> = fn(app_data: &AppData, state: &mut T, size: (u32, u32));
pub type UpdateFn<T> = fn(app_data: &AppData, state: &mut T);
pub type RenderFn<T> = fn(app_data: &AppData, state: &mut T, view: wgpu::TextureView, encoder: wgpu::CommandEncoder);

pub struct AppData {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    
    pub size: winit::dpi::PhysicalSize<u32>,

    pub surface: wgpu::Surface,
}

impl AppData {
    pub async fn new(
        window: &winit::window::Window
    ) -> Self {
        // The `instance` is a handle to our GPU. Its main purpose is to create `Adapter`s and `Surface`s.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(), // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
            ..Default::default()
        });
        
        // The `surface` is the part of the window that we are drawing to.
        let surface = unsafe { instance.create_surface(&window) }.unwrap(); // The surface needs to live as long as the window that created it!

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
        let size = window.inner_size();
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

        AppData {
            device,
            queue,
            config,
            size,
            surface,
        }
    }
}

pub struct App<T: 'static> {
    state: T, // A struct to hold application-specific state
    app_data: AppData, // Holds generic application state
    
    window_event_fn: WindowEventFn<T>, // Called on every WindowEvent other than CloseRequested and Resized
    resize_fn: ResizeFn<T>, // Called on every ScaleFactorChaanged and Resized WindowEvents
    update_fn: UpdateFn<T>, // Called before RenderFn every frame
    render_fn: RenderFn<T>, // Called after UpdateFn every frame
}

impl<T: 'static> App<T> {
    pub async fn new(
        state: T,
        app_data: AppData,
        window_event_fn: WindowEventFn<T>,
        resize_fn: ResizeFn<T>,
        update_fn: UpdateFn<T>,
        render_fn: RenderFn<T>,
    ) -> Self {
        App {
            state,
            app_data,
            window_event_fn,
            resize_fn,
            update_fn,
            render_fn,
        }
    }

    fn resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>
    ) {
        // Configures `self.surface` to match `new_size`.
        if new_size.width > 0 && new_size.height > 0 { // height or width being 0 may cause crashes
            return;
        }

        // self.window.set_inner_size(new_size);
        self.app_data.size = new_size;
        self.app_data.config.width = new_size.width;
        self.app_data.config.height = new_size.height;
        self.app_data.surface.configure(&self.app_data.device, &self.app_data.config);
        // TODO: Move this to the actual app's resize_fn
        // self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
    }

    fn render(
        &mut self,
    ) -> Result<(), wgpu::SurfaceError> {
        // Get a frame to render to
        let output = self.app_data.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Create a `CommandEncoder` to create the store commands in a command buffer that will be sent to the GPU
        let mut encoder = self.app_data.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        (self.render_fn)(&self.app_data, &mut self.state, view, encoder);

        output.present();

        Ok(())
    }

    pub fn run(
        mut self,
        window: winit::window::Window,
        event_loop: EventLoop<()>,
    ) {
        window.set_visible(true);

        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } => {
                if window_id != window.id() {
                    return;
                }
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(physical_size) => {
                        log::info!("Resized to {:?}", physical_size);
                        self.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged{ new_inner_size, .. } => {
                        self.resize(**new_inner_size);
                    }
                    _ => {
                        (self.window_event_fn)(&self.app_data, &mut self.state, event);
                    }
                };
            }
            Event::RedrawRequested(
                window_id,
            ) => {
                if window_id != window.id() {
                    return;
                }
                (self.update_fn)(&self.app_data, &mut self.state);
                match self.render() {
                    Ok(_) => {}
                    // The surface is lost, so we need to reconfigure the surface
                    Err(wgpu::SurfaceError::Lost) => self.resize(self.app_data.size),
                    // The system is OOM, so let's just quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // Anything else should be resolved by the next frame, so print an error and move on
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once unless we manually request it.
                window.request_redraw();
            }
            _ => {}
        });
    }
}

pub fn create_window(
    title: &str,
    event_loop: &EventLoop<()>,
) -> winit::window::Window {
    let mut builder = WindowBuilder::new();
    // (create window in DOM if targeting wasm)
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
    builder = builder.with_title(title);
    builder.build(event_loop).unwrap()
}

pub async fn run_app<T: 'static>(
    title: &str,
    state: T,
    window_event_fn: WindowEventFn<T>,
    resize_fn: ResizeFn<T>,
    update_fn: UpdateFn<T>,
    render_fn: RenderFn<T>,
){
    let event_loop = EventLoop::new();
    let window = create_window(title, &event_loop);

    let app_data = AppData::new(&window).await;

    let mut app = App::new(
        state,
        app_data,
        window_event_fn,
        resize_fn,
        update_fn,
        render_fn,
    ).await;
    app.run(window, event_loop);
}