struct InputVertex {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct OutputVertex {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(in: InputVertex) -> OutputVertex {
    var out: OutputVertex;
    var clip = camera.view_proj * vec4<f32>(in.position, 1.0);

    // Make sure gizmos are rendered on top
    clip.z -= 0.0001 * clip.w;
    out.clip_position = clip;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(vertex: OutputVertex) -> @location(0) vec4<f32> {
    return vertex.color;
}
