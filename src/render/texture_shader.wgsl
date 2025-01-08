struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Light {
    position: vec4<f32>,
    color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> light: Light;

struct Transform {
    matrix: mat4x4<f32>,
};

@group(2) @binding(0)
var<uniform> transform: Transform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = in.tex_coords;

    var pos = transform.matrix * vec4<f32>(in.position, 1.0);

    out.world_position = pos.xyz;
    out.clip_position = camera.view_proj * pos;

    return out;
}

@group(3) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(3) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}