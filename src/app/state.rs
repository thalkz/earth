use crate::app::InputState;
use crate::gpu::{GpuContext, SurfaceHandler};
use crate::renderer::{RenderScene, Renderer};
use crate::world::{Scene, SceneConfig};
use std::sync::Arc;
use std::time::{Duration, Instant};
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
        let _span = tracing::info_span!("State::new").entered();
        let start = Instant::now();
        tracing::info!("Initializing State");

        let (gpu, surface) = {
            let _span = tracing::info_span!("init_gpu_and_surface").entered();
            GpuContext::init_window(window.clone()).await?
        };
        tracing::info!("Window initialized at {}ms", start.elapsed().as_millis());

        let renderer = {
            let _span = tracing::info_span!("Renderer::new").entered();
            Renderer::new(&gpu.device, surface.config.format)
        };

        let scene_config = SceneConfig {
            width: surface.config.width,
            height: surface.config.height,
            enable_gizmos: true,
        };

        let scene = {
            let _span = tracing::info_span!("Scene::load").entered();
            Scene::load(scene_config).await?
        };
        tracing::info!("Scene loaded at {}ms", start.elapsed().as_millis());

        let render_scene = {
            let _span = tracing::info_span!("RenderScene::new").entered();
            RenderScene::new(
                &gpu.device,
                &gpu.queue,
                &scene,
                &renderer.bind_group_layouts,
            )
        };
        tracing::info!("RenderScene created at {}ms", start.elapsed().as_millis());

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
        let _span = tracing::trace_span!("State::resize", width, height).entered();
        self.surface.resize(&self.gpu.device, width, height);
        self.scene.resize(width, height);
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let _span = tracing::trace_span!("State::render").entered();
        self.render_scene
            .update(&self.gpu.device, &self.gpu.queue, &self.scene);
        self.renderer
            .render(&self.gpu, &mut self.surface, &self.render_scene)?;
        self.window.request_redraw();
        Ok(())
    }

    pub fn update(&mut self, dt: Duration, input: &InputState) {
        let _span = tracing::trace_span!("State::update").entered();
        self.scene.update(dt, input);
    }
}
