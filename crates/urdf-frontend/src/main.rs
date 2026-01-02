//! URDF Editor main entry point

fn main() -> eframe::Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "urdf_frontend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting URDF Editor");

    // Configure wgpu - allow all backends and let wgpu choose the best one
    let wgpu_options = egui_wgpu::WgpuConfiguration {
        wgpu_setup: egui_wgpu::WgpuSetup::CreateNew {
            supported_backends: wgpu::Backends::all(),
            power_preference: wgpu::PowerPreference::default(),
            device_descriptor: std::sync::Arc::new(|adapter| wgpu::DeviceDescriptor {
                label: Some("urdf-editor device"),
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
        "urdf-editor",
        native_options,
        Box::new(|cc| Ok(Box::new(urdf_frontend::UrdfEditorApp::new(cc)))),
    )
}
