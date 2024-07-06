#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
    ambient: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Locals {
    position: [f32; 4],
    color: [f32; 4],
    normal: [f32; 4],
    lights: [f32; 4],
}