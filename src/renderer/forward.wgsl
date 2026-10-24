struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) sphere_dir: vec3<f32>,
    @location(1) world_position: vec3<f32>,
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
var texture_elevation: texture_cube<f32>;
@group(2) @binding(1)
var sampler_elevation: sampler;

@vertex
fn vs_main(
    vertex: VertexInput,
) -> VertexOutput {
    let base_radius = length(vertex.position);
    let unit_position = vertex.position / base_radius;
    let camera_dist = length(camera.view_pos.xyz - unit_position);

    let min_dist = 1.0;
    let max_dist = 2.0;
    let t = clamp((camera_dist - min_dist) / (max_dist - min_dist), 0.0, 1.0);
    let target_lod = t * 6.0;

    let elevation_sample = textureSampleLevel(texture_elevation, sampler_elevation, unit_position, target_lod);

    let elevation = elevation_sample.a; // alpha channel is elevation
    let height_scale = 0.03;
    let displaced_position = unit_position * (base_radius + elevation * height_scale);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(displaced_position, 1.0);
    out.world_position = displaced_position;
    out.sphere_dir = unit_position;
    return out;
}

@group(3) @binding(0)
var texture_diffuse: texture_cube<f32>;
@group(3) @binding(1)
var sampler_diffuse: sampler;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(vertex.sphere_dir);
    let diffuse_sample = textureSample(texture_diffuse, sampler_diffuse, dir);
    let elevation_sample = textureSample(texture_elevation, sampler_elevation, dir);

    // Elevation alpha is 0.0 for water and >= 1/255 (~0.00392) for land.
    // Smoothstep across this 1-texel bilinear transition anti-aliases the coastline.
    let water_factor = 1.0 - smoothstep(0.0, 0.0039, elevation_sample.a);

    let ocean_color = vec3<f32>(0.0, 0.1, 0.6);
    let object_color = mix(diffuse_sample.rgb, ocean_color, water_factor);

    let ambient_strength = 0.2;
    let ambient_color = light.color * ambient_strength;

    let normal = normalize(elevation_sample.rgb * 2.0 - vec3<f32>(1.0));
    let light_dir = normalize(light.direction);
    let diffuse_strength = max(dot(normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    // Specular lighting (sun glint on water only, smoothly faded at coastlines)
    var specular_color = vec3<f32>(0.0, 0.0, 0.0);
    if (water_factor > 0.0 && diffuse_strength > 0.0) {
        let view_dir = normalize(camera.view_pos.xyz - vertex.world_position);
        let half_dir = normalize(light_dir + view_dir);
        let spec_angle = max(dot(normal, half_dir), 0.0);
        let shininess = 64.0;
        let specular_strength = 0.8;
        let specular_factor = pow(spec_angle, shininess);
        let terminator_fade = smoothstep(0.0, 0.05, diffuse_strength);
        specular_color = light.color * (specular_factor * specular_strength * terminator_fade * water_factor);
    }

    let result = (ambient_color + diffuse_color) * object_color + specular_color;

    return vec4<f32>(result, diffuse_sample.a);
}
