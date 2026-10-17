use crate::app::InputState;
use crate::assets::{self};
use crate::gpu::{GpuContext, SurfaceHandler};
use crate::renderer::Renderer;
use crate::scene::Scene;
use std::sync::Arc;
use std::time::Duration;
use winit::window::Window;

pub struct State {
    window: Arc<Window>,
    gpu: GpuContext,
    surface: SurfaceHandler,
    renderer: Renderer,
    scene: Scene,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let (gpu, surface) = GpuContext::init_window(window.clone()).await?;

        let mut renderer = Renderer::new(&gpu.device, surface.config.format);

        // let earth_model = assets::load_model("cube.obj").await?;
        let cube = assets::load_cube(4);

        let scene = Scene::new(surface.config.width, surface.config.height, cube);
        renderer.prepare(&gpu, &scene);

        Ok(Self {
            window,
            gpu,
            surface,
            renderer,
            scene,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(&self.gpu.device, width, height);
        self.scene.resize(width, height);
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();
        self.renderer.prepare(&self.gpu, &self.scene);
        self.renderer
            .render(&self.gpu, &mut self.surface, &self.scene)
    }

    pub fn update(&mut self, dt: Duration, input: &InputState) {
        self.scene.update(dt, &input.into());
    }
}
