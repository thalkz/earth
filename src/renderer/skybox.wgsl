struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) ray_dir: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle in NDC [-1, 1]^2 at the far plane (z = 1.0)
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index & 2u) * 2 - 1);

    let far_world = camera.inv_view_proj * vec4<f32>(x, y, 1.0, 1.0);
    let far_pos = far_world.xyz / far_world.w;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(x, y, 1.0, 1.0);
    out.ray_dir = far_pos - camera.view_pos.xyz;
    return out;
}

struct CubeCoord {
    face: u32,
    uv: vec2<f32>,
};

fn dir_to_cube(d: vec3<f32>) -> CubeCoord {
    let ad = abs(d);
    if (ad.x >= ad.y && ad.x >= ad.z) {
        let s = select(-1.0, 1.0, d.x >= 0.0);
        return CubeCoord(select(1u, 0u, d.x >= 0.0), vec2<f32>(d.z * s, d.y) / ad.x);
    } else if (ad.y >= ad.x && ad.y >= ad.z) {
        let s = select(-1.0, 1.0, d.y >= 0.0);
        return CubeCoord(select(3u, 2u, d.y >= 0.0), vec2<f32>(d.x, d.z * s) / ad.y);
    } else {
        let s = select(-1.0, 1.0, d.z >= 0.0);
        return CubeCoord(select(5u, 4u, d.z >= 0.0), vec2<f32>(d.x * s, d.y) / ad.z);
    }
}

fn cube_to_dir(face: u32, uv: vec2<f32>) -> vec3<f32> {
    switch (face) {
        case 0u: { return normalize(vec3<f32>( 1.0, uv.y,  uv.x)); }
        case 1u: { return normalize(vec3<f32>(-1.0, uv.y, -uv.x)); }
        case 2u: { return normalize(vec3<f32>(uv.x,  1.0,  uv.y)); }
        case 3u: { return normalize(vec3<f32>(uv.x, -1.0, -uv.y)); }
        case 4u: { return normalize(vec3<f32>( uv.x, uv.y,  1.0)); }
        default: { return normalize(vec3<f32>(-uv.x, uv.y, -1.0)); }
    }
}

fn hash3(p: vec3<u32>) -> vec3<f32> {
    var q = p * vec3<u32>(1597334673u, 3812015801u, 2798796415u);
    q = (q.x ^ q.y ^ q.z) * vec3<u32>(1597334673u, 3812015801u, 2798796415u);
    return vec3<f32>(q) * (1.0 / f32(0xffffffffu));
}

fn star_layer(
    dir: vec3<f32>,
    cube: CubeCoord,
    scale: f32,
    density: f32,
    pixel_angle: f32,
    layer_seed: u32,
) -> vec3<f32> {
    let grid_uv = (cube.uv * 0.5 + 0.5) * scale;
    let cell = floor(grid_uv);
    let cell_id = vec3<u32>(
        u32(i32(cell.x) + 4096),
        u32(i32(cell.y) + 4096),
        cube.face + layer_seed * 17u,
    );

    let rnd = hash3(cell_id);
    if (rnd.z > density) {
        return vec3<f32>(0.0);
    }

    // Place star center strictly inside [0.22, 0.78] of cell to prevent boundary clipping
    let star_grid_pos = cell + 0.22 + 0.56 * rnd.xy;
    let star_uv = (star_grid_pos / scale) * 2.0 - 1.0;
    let star_dir = cube_to_dir(cube.face, star_uv);

    let dist = length(dir - star_dir);
    let mag = fract(rnd.z / density);
    let brightness = pow(mag, 3.5);

    let radius = pixel_angle * (1.0 + 0.7 * mag);
    let falloff = smoothstep(radius, 0.0, dist);

    // Subtle stellar color variation (warm K/M to cool O/B stars)
    let temp = fract(rnd.x * 7.13);
    let warm_star = vec3<f32>(1.0, 0.80, 0.58);
    let cool_star = vec3<f32>(0.72, 0.86, 1.0);
    let star_color = mix(warm_star, cool_star, temp);

    return star_color * (falloff * falloff * brightness);
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(vertex.ray_dir);
    let pixel_angle = clamp(length(fwidth(dir)), 0.0004, 0.008);
    let cube = dir_to_cube(dir);

    // Layer 1: sparse primary stars
    let bright_stars = star_layer(dir, cube, 65.0, 0.12, pixel_angle * 1.15, 1u) * 0.45;
    // Layer 2: subtle background stars
    let faint_stars = star_layer(dir, cube, 140.0, 0.18, pixel_angle, 2u) * 0.15;

    let color = bright_stars + faint_stars;
    return vec4<f32>(color, 1.0);
}
