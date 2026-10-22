struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) elevation: f32,
};

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    direction: vec3<f32>,
    color: vec3<f32>,
}

@group(1) @binding(0)
var<uniform> light: Light;

@group(2) @binding(0)
var texture_elevation: texture_2d<f32>;
@group(2) @binding(1)
var sampler_elevation: sampler;

@vertex
fn vs_main(
    vertex: VertexInput,
) -> VertexOutput {
    let unit_position = normalize(vertex.position);
    let uv = vertex.tex_coords;

    let elevation_sample = textureSampleLevel(texture_elevation, sampler_elevation, uv, 0.0);
    let elevation = elevation_sample.a; // alpha channel is elevation
    let height_scale = 0.03;
    let displaced_position = unit_position * (1.0 + elevation * height_scale);

    let normal = normalize(elevation_sample.rgb * 2.0 - vec3<f32>(1.0)); // rgb channels is world_normal

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(displaced_position, 1.0);
    out.world_position = displaced_position;
    out.world_normal = normal;
    out.tex_coords = vertex.tex_coords;
    out.elevation = elevation;
    return out;
}

@group(3) @binding(0)
var texture_diffuse: texture_2d<f32>;
@group(3) @binding(1)
var sampler_diffuse: sampler;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let diffuse_sample = textureSample(texture_diffuse, sampler_diffuse, vertex.tex_coords);

    let is_water = vertex.elevation <= 0.001;
    var object_color = diffuse_sample;
    if (is_water) {
        object_color = vec4<f32>(0.0, 0.1, 0.6, 1.0);
    }

    let ambient_strength = 0.2;
    let ambient_color = light.color * ambient_strength;

    let normal = normalize(vertex.world_normal);
    let light_dir = normalize(light.direction);
    let diffuse_strength = max(dot(normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    // Specular lighting (sun glint on water only)
    var specular_color = vec3<f32>(0.0, 0.0, 0.0);
    if (is_water && diffuse_strength > 0.0) {
        let view_dir = normalize(camera.view_pos.xyz - vertex.world_position);
        let half_dir = normalize(light_dir + view_dir);
        let spec_angle = max(dot(normal, half_dir), 0.0);
        let shininess = 64.0;
        let specular_strength = 0.8;
        let specular_factor = pow(spec_angle, shininess);
        let terminator_fade = smoothstep(0.0, 0.05, diffuse_strength);
        specular_color = light.color * (specular_factor * specular_strength * terminator_fade);
    }

    let result = (ambient_color + diffuse_color) * object_color.xyz + specular_color;

    return vec4<f32>(result, object_color.a);
}
