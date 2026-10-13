# Architecture & Design Conventions

This document outlines the architectural conventions and best practices for the **Earth** project (`winit` + `wgpu`).

---

## 1. Game Loop Pattern: `update` vs `render`

The application loop separates simulation logic from graphics rendering.

```
                  ┌──────────────────────┐
                  │ Winit Event Loop     │
                  └──────────┬───────────┘
                             │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
   WindowEvent::KeyboardInput       WindowEvent::RedrawRequested
            │                                 │
   Record Input State                         ├──────────────────┐
                                              ▼                  ▼
                                           update()           render()
                                      (advance state)    (draw to surface)
```

### Roles and Responsibilities

| Method | Role | Responsibilities |
| :--- | :--- | :--- |
| **`update`** | **Simulation & State Evolution** | Advances simulation time, processes game/camera logic, handles physics, and writes updated uniform buffers to the GPU. |
| **`render`** | **Visual Presentation** | Obtains the swapchain surface texture, encodes render passes and draw commands, submits command buffers to the GPU queue, and presents the frame. |

### Why Separate Them?

1. **Decoupling Simulation Rate from Render Rate:**
   * Rendering speed varies across hardware (60Hz, 144Hz, 240Hz, VSync).
   * Simulation and physics require deterministic, fixed time steps to avoid instability or tunneling.
   * Decoupling allows running multiple physics steps if rendering lags, or interpolating visual state between frames.
2. **Side-Effect Free Presentation:**
   * `render` acts as an observer. It reads the current world state to draw it and avoids mutating gameplay data.
3. **Headless Execution & Testability:**
   * `update` can be invoked in unit tests or headless servers without needing a GPU, surface, or window system.

---

## 2. Key Architectural Guidelines

### 2.1 Decompose the `State` Struct

Avoid turning `State` into a "God Struct" that owns GPU primitives, scene entities, and input logic simultaneously.

#### Target Layering

```
┌──────────────────────────────────────────────┐
│                    App                       │  (winit event handling & lifecycle)
└───────┬──────────────────────────────┬───────┘
        ▼                              ▼
┌──────────────────────────┐    ┌─────────────────────────┐
│         Renderer         │    │          World          │
│ (device, queue, surface, │    │ (camera, scene entities,│
│  pipelines, bind layouts)│    │  transforms, lighting)  │
└──────────────────────────┘    └─────────────────────────┘
        │                              │
        └──────────────┬───────────────┘
                       ▼
            ┌─────────────────────┐
            │   Asset Manager     │
            │  (meshes, textures) │
            └─────────────────────┘
```

* **`Renderer` / `GpuContext`:** Owns low-level GPU objects (`wgpu::Device`, `wgpu::Queue`, `wgpu::Surface`, `wgpu::SurfaceConfiguration`). Knows *how to render*, but is agnostic of scene semantics.
* **`World` / `Scene`:** Owns entities, camera, and simulation state. Knows *what exists*.
* **`Assets`:** Caches and shares GPU textures, buffers, and materials.

---

### 2.2 Frame-Rate Independent Delta Time (`dt`)

Never update movement using constant per-frame increments (e.g. `self.speed = 0.2`). Movement speed must scale with elapsed time.

#### Implementation

Track frame times in the application state using `std::time::Instant`:

```rust
pub struct App {
    state: Option<State>,
    last_frame_time: std::time::Instant,
}
```

In `WindowEvent::RedrawRequested`:

```rust
let now = std::time::Instant::now();
let dt = now - self.last_frame_time;
self.last_frame_time = now;

state.update(dt);
state.render()?;
```

In the camera or movement controller:

```rust
pub fn update_camera(&self, camera: &mut Camera, dt: std::time::Duration) {
    let dt_secs = dt.as_secs_f32();
    let move_amount = self.speed * dt_secs; // self.speed in units/sec

    if self.is_forward_pressed {
        camera.eye += forward_norm * move_amount;
    }
}
```

---

### 2.3 Synchronize Window Resizes Across Subsystems

When the window resizes, multiple systems must update in unison:

1. **Surface Configuration:** Update width and height on `wgpu::SurfaceConfiguration`.
2. **Camera Projection:** Update aspect ratio (`width / height`) so rendered geometry does not stretch.
3. **Depth/Auxiliary Textures:** Recreate any resolution-dependent buffers (e.g. depth textures, post-processing targets).

```rust
pub fn resize(&mut self, width: u32, height: u32) {
    if width > 0 && height > 0 {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.camera.resize(width, height);
        self.is_surface_configured = true;
    }
}
```

---

### 2.4 Modular Geometry and Materials

Encapsulate vertex buffers, index buffers, and texture bindings into reusable abstractions rather than holding raw buffers directly in `State`.

#### `Mesh` Abstraction

```rust
pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

impl Mesh {
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_elements, 0, 0..1);
    }
}
```

#### `Material` Abstraction

Group pipeline bind groups (diffuse textures, samplers, normal maps) into a `Material` struct. This enables rendering multiple objects in a loop:

```rust
for entity in &self.entities {
    entity.material.bind(&mut render_pass);
    entity.mesh.draw(&mut render_pass);
}
```

---

### 2.5 Input Abstraction

Decouple hardware-specific event codes (`winit::keyboard::KeyCode`) from application behavior:

* **Input Collection:** Record raw keyboard and mouse state in an `Input` manager during `window_event`.
* **Action Mapping:** Map hardware keys to virtual actions (e.g., `Action::MoveForward`, `Action::RotateCamera`).
* **Input Consumption:** Query virtual actions inside `update(dt)`.

---

## 3. Architecture Checklist

- [ ] **Decoupled Update & Render:** `update(dt)` handles simulation; `render()` handles draw calls.
- [ ] **Delta Time Integration:** Movement scaled by elapsed duration (`dt`).
- [ ] **Resize Propagation:** Camera aspect ratio updated on window resize.
- [ ] **Layered Architecture:** Split `State` into `Renderer`, `Scene`, and `Asset` structures.
- [ ] **Reusable Assets:** Geometry encapsulated in `Mesh` and textures in `Material`.
- [ ] **Input Abstraction:** Virtual actions decoupled from raw `winit::keyboard::KeyCode`.
