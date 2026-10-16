# Architecture & Design Conventions

This document outlines the architectural conventions, design patterns, and engineering standards for the **Earth** project (`winit` + `wgpu`).

---

## 1. Core Architectural Pillars

The architecture follows idiomatic **Rust**, **WGPU**, and **real-time computer graphics** engine principles:

1. **Strict 4-Layer Hierarchy:** Unidirectional dependencies. Game logic never interacts with raw GPU handles directly, and GPU rendering never mutates simulation state.
2. **Frequency-Based Bind Group Convention:** Shaders organize descriptor sets/bind groups from least frequent update to most frequent update (`@group(0)` to `@group(3)`).
3. **The 3-Phase Frame Loop (*Extract $\to$ Prepare $\to$ Render*):** Clear boundary between simulation update, GPU data synchronization, and command encoding.
4. **Handle-Based Asset Management:** Meshes, textures, and materials are identified by lightweight, copyable handles rather than heavy heap structures embedded in scene entities.
5. **Frame-Rate Independent Simulation:** All movement and simulation advance via explicit delta time (`dt: Duration`), decoupled from swapchain presentation rate.

---

## 2. The 4-Layer Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      1. Application Layer                       │  (winit 0.30 lifecycle, input, time loop)
└────────────────┬───────────────────────────────┬────────────────┘
                 │                               │
                 ▼                               ▼
┌────────────────────────────────┐  ┌─────────────────────────────┐
│       2. Scene & World         │  │       3. Asset Server       │  (Handles, CPU geometry/images,
│   (Entities, Transforms,       │  │   (Meshes, Materials,       │   GPU resource caches)
│    Cameras, Lights, Actions)   │  │    Textures, Pipelines)     │
└────────────────┬───────────────┘  └────────────┬────────────────┘
                 │                               │
                 └───────────────┬───────────────┘
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                      4. Renderer Layer                          │  (Render passes, frame queues)
├─────────────────────────────────────────────────────────────────┤
│                     GpuContext & Surface                        │  (Device, Queue, Surface, Depth buffer)
└─────────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

| Layer | Module Path | Primary Responsibility | Owns |
| :--- | :--- | :--- | :--- |
| **1. Application** | `src/app/` | Window lifecycle, event loop dispatch, high-precision timing, raw hardware input capture. | `ActiveEventLoop`, `Window`, `Clock`, `InputState` |
| **2. Scene & World** | `src/scene/` | Simulation logic, camera calculations, light entities, spatial transforms (`TRS`). Agnostic of rendering specifics. | `Scene`, `Camera`, `Transform`, `Light` |
| **3. Asset Server** | `src/assets/` | File I/O, format decoding (OBJ/glTF/PNG), deduplication, CPU $\to$ GPU buffer allocation, handle generation. | `Handle<T>`, `MeshPool`, `TexturePool`, `MaterialPool` |
| **4. Renderer & GPU** | `src/gpu/`, `src/renderer/` | Command encoding, pipeline caches, bind group generation, swapchain presentation, pass execution. | `GpuContext`, `SurfaceManager`, `ForwardPass`, `PipelineCache` |

---

## 3. Graphics & WGPU Design Patterns

### 3.1 Bind Group Update Frequency Rule

To minimize GPU state changes and CPU command encoding overhead, bind groups must adhere to this standardized frequency layout across **all** shaders:

```
@group(0)  ->  Global / Frame Scope     (Updated once per frame: Camera matrices, Viewport, Time)
@group(1)  ->  Pass / Environment Scope  (Updated once per pass: Directional lights, Shadows, Environment maps)
@group(2)  ->  Material Scope           (Updated per material switch: Diffuse/Normal maps, Samplers, PBR factors)
@group(3)  ->  Object / Instance Scope   (Updated per draw/instance: Per-object transforms if not using instanced VBOs)
```

#### WGSL Example:
```wgsl
// Global / Frame scope
struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

// Pass / Environment scope
struct LightUniform {
    position: vec3<f32>,
    color: vec3<f32>,
};
@group(1) @binding(0) var<uniform> light: LightUniform;

// Material scope
@group(2) @binding(0) var t_diffuse: texture_2d<f32>;
@group(2) @binding(1) var s_diffuse: sampler;
```

---

### 3.2 The 3-Phase Frame Execution

Never call `write_buffer` or mutate gameplay data inside the render pass encoder.

```
       ┌────────────────────────┐
       │ 1. Update (Simulation) │  Advance state by dt; process input actions.
       └───────────┬────────────┘
                   │
                   ▼
       ┌────────────────────────┐
       │ 2. Extract & Prepare   │  Collect visible entities; upload dirty uniforms & instance VBOs.
       └───────────┬────────────┘
                   │
                   ▼
       ┌────────────────────────┐
       │ 3. Render (Presentation)│  Begin render pass; bind pipelines; execute draw queue; present.
       └────────────────────────┘
```

1. **Update:** `scene.update(dt, &input_state)` updates camera angles, light trajectories, and object transforms.
2. **Extract & Prepare:** 
   - Scene extracts an ordered draw queue (e.g., sorted opaque front-to-back, transparent back-to-front).
   - Write uniform buffers: `queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform))`.
   - Pack dynamic instance data into GPU instance buffers.
3. **Render:**
   - Acquire current surface texture view.
   - Execute registered passes (`ForwardPass`, `DebugLightPass`).
   - Submit command buffer and present frame.

---

### 3.3 Separation of `GpuContext` and `SurfaceManager`

Avoid bundling `wgpu::Device` with `wgpu::Surface`. Headless tests or background computations require a `Device` without a window surface:

* **`GpuContext`:** Owns `wgpu::Instance`, `wgpu::Adapter`, `wgpu::Device`, `wgpu::Queue`.
* **`SurfaceManager`:** Owns `wgpu::Surface`, `wgpu::SurfaceConfiguration`, and surface-dependent resources (such as `depth_texture`).

When a window resize occurs, only `SurfaceManager` and the camera aspect ratio are affected:

```rust
pub fn resize(&mut self, gpu: &GpuContext, width: u32, height: u32) {
    if width > 0 && height > 0 {
        self.surface_manager.resize(&gpu.device, width, height);
        self.scene.camera.set_aspect_ratio(width as f32 / height as f32);
    }
}
```

---

## 4. Scene & Asset Design Patterns

### 4.1 Handle-Based Asset Storage

Entities in the scene should not directly own `wgpu::Buffer` or `wgpu::Texture`. Instead, they reference asset handles:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshHandle(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialHandle(pub u32);

pub struct RenderEntity {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub transform: Transform,
}
```

An `AssetServer` or `ResourceRegistry` owns the GPU resources and resolves handles during the **Extract** phase:

```rust
pub struct AssetServer {
    meshes: Vec<GpuMesh>,
    materials: Vec<GpuMaterial>,
    textures: Vec<GpuTexture>,
}
```

---

### 4.2 Math Ecosystem: `glam` Standard

The project adopts **`glam`** (instead of `cgmath`):
* **Performance:** Direct SIMD vectorization.
* **Modern WebGPU Support:** `Mat4::perspective_rh(fovy, aspect, znear, zfar)` maps directly to WebGPU/DirectX depth range `[0.0, 1.0]` without requiring correction matrices (e.g., `OPENGL_TO_WGPU_MATRIX`).
* **Ergonomics:** Direct compatibility with `bytemuck` and standard Rust gamedev crates.

---

### 4.3 Input Decoupling

Decouple hardware event codes (`winit::keyboard::KeyCode`) from application intent:

1. **`InputState`:** Captures raw key down/up states and cursor delta during `WindowEvent`.
2. **`Action`:** Maps keys to semantic operations:
   ```rust
   pub enum Action {
       MoveForward,
       MoveBackward,
       MoveLeft,
       MoveRight,
       OrbitCamera,
   }
   ```
3. **Simulation:** Reads actions scaled by `dt`:
   ```rust
   if input.is_active(Action::MoveForward) {
       camera.translate_forward(speed * dt.as_secs_f32());
   }
   ```

---

## 5. Target Directory & Module Structure

```text
src/
├── main.rs                   # Executable entry point
├── lib.rs                    # Library root, exports run()
├── app.rs                    # Application & Platform Layer entry & lifecycle
├── app/                      # Application submodules
│   ├── input.rs              # Raw input tracking & virtual Action mapping
│   └── time.rs               # High-precision frame timing & delta calculations
├── gpu.rs                    # Graphics core facade
├── gpu/                      # Low-level graphics submodules
│   ├── context.rs            # GpuContext (Instance, Adapter, Device, Queue)
│   ├── surface.rs            # SurfaceManager (Surface, Config, DepthTexture)
│   ├── pipeline.rs           # Pipeline builder & layout utilities
│   └── texture.rs            # Texture, TextureView, and Sampler abstractions
├── scene.rs                  # Scene facade & container
├── scene/                    # Scene submodules
│   ├── camera.rs             # Camera, projection, and orbital controller (glam)
│   ├── transform.rs          # Translation, Rotation, Scale (TRS)
│   └── light.rs              # Light structs & GPU uniform representations
├── assets.rs                 # Asset facade
├── assets/                   # Asset submodules
│   ├── model.rs              # Model, Mesh, Material, and draw helpers
│   └── loader.rs             # Async OBJ/MTL, image, and binary loaders
├── renderer.rs               # Renderer facade & draw queue coordination
└── renderer/                 # Modular render passes
    └── passes/
        ├── forward.rs        # Main mesh forward-lighting pass
        └── light_viz.rs      # Light source indicator debug pass
```

---

## 6. Implementation Reference Skeletons

### 6.1 `GpuContext` & `SurfaceManager`

```rust
// src/gpu/context.rs
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

// src/gpu/surface.rs
pub struct SurfaceManager {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub depth_texture: Texture,
}

impl SurfaceManager {
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(device, &self.config);
            self.depth_texture = Texture::create_depth_texture(device, &self.config, "depth_texture");
        }
    }
}
```

### 6.2 Render Pass Abstraction

```rust
// src/renderer/passes/forward.rs
pub struct ForwardPass {
    pub pipeline: wgpu::RenderPipeline,
    pub global_layout: wgpu::BindGroupLayout,   // @group(0)
    pub light_layout: wgpu::BindGroupLayout,    // @group(1)
    pub material_layout: wgpu::BindGroupLayout, // @group(2)
}

impl ForwardPass {
    pub fn execute<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        global_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
        assets: &'a AssetServer,
        draw_queue: &'a [DrawCall],
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, global_bind_group, &[]);
        pass.set_bind_group(1, light_bind_group, &[]);

        for call in draw_queue {
            let material = assets.get_material(call.material_handle);
            let mesh = assets.get_mesh(call.mesh_handle);

            pass.set_bind_group(2, &material.bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, call.instance_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.num_elements, 0, call.instances.clone());
        }
    }
}
```

---

## 7. Migration Checklist

- [x] **Decouple `State`:** Extract `GpuContext` and `SurfaceManager` from `State`.
- [ ] **Standardize Bind Group Frequencies:** Refactor `@group(...)` indices across `shader.wgsl` and `light.wgsl`.
- [x] **Adopt `glam`:** Replace `cgmath` in `camera.rs` and `instance.rs` with `glam::Vec3`, `glam::Quat`, and `glam::Mat4`.
- [x] **Frame Timing (`dt`):** Integrate high-precision delta time in `App::window_event` (`RedrawRequested`).
- [x] **Input Abstraction:** Extract key handling into an `InputState` with an `Action` mapping enum.
- [ ] **Asset Handles:** Transition `Model` into reusable `GpuMesh` and `Material` entries inside an `AssetServer`.
- [ ] **Pass Separation:** Split main rendering into `ForwardPass` and `LightGizmoPass`.
