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
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) camera_view_pos: vec4<f32>,
    @location(3) tex_coords: vec2<f32>,
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
    out.camera_view_pos = camera.view_pos;
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);

    return out;
}

@group(3) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(3) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ambient_color = light.color.xyz * light.color.a;

    let light_dir = normalize(light.position.xyz - in.world_position);

    let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
    let diffuse_color = light.color.xyz * diffuse_strength;

    // This would be lighting modeled after the Phong model only.
    //let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    //let reflect_dir = reflect(-light_dir, in.world_normal);
    //let specular_strength = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);

    // Blinn-Phong lighsting.
    let view_dir = normalize(in.camera_view_pos.xyz - in.world_position);
    let half_dir = normalize(view_dir + light_dir);
    let specular_strength = pow(max(dot(in.world_normal, half_dir), 0.0), 32.0);

    let specular_color = light.color.xyz * specular_strength;

    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let result = (ambient_color + diffuse_color + specular_color) * color.xyz;

    return vec4<f32>(result, color.a);
}