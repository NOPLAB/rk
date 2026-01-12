//! Plugin system for renderer extensibility.
//!
//! This module provides a plugin-based architecture for extending the renderer
//! with custom sub-renderers.

use crate::context::RenderContext;
use crate::scene::Scene;
use crate::traits::SubRenderer;

/// Registry for managing sub-renderers.
///
/// The registry maintains a collection of sub-renderers and handles
/// their lifecycle (initialization, rendering, cleanup).
pub struct RendererRegistry {
    sub_renderers: Vec<Box<dyn SubRenderer>>,
    sorted: bool,
}

impl RendererRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            sub_renderers: Vec::new(),
            sorted: true,
        }
    }

    /// Registers a new sub-renderer.
    ///
    /// The sub-renderer will be initialized on the next frame if a render
    /// context is available.
    pub fn register<R: SubRenderer + 'static>(&mut self, renderer: R) {
        self.sub_renderers.push(Box::new(renderer));
        self.sorted = false;
    }

    /// Unregisters a sub-renderer by name.
    ///
    /// Returns the removed sub-renderer, or None if not found.
    pub fn unregister(&mut self, name: &str) -> Option<Box<dyn SubRenderer>> {
        if let Some(pos) = self.sub_renderers.iter().position(|r| r.name() == name) {
            Some(self.sub_renderers.remove(pos))
        } else {
            None
        }
    }

    /// Gets a sub-renderer by name.
    pub fn get(&self, name: &str) -> Option<&dyn SubRenderer> {
        self.sub_renderers
            .iter()
            .find(|r| r.name() == name)
            .map(|r| r.as_ref())
    }

    /// Gets a mutable reference to a sub-renderer by name.
    pub fn get_mut<'a>(&'a mut self, name: &str) -> Option<&'a mut (dyn SubRenderer + 'a)> {
        for renderer in &mut self.sub_renderers {
            if renderer.name() == name {
                return Some(renderer.as_mut());
            }
        }
        None
    }

    /// Returns true if the registry contains a sub-renderer with the given name.
    pub fn contains(&self, name: &str) -> bool {
        self.sub_renderers.iter().any(|r| r.name() == name)
    }

    /// Returns the number of registered sub-renderers.
    pub fn len(&self) -> usize {
        self.sub_renderers.len()
    }

    /// Returns true if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.sub_renderers.is_empty()
    }

    /// Returns an iterator over all sub-renderers in priority order.
    pub fn iter(&self) -> impl Iterator<Item = &dyn SubRenderer> {
        self.sub_renderers.iter().map(|r| r.as_ref())
    }

    /// Returns a mutable iterator over all sub-renderers.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn SubRenderer>> {
        self.sub_renderers.iter_mut()
    }

    /// Sorts sub-renderers by priority if needed.
    fn ensure_sorted(&mut self) {
        if !self.sorted {
            self.sub_renderers.sort_by_key(|r| r.priority());
            self.sorted = true;
        }
    }

    /// Initializes all sub-renderers with the given context.
    pub fn init_all(&mut self, ctx: &RenderContext) {
        self.ensure_sorted();
        for renderer in &mut self.sub_renderers {
            renderer.on_init(ctx);
        }
    }

    /// Notifies all sub-renderers of a resize.
    pub fn resize_all(&mut self, ctx: &RenderContext, width: u32, height: u32) {
        for renderer in &mut self.sub_renderers {
            renderer.on_resize(ctx, width, height);
        }
    }

    /// Prepares all sub-renderers for rendering.
    pub fn prepare_all(&mut self, ctx: &RenderContext, scene: &Scene) {
        self.ensure_sorted();
        for renderer in &mut self.sub_renderers {
            if renderer.is_enabled() {
                renderer.prepare(ctx, scene);
            }
        }
    }

    /// Renders all enabled sub-renderers in priority order.
    pub fn render_all<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, scene: &Scene) {
        for renderer in &self.sub_renderers {
            if renderer.is_enabled() {
                renderer.render(pass, scene);
            }
        }
    }

    /// Destroys all sub-renderers.
    pub fn destroy_all(&mut self) {
        for renderer in &mut self.sub_renderers {
            renderer.on_destroy();
        }
        self.sub_renderers.clear();
    }
}

impl Default for RendererRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for renderer plugins.
///
/// Plugins can register multiple sub-renderers and perform additional setup.
pub trait RendererPlugin: Send + Sync + 'static {
    /// Returns the name of this plugin.
    fn name(&self) -> &str;

    /// Returns the version of this plugin.
    fn version(&self) -> &str;

    /// Called when the plugin is registered.
    ///
    /// Use this to register sub-renderers and perform other setup.
    fn on_register(&mut self, registry: &mut RendererRegistry);

    /// Called when the plugin is unregistered.
    ///
    /// Use this to clean up resources.
    fn on_unregister(&mut self, registry: &mut RendererRegistry);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRenderer {
        name: String,
        priority: i32,
        enabled: bool,
    }

    impl TestRenderer {
        fn new(name: &str, priority: i32) -> Self {
            Self {
                name: name.to_string(),
                priority,
                enabled: true,
            }
        }
    }

    impl SubRenderer for TestRenderer {
        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> i32 {
            self.priority
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }

        fn set_enabled(&mut self, enabled: bool) {
            self.enabled = enabled;
        }

        fn on_init(&mut self, _ctx: &RenderContext) {}
        fn on_resize(&mut self, _ctx: &RenderContext, _width: u32, _height: u32) {}
        fn prepare(&mut self, _ctx: &RenderContext, _scene: &Scene) {}
        fn render<'a>(&'a self, _pass: &mut wgpu::RenderPass<'a>, _scene: &Scene) {}
    }

    #[test]
    fn test_registry_ordering() {
        let mut registry = RendererRegistry::new();

        registry.register(TestRenderer::new("third", 300));
        registry.register(TestRenderer::new("first", 100));
        registry.register(TestRenderer::new("second", 200));

        registry.ensure_sorted();

        let names: Vec<&str> = registry.iter().map(|r| r.name()).collect();
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = RendererRegistry::new();

        registry.register(TestRenderer::new("test", 100));
        assert!(registry.contains("test"));

        let removed = registry.unregister("test");
        assert!(removed.is_some());
        assert!(!registry.contains("test"));
    }
}
