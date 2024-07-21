// Vertex shader

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};
// `group` refers to the index of the bind group passed to `render_pipeline_layout we want to access here 
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    let scale = 1.0;
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position * scale, 1.0);
    out.color = vec3<f32>(1.0, 1.0, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}