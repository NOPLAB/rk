//! SubRenderer trait definition.

use crate::context::RenderContext;
use crate::scene::Scene;

/// A sub-renderer that handles a specific type of rendering.
///
/// Sub-renderers are composable units that can be registered with the main
/// renderer to add new rendering capabilities. Each sub-renderer is responsible
/// for a specific visual element (e.g., grid, meshes, gizmos).
///
/// # Priority
///
/// Sub-renderers are executed in order of their priority (lower values first).
/// Typical priority ranges:
/// - 0-99: Background elements (grid, skybox)
/// - 100-199: Main geometry (meshes)
/// - 200-299: Overlays (axes, markers)
/// - 1000+: Always-on-top elements (gizmos, UI)
pub trait SubRenderer: Send + Sync {
    /// Returns the unique name of this sub-renderer.
    fn name(&self) -> &str;

    /// Returns the render priority (lower = rendered first).
    fn priority(&self) -> i32;

    /// Returns whether this sub-renderer is currently enabled.
    fn is_enabled(&self) -> bool;

    /// Enables or disables this sub-renderer.
    fn set_enabled(&mut self, enabled: bool);

    /// Called when the render context is initialized.
    ///
    /// Use this to create GPU resources (pipelines, buffers, etc.).
    fn on_init(&mut self, ctx: &RenderContext);

    /// Called when the viewport is resized.
    fn on_resize(&mut self, ctx: &RenderContext, width: u32, height: u32);

    /// Prepare data for rendering.
    ///
    /// Called once per frame before the render pass. Use this to update
    /// instance buffers, compute visibility, etc.
    fn prepare(&mut self, ctx: &RenderContext, scene: &Scene);

    /// Execute the render commands.
    ///
    /// Called during the render pass. The sub-renderer should set its pipeline,
    /// bind groups, and issue draw calls.
    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, scene: &Scene);

    /// Called when the sub-renderer is being destroyed.
    ///
    /// Use this to clean up any resources.
    fn on_destroy(&mut self) {}
}

/// Extension trait for sub-renderers that support configuration.
pub trait ConfigurableSubRenderer: SubRenderer {
    /// The configuration type for this sub-renderer.
    type Config: Default + Clone;

    /// Returns the current configuration.
    fn config(&self) -> &Self::Config;

    /// Updates the configuration.
    fn set_config(&mut self, config: Self::Config);
}
