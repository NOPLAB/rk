//! Welcome dialog shown on first launch

const DOCS_URL: &str = "https://noplab.github.io/rk/docs";
const GETTING_STARTED_URL: &str = "https://noplab.github.io/rk/docs/getting-started/quick-start";

/// Welcome dialog state
#[derive(Default)]
pub struct WelcomeDialog {
    /// Whether to show the dialog
    pub open: bool,
}

impl WelcomeDialog {
    /// Create a new welcome dialog (shown by default for first launch)
    pub fn new(is_first_launch: bool) -> Self {
        Self {
            open: is_first_launch,
        }
    }

    /// Show the welcome dialog
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        // Semi-transparent background overlay
        let screen_rect = ctx.content_rect();
        egui::Area::new(egui::Id::new("welcome_overlay"))
            .fixed_pos(screen_rect.min)
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));
            });

        // Center the window
        egui::Window::new("Welcome to URDF Editor")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(450.0)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.heading("URDF Editor");
                    ui.add_space(4.0);
                    ui.label("A visual editor for creating robot descriptions");
                    ui.add_space(16.0);
                });

                ui.separator();
                ui.add_space(8.0);

                // Quick start guide
                ui.label(egui::RichText::new("Quick Start").strong());
                ui.add_space(4.0);

                egui::Grid::new("quick_start_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("1.");
                        ui.label("Import STL files or create primitive shapes");
                        ui.end_row();

                        ui.label("2.");
                        ui.label("Add joint points to parts for connections");
                        ui.end_row();

                        ui.label("3.");
                        ui.label("Connect parts to build your robot structure");
                        ui.end_row();

                        ui.label("4.");
                        ui.label("Configure joint properties and export to URDF");
                        ui.end_row();
                    });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Tips
                ui.label(egui::RichText::new("Tips").strong());
                ui.add_space(4.0);
                ui.label("- Right-click in viewport to add joint points");
                ui.label("- Use W/E/R keys to switch between Move/Rotate/Scale tools");
                ui.label("- Ctrl+S to save, Ctrl+O to open project");

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Documentation link
                ui.horizontal(|ui| {
                    ui.label("For detailed instructions, visit:");
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui
                        .hyperlink_to("Documentation", DOCS_URL)
                        .on_hover_text(DOCS_URL)
                        .clicked()
                    {
                        open_url(DOCS_URL);
                    }
                    ui.label(" | ");
                    if ui
                        .hyperlink_to("Getting Started Guide", GETTING_STARTED_URL)
                        .on_hover_text(GETTING_STARTED_URL)
                        .clicked()
                    {
                        open_url(GETTING_STARTED_URL);
                    }
                });

                ui.add_space(16.0);

                // Close button
                ui.vertical_centered(|ui| {
                    if ui.button("Get Started").clicked() {
                        self.open = false;
                    }
                });

                ui.add_space(8.0);
            });
    }
}

/// Open a URL in the default browser
fn open_url(url: &str) {
    if let Err(e) = open::that(url) {
        tracing::warn!("Failed to open URL: {}", e);
    }
}
