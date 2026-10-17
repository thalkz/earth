use crate::app::InputState;
use crate::assets;
use crate::gpu::{GpuContext, SurfaceHandler};
use crate::renderer::{RenderScene, Renderer};
use crate::scene::{Scene, SceneConfig};
use std::sync::Arc;
use std::time::Duration;
use winit::window::Window;

pub struct State {
    window: Arc<Window>,
    gpu: GpuContext,
    surface: SurfaceHandler,
    renderer: Renderer,
    scene: Scene,
    render_scene: RenderScene,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let (gpu, surface) = GpuContext::init_window(window.clone()).await?;

        let renderer = Renderer::new(&gpu.device, surface.config.format);

        // let earth_model = assets::load_model("cube.obj").await?;
        let cube = assets::load_cube(10);

        let scene_config = SceneConfig {
            width: surface.config.width,
            height: surface.config.height,
            enable_gizmos: true,
        };

        let scene = Scene::new(scene_config, cube);

        let render_scene = RenderScene::new(
            &gpu.device,
            &gpu.queue,
            &scene,
            &renderer.texture_bind_group_layout,
            &renderer.default_sampler,
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

    pub fn update(&mut self, dt: Duration, input: &InputState) {
        self.scene.update(dt, &input.into());
    }
}
