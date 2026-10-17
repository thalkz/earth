use crate::gpu::SurfaceHandler;
use std::sync::Arc;
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, ExperimentalFeatures, Features, Instance,
    InstanceDescriptor, Limits, PowerPreference, Queue, RequestAdapterOptions, Surface, Trace,
};
use winit::window::Window;

pub struct GpuContext {
    #[allow(dead_code)]
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl GpuContext {
    pub async fn new(
        instance: Instance,
        compatible_surface: Option<&Surface<'_>>,
    ) -> anyhow::Result<Self> {
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface,
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Primary Device"),
                required_features: Features::empty(),
                experimental_features: ExperimentalFeatures::disabled(),
                required_limits: Limits::default(),
                memory_hints: Default::default(),
                trace: Trace::Off,
            })
            .await?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    pub async fn init_window(window: Arc<Window>) -> anyhow::Result<(Self, SurfaceHandler)> {
        let size = window.inner_size();
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window)?;
        let gpu = Self::new(instance, Some(&surface)).await?;
        let surface_manager =
            SurfaceHandler::new(surface, &gpu.adapter, &gpu.device, size.width, size.height);

        Ok((gpu, surface_manager))
    }
}
