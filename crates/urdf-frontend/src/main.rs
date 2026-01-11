//! URDF Editor main entry point

// Native entry point
#[cfg(not(target_arch = "wasm32"))]
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

    // Load app icon
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icons/256x256.png"))
            .expect("Failed to load icon");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("URDF Editor")
            .with_icon(icon),
        wgpu_options,
        ..Default::default()
    };

    eframe::run_native(
        "rk",
        native_options,
        Box::new(|cc| Ok(Box::new(urdf_frontend::UrdfEditorApp::new(cc)))),
    )
}

// WASM entry point
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        // Get the canvas element
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("urdf-editor-canvas")
            .expect("Failed to find canvas element")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("urdf-editor-canvas was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(urdf_frontend::UrdfEditorApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        let loading_text = document.get_element_by_id("loading");
        if let Some(loading_text) = loading_text {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(&format!(
                        "<p>The app has crashed. See the developer console for details.</p><p>{:?}</p>",
                        e
                    ));
                }
            }
        }
    });
}
