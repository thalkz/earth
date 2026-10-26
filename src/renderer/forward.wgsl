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
    inv_view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    direction: vec3<f32>,
    time: f32,
    color: vec3<f32>,
}

@group(1) @binding(0)
var<uniform> light: Light;

@group(2) @binding(0)
var texture_elevation: texture_cube<f32>;
@group(2) @binding(1)
var sampler_elevation: sampler;
@group(2) @binding(2)
var texture_ocean: texture_cube<f32>;
@group(2) @binding(3)
var texture_water_normal_1: texture_2d<f32>;
@group(2) @binding(4)
var texture_water_normal_2: texture_2d<f32>;
@group(2) @binding(5)
var sampler_water: sampler;

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

    // Alpha is 0.0 for water (clamped at elevation=0) and > 0.0 for land
    let elevation = elevation_sample.a;
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

fn sample_triplanar_normal(
    pos: vec3<f32>,
    sphere_normal: vec3<f32>,
    tex: texture_2d<f32>,
    samp: sampler,
    scale: f32,
    offset: vec2<f32>,
) -> vec3<f32> {
    let w = pow(abs(sphere_normal), vec3<f32>(4.0));
    let weights = w / (w.x + w.y + w.z);

    let uv_x = pos.zy * scale + offset;
    let uv_y = pos.xz * scale + offset;
    let uv_z = pos.xy * scale + offset;

    let t_x = textureSample(tex, samp, uv_x).xyz * 2.0 - 1.0;
    let t_y = textureSample(tex, samp, uv_y).xyz * 2.0 - 1.0;
    let t_z = textureSample(tex, samp, uv_z).xyz * 2.0 - 1.0;

    // Whiteout triplanar normal perturbation around sphere_normal
    let n_x = vec3<f32>(sphere_normal.x, t_x.y + sphere_normal.y, t_x.x + sphere_normal.z);
    let n_y = vec3<f32>(t_y.x + sphere_normal.x, sphere_normal.y, t_y.y + sphere_normal.z);
    let n_z = vec3<f32>(t_z.x + sphere_normal.x, t_z.y + sphere_normal.y, sphere_normal.z);

    return normalize(n_x * weights.x + n_y * weights.y + n_z * weights.z);
}

fn shade_land(
    dir: vec3<f32>,
    diffuse_rgb: vec3<f32>,
    elevation_sample: vec4<f32>,
    light_dir: vec3<f32>,
    ambient_color: vec3<f32>,
    direct_light_color: vec3<f32>,
) -> vec3<f32> {
    let tex_normal = normalize(elevation_sample.rgb * 2.0 - vec3<f32>(1.0));
    // Temper the baked normal tilt to match vertex height_scale (0.03) and prevent 89-degree horizontal saturation
    let normal = normalize(mix(dir, tex_normal, 0.35));
    let terminator_fade = smoothstep(-0.02, 0.08, dot(dir, light_dir));
    let diffuse_strength = max(dot(normal, light_dir), 0.0) * terminator_fade;
    let diffuse_color = direct_light_color * diffuse_strength;
    return (ambient_color + diffuse_color) * diffuse_rgb;
}

fn shade_water(
    dir: vec3<f32>,
    water_albedo: vec3<f32>,
    light_dir: vec3<f32>,
    view_dir: vec3<f32>,
    ambient_color: vec3<f32>,
    direct_light_color: vec3<f32>,
) -> vec3<f32> {
    // Two scrolling triplanar water normal layers at different scales and velocities
    let scale_1 = 14.5;
    let offset_1 = vec2<f32>(light.time * 0.008, light.time * 0.006);
    let wave_n1 = sample_triplanar_normal(dir, dir, texture_water_normal_1, sampler_water, scale_1, offset_1);

    let scale_2 = 17.8;
    let offset_2 = vec2<f32>(-light.time * 0.007, light.time * 0.009);
    let wave_n2 = sample_triplanar_normal(dir, dir, texture_water_normal_2, sampler_water, scale_2, offset_2);

    let combined_wave = normalize(wave_n1 + wave_n2);
    let wave_strength = 0.35;
    let water_normal = normalize(mix(dir, combined_wave, wave_strength));

    let terminator_fade = smoothstep(-0.02, 0.08, dot(dir, light_dir));
    let diffuse_strength = max(dot(water_normal, light_dir), 0.0) * terminator_fade;
    let diffuse_color = direct_light_color * diffuse_strength;

    // Specular highlight reflecting the sun on the perturbed water surface
    let half_dir = normalize(light_dir + view_dir);
    let spec_angle = max(dot(water_normal, half_dir), 0.0);
    let shininess = 148.0;
    let specular_strength = 0.85;
    let specular_factor = pow(spec_angle, shininess);
    let specular_color = direct_light_color * (specular_factor * specular_strength * terminator_fade);

    return (ambient_color + diffuse_color) * water_albedo + specular_color;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(vertex.sphere_dir);
    let diffuse_sample = textureSample(texture_diffuse, sampler_diffuse, dir);
    let elevation_sample = textureSample(texture_elevation, sampler_elevation, dir);
    let ocean_sample = textureSample(texture_ocean, sampler_elevation, dir);

    // Elevation alpha is 0.0 for water and >= 1/255 (~0.00392) for land.
    // Smoothstep across this 1-texel bilinear transition anti-aliases the coastline.
    let water_factor = 1.0 - smoothstep(0.0, 0.0039, elevation_sample.a);

    let ambient_strength = 0.05;
    let ambient_color = light.color * ambient_strength;
    let direct_light_color = light.color;

    let light_dir = normalize(light.direction);
    let view_dir = normalize(camera.view_pos.xyz - vertex.world_position);

    let land_color = shade_land(
        dir,
        diffuse_sample.rgb,
        elevation_sample,
        light_dir,
        ambient_color,
        direct_light_color,
    );
    let water_color = shade_water(
        dir,
        ocean_sample.rgb,
        light_dir,
        view_dir,
        ambient_color,
        direct_light_color,
    );

    let surface_color = mix(land_color, water_color, water_factor);

    // Atmospheric Fresnel rim glow:
    // Strictly 0 inside the inner disk (n_dot_v > 0.65), ramping up smoothly toward the horizon limb
    let sphere_normal = dir;
    let n_dot_l = dot(sphere_normal, light_dir);
    let n_dot_v = max(dot(sphere_normal, view_dir), 0.0);
    let rim = smoothstep(0.65, 0.0, n_dot_v);
    let fresnel = pow(rim, 2.5);
    let sun_rim = smoothstep(-0.08, 0.3, n_dot_l);
    let atmosphere_color = vec3<f32>(0.35, 0.70, 1.0);
    let rim_glow = atmosphere_color * (fresnel * sun_rim * 0.6);

    let result = surface_color + rim_glow;

    return vec4<f32>(result, diffuse_sample.a);
}
