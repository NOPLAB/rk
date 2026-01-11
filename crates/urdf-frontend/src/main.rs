//! URDF Editor main entry point

fn main() -> eframe::Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "urdf_frontend=debug,urdf_renderer=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting URDF Editor");

    // Configure wgpu
    // Use DX12 on Windows to avoid AMD Vulkan driver freeze issues
    // See: https://github.com/emilk/egui/issues/7718
    let wgpu_options = egui_wgpu::WgpuConfiguration {
        wgpu_setup: egui_wgpu::WgpuSetup::CreateNew {
            #[cfg(target_os = "windows")]
            supported_backends: wgpu::Backends::DX12,
            #[cfg(not(target_os = "windows"))]
            supported_backends: wgpu::Backends::all(),
            power_preference: wgpu::PowerPreference::default(),
            device_descriptor: std::sync::Arc::new(|adapter| wgpu::DeviceDescriptor {
                label: Some("rk device"),
                required_features: wgpu::Features::empty(),
                required_limits: adapter.limits(),
                memory_hints: wgpu::MemoryHints::default(),
            }),
        },
        ..Default::default()
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("URDF Editor"),
        wgpu_options,
        ..Default::default()
    };

    eframe::run_native(
        "rk",
        native_options,
        Box::new(|cc| Ok(Box::new(urdf_frontend::UrdfEditorApp::new(cc)))),
    )
}
