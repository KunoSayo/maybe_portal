struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

struct Light {
    color: vec3<f32>,
    width: f32,
    dir: vec3<f32>,
    height: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var<uniform> light: Light;

struct PlaneVertexIn {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct PlaneVertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
}

@vertex
fn plane_vs(input: PlaneVertexIn) -> PlaneVertexOut {
    var out: PlaneVertexOut;

    out.tex_coords = input.tex_coords;
    out.pos = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.normal = input.normal;

    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(2) @binding(0)
var t_depth: texture_depth_2d;



@fragment
fn portal_fs(in: PlaneVertexOut) -> @location(0) vec4<f32> {

    var pos = in.pos;



    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let portal_dep = textureLoad(t_depth, vec2<i32>(i32(pos.x), i32(pos.y)), 0);

    // make sure the things behind the portal
    if (pos.z <= portal_dep) {
        discard;
    }


    let ambient_color = vec3<f32>(1.0, 1.0, 1.0) * 0.25;
    let diffuse_strength = max(dot(in.normal, light.dir), 0.0) * 0.75;
    let diffuse_color = light.color * diffuse_strength;
    let result = vec4<f32>((ambient_color + diffuse_color) * object_color.rgb, object_color.a);

    return result;
}
