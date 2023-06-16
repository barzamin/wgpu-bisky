use anyhow::Result;
use wgpu::{Instance, Adapter, Device, Queue, Limits, Features, DeviceDescriptor, RequestAdapterOptions, InstanceDescriptor, Backends, Surface};
use winit::window::Window;

pub struct RenderCtx {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl RenderCtx {
    pub async fn with_window(window: &Window) -> Result<(Self, Surface)> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::DX12 | Backends::VULKAN | Backends::METAL,
            dx12_shader_compiler: wgpu::Dx12Compiler::Dxc {
                dxil_path: None,
                dxc_path: None,
            }, // todo
        });

        let window_surface = unsafe { instance.create_surface(window) }?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&window_surface),
            })
            .await
            .expect("adapter");

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("device0"),
                    features: Features::empty(),
                    limits: Limits::default(),
                },
                None,
            )
            .await?;

        Ok((
            Self {
                instance,
                adapter,
                device,
                queue,
            },
            window_surface,
        ))
    }
}
