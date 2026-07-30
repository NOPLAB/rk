//! Headless (offscreen) rendering: the agent's "eyes".
//!
//! Creates a wgpu device without any surface, rebuilds a fresh
//! `rk_renderer::Renderer` scene from the engine on every shot, renders
//! into a COPY_SRC texture and reads it back as an encoded PNG.
//! `rk-renderer` itself is unchanged — it never needed a window.

use glam::{Mat4, Vec3};
use rk_engine::Engine;

/// Texture format used for offscreen rendering (maps directly to PNG bytes)
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Default CAD body display color (matches rk-desktop's viewport)
const BODY_COLOR: [f32; 4] = [0.7, 0.7, 0.8, 1.0];

#[derive(Debug, thiserror::Error)]
pub enum HeadlessError {
    #[error("no compatible GPU adapter found: {0}")]
    NoAdapter(String),
    #[error("failed to create GPU device: {0}")]
    Device(String),
    #[error("GPU readback failed: {0}")]
    Readback(String),
    #[error("PNG encoding failed: {0}")]
    Png(#[from] png::EncodingError),
    #[error("invalid screenshot options: {0}")]
    InvalidOptions(String),
}

/// Named camera presets (yaw/pitch around the scene)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewPreset {
    /// Isometric-style three-quarter view
    #[default]
    Iso,
    /// Looking at the XZ plane from -Y
    Front,
    /// Looking down the Z axis
    Top,
    /// Looking at the YZ plane from +X
    Side,
}

impl ViewPreset {
    fn yaw_pitch_deg(self) -> (f32, f32) {
        match self {
            ViewPreset::Iso => (45.0, 30.0),
            ViewPreset::Front => (-90.0, 0.0),
            ViewPreset::Top => (0.0, 89.0),
            ViewPreset::Side => (0.0, 0.0),
        }
    }
}

/// Options for a single screenshot
#[derive(Debug, Clone)]
pub struct ScreenshotOptions {
    pub width: u32,
    pub height: u32,
    pub view: ViewPreset,
    /// Override the preset yaw (degrees)
    pub yaw_deg: Option<f32>,
    /// Override the preset pitch (degrees)
    pub pitch_deg: Option<f32>,
    /// Override the auto-fit camera distance (meters)
    pub distance: Option<f32>,
    /// Override the auto-fit camera target (meters)
    pub target: Option<[f32; 3]>,
    pub show_grid: bool,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 768,
            view: ViewPreset::Iso,
            yaw_deg: None,
            pitch_deg: None,
            distance: None,
            target: None,
            show_grid: true,
        }
    }
}

/// Owns the surfaceless GPU device; renderers are rebuilt per shot
pub struct HeadlessRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl HeadlessRenderer {
    /// Create a GPU device with no window/surface attached
    pub fn new() -> Result<Self, HeadlessError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .map_err(|e| HeadlessError::NoAdapter(e.to_string()))?;

        tracing::info!("headless adapter: {:?}", adapter.get_info());

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("rk-mcp headless device"),
            ..Default::default()
        }))
        .map_err(|e| HeadlessError::Device(e.to_string()))?;

        Ok(Self { device, queue })
    }

    /// Render the engine's current scene and return PNG bytes.
    ///
    /// Takes `&mut Engine` because body meshes are lazily tessellated
    /// and cached inside the engine.
    pub fn screenshot(
        &self,
        engine: &mut Engine,
        opts: &ScreenshotOptions,
    ) -> Result<Vec<u8>, HeadlessError> {
        if opts.width == 0 || opts.height == 0 || opts.width > 4096 || opts.height > 4096 {
            return Err(HeadlessError::InvalidOptions(format!(
                "width/height must be in 1..=4096, got {}x{}",
                opts.width, opts.height
            )));
        }

        let mut renderer =
            rk_renderer::Renderer::new(&self.device, FORMAT, opts.width, opts.height);
        renderer.set_show_grid(opts.show_grid);

        let bounds = self.populate_scene(&mut renderer, engine)?;
        Self::place_camera(&mut renderer, opts, bounds);

        let rgba = self.render_to_rgba(&renderer, opts.width, opts.height)?;
        encode_png(&rgba, opts.width, opts.height)
    }

    /// The same scene rk-desktop builds: parts with meshes, final
    /// render transforms (link world x part origin), and CAD bodies.
    /// Returns the scene bounding sphere (center, radius) if non-empty.
    fn populate_scene(
        &self,
        renderer: &mut rk_renderer::Renderer,
        engine: &mut Engine,
    ) -> Result<Option<(Vec3, f32)>, HeadlessError> {
        let mut center_sum = Vec3::ZERO;
        let mut point_count = 0u32;
        let mut max_radius = 0.0f32;

        let transforms = engine.part_render_transforms();
        let parts: Vec<rk_core::Part> = engine
            .parts()
            .filter(|p| !p.vertices.is_empty())
            .cloned()
            .collect();

        for part in &parts {
            renderer.add_part(&self.device, part);
        }
        for (part_id, transform) in &transforms {
            renderer.update_part_transform(&self.queue, *part_id, *transform);
        }
        for part in &parts {
            let transform = transforms
                .iter()
                .find(|(id, _)| *id == part.id)
                .map(|(_, t)| *t)
                .unwrap_or(Mat4::IDENTITY);
            let center = transform.transform_point3(part.center());
            center_sum += center;
            point_count += 1;
            max_radius = max_radius.max(part.size().length() / 2.0);
        }

        for body_id in engine.body_ids() {
            match engine.body_mesh(body_id) {
                Ok(mesh) => {
                    let (center, radius) = mesh_bounds(&mesh.vertices);
                    renderer.add_cad_body(
                        &self.device,
                        body_id,
                        &mesh.vertices,
                        &mesh.normals,
                        &mesh.indices,
                        Mat4::IDENTITY,
                        BODY_COLOR,
                    );
                    center_sum += center;
                    point_count += 1;
                    max_radius = max_radius.max(radius);
                }
                Err(e) => {
                    tracing::warn!("skipping body {body_id} in screenshot: {e}");
                }
            }
        }

        if point_count == 0 {
            return Ok(None);
        }
        Ok(Some((center_sum / point_count as f32, max_radius)))
    }

    fn place_camera(
        renderer: &mut rk_renderer::Renderer,
        opts: &ScreenshotOptions,
        bounds: Option<(Vec3, f32)>,
    ) {
        let (fit_center, fit_radius) = bounds.unwrap_or((Vec3::ZERO, 1.0));
        let (preset_yaw, preset_pitch) = opts.view.yaw_pitch_deg();

        let camera = renderer.camera_mut();
        camera.target = opts.target.map(Vec3::from).unwrap_or(fit_center);
        camera.distance = opts
            .distance
            .unwrap_or((fit_radius * 2.5).max(1.0))
            .clamp(0.1, 10000.0);
        camera.yaw = opts.yaw_deg.unwrap_or(preset_yaw).to_radians();
        camera.pitch = opts.pitch_deg.unwrap_or(preset_pitch).to_radians();
        // Recompute the eye position from the orbit parameters
        camera.orbit(0.0, 0.0);
    }

    fn render_to_rgba(
        &self,
        renderer: &rk_renderer::Renderer,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, HeadlessError> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rk-mcp screenshot target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bytes_per_row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rk-mcp screenshot readback"),
            size: u64::from(bytes_per_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rk-mcp screenshot encoder"),
            });
        renderer.render(&mut encoder, &view, &self.queue);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| HeadlessError::Readback(e.to_string()))?;
        rx.recv()
            .map_err(|_| HeadlessError::Readback("map_async callback dropped".into()))?
            .map_err(|e| HeadlessError::Readback(e.to_string()))?;

        let mapped = slice.get_mapped_range();
        let row_bytes = (width * 4) as usize;
        let mut rgba = Vec::with_capacity(row_bytes * height as usize);
        for row in 0..height as usize {
            let start = row * bytes_per_row as usize;
            rgba.extend_from_slice(&mapped[start..start + row_bytes]);
        }
        drop(mapped);
        buffer.unmap();
        Ok(rgba)
    }
}

/// Bounding sphere of a vertex list (center, radius)
fn mesh_bounds(vertices: &[[f32; 3]]) -> (Vec3, f32) {
    if vertices.is_empty() {
        return (Vec3::ZERO, 0.0);
    }
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for v in vertices {
        let p = Vec3::from(*v);
        min = min.min(p);
        max = max.max(p);
    }
    let center = (min + max) / 2.0;
    ((center), (max - min).length() / 2.0)
}

fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, HeadlessError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(rgba)?;
    }
    Ok(out)
}
