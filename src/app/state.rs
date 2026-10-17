use crate::app;
use crate::assets;
use crate::gpu;
use crate::renderer;
use crate::scene;
use std::sync::Arc;
use std::time;
use winit::window;

pub struct State {
    window: Arc<window::Window>,
    gpu: gpu::GpuContext,
    surface: gpu::SurfaceHandler,
    renderer: renderer::Renderer,
    scene: scene::Scene,
    render_scene: renderer::RenderScene,
}

impl State {
    pub async fn new(window: Arc<window::Window>) -> anyhow::Result<Self> {
        let (gpu, surface) = gpu::GpuContext::init_window(window.clone()).await?;

        let renderer = renderer::Renderer::new(&gpu.device, surface.config.format);

        // let earth_model = assets::load_model("cube.obj").await?;
        let cube = assets::load_cube(10);

        let scene_config = scene::SceneConfig {
            width: surface.config.width,
            height: surface.config.height,
            enable_gizmos: true,
        };

        let scene = scene::Scene::new(scene_config, cube);

        let render_scene = renderer::RenderScene::new(
            &gpu.device,
            &gpu.queue,
            &scene,
            &renderer.texture_bind_group_layout,
            &renderer.camera_bind_group_layout,
            &renderer.light_bind_group_layout,
        );

        Ok(Self {
            window,
            gpu,
            surface,
            renderer,
            scene,
            render_scene,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(&self.gpu.device, width, height);
        self.scene.resize(width, height);
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();
        self.render_scene
            .update(&self.gpu.device, &self.gpu.queue, &self.scene);
        self.renderer
            .render(&self.gpu, &mut self.surface, &self.render_scene)
    }

    pub fn update(&mut self, dt: time::Duration, input: &app::InputState) {
        self.scene.update(dt, &input.into());
    }
}
