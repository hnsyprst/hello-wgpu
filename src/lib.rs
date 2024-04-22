#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

use wgpu::{util::DeviceExt, Color};

use winit::{
    event::{self, *},
    event_loop::{self, ControlFlow, EventLoop},
    window::{self, Window, WindowBuilder},
};

// When adding a field here, remember to add it's corresponding wgpu::VertexAttribute to vertex::describe()
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn describe() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress, // `array_stride` defines how wide a vertex is; when the shader goes to read the next vertex, it will skip over `array_stride` bytes
            step_mode: wgpu::VertexStepMode::Vertex, // `step_mode` tells the pipeline whether each element of the array represents per-vertex or per-instance data
            attributes: &[ // The attributes that make up a single vertex
                wgpu::VertexAttribute {
                    offset: 0, // defines the offset in bytes from the start of the struct until this attribute begins
                    shader_location: 0, // which location in the shader to store this attribute (in this case, @location(0))
                    format: wgpu::VertexFormat::Float32x3, // `format` tells the shader the shape of the attribute: `Float32x3` corresponds to `vec3<f32>`
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                }
            ]
        }
    }
}

// Make a triangle
const VERTICES: &[Vertex] = &[
    // Vertices arranged in counter-clockwise order
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.5, 0.0] },
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.0, 0.5, 0.5] },
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.0, 0.0, 0.5] },
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.0] },
];

const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];
 
fn create_render_pipeline_with_shader(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, shader: &wgpu::ShaderModule) -> wgpu::RenderPipeline {
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { 
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::describe()],
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
        depth_stencil: None,
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
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
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

        // Set up a default screen clear colour
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        // Set up the render pipeline
        let default_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let colourful_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: Some("Colourful Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("colourful_shader.wgsl").into()),
        });
        let render_pipelines = [
            create_render_pipeline_with_shader(&device, &config, &default_shader),
            create_render_pipeline_with_shader(&device, &config, &colourful_shader),
        ];
        let render_pipeline_index: usize = 0;

        // Set up the vertex and index buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        Self {
            surface,
            device,
            queue,
            config,
            size,
            clear_color,
            render_pipelines,
            render_pipeline_index,
            vertex_buffer,
            index_buffer,
            num_indices,
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
            _ => false,
        }
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
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(&self.render_pipelines[self.render_pipeline_index]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1); // Draw 3 vertices and 1 instance
        drop(render_pass); // Need to drop `render_pass` to release the mutable borrow of `encoder` so we can call `encoder.finish()`

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