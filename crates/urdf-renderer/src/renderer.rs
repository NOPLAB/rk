//! Main renderer combining all sub-renderers

use std::collections::HashMap;

use glam::Mat4;
use uuid::Uuid;
use wgpu::util::DeviceExt;

use urdf_core::Part;

use crate::axis::{AxisInstance, AxisRenderer};
use crate::camera::Camera;
use crate::grid::GridRenderer;
use crate::marker::{MarkerInstance, MarkerRenderer};
use crate::mesh::{MeshData, MeshRenderer};

/// Mesh entry with bind group
pub struct MeshEntry {
    pub data: MeshData,
    pub bind_group: wgpu::BindGroup,
    pub part_id: Uuid,
}

/// Main renderer
pub struct Renderer {
    pub camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    // Sub-renderers
    grid_renderer: GridRenderer,
    mesh_renderer: MeshRenderer,
    axis_renderer: AxisRenderer,
    marker_renderer: MarkerRenderer,

    // Data
    meshes: Vec<MeshEntry>,
    part_to_mesh: HashMap<Uuid, usize>,

    // Display options
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_markers: bool,
    selected_mesh: Option<usize>,

    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
        let depth_format = wgpu::TextureFormat::Depth32Float;

        let camera = Camera::new(width as f32 / height as f32);
        let camera_uniform = camera.uniform();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let (depth_texture, depth_view) = Self::create_depth_texture(device, width, height);

        let grid_renderer = GridRenderer::new(
            device,
            format,
            depth_format,
            &camera_bind_group_layout,
            &camera_buffer,
        );

        let mesh_renderer = MeshRenderer::new(
            device,
            format,
            depth_format,
            &camera_bind_group_layout,
            &camera_buffer,
        );

        let axis_renderer = AxisRenderer::new(
            device,
            format,
            depth_format,
            &camera_bind_group_layout,
            &camera_buffer,
        );

        let marker_renderer = MarkerRenderer::new(
            device,
            format,
            depth_format,
            &camera_bind_group_layout,
            &camera_buffer,
        );

        Self {
            camera,
            camera_buffer,
            camera_bind_group_layout,
            depth_texture,
            depth_view,
            grid_renderer,
            mesh_renderer,
            axis_renderer,
            marker_renderer,
            meshes: Vec::new(),
            part_to_mesh: HashMap::new(),
            show_grid: true,
            show_axes: true,
            show_markers: true,
            selected_mesh: None,
            format,
            width,
            height,
        }
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.camera.update_aspect(width as f32 / height as f32);
        let (depth_texture, depth_view) = Self::create_depth_texture(device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    fn update_camera(&self, queue: &wgpu::Queue) {
        let camera_uniform = self.camera.uniform();
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
    }

    /// Add a part to the renderer
    pub fn add_part(&mut self, device: &wgpu::Device, part: &Part) -> usize {
        let data = MeshData::from_part(device, part);
        let bind_group = self.mesh_renderer.create_instance_bind_group(device, &data);

        let idx = self.meshes.len();
        self.meshes.push(MeshEntry {
            data,
            bind_group,
            part_id: part.id,
        });
        self.part_to_mesh.insert(part.id, idx);
        idx
    }

    /// Update a part's transform
    pub fn update_part_transform(&mut self, queue: &wgpu::Queue, part_id: Uuid, transform: Mat4) {
        if let Some(&idx) = self.part_to_mesh.get(&part_id) {
            if let Some(entry) = self.meshes.get_mut(idx) {
                entry.data.update_transform(queue, transform);
            }
        }
    }

    /// Update a part's color
    pub fn update_part_color(&mut self, queue: &wgpu::Queue, part_id: Uuid, color: [f32; 4]) {
        if let Some(&idx) = self.part_to_mesh.get(&part_id) {
            if let Some(entry) = self.meshes.get_mut(idx) {
                entry.data.update_color(queue, color);
            }
        }
    }

    /// Set selected part
    pub fn set_selected_part(&mut self, queue: &wgpu::Queue, part_id: Option<Uuid>) {
        // Deselect previous
        if let Some(prev_idx) = self.selected_mesh {
            if let Some(entry) = self.meshes.get_mut(prev_idx) {
                entry.data.set_selected(queue, false);
            }
        }

        // Select new
        self.selected_mesh = part_id.and_then(|id| self.part_to_mesh.get(&id).copied());
        if let Some(idx) = self.selected_mesh {
            if let Some(entry) = self.meshes.get_mut(idx) {
                entry.data.set_selected(queue, true);
            }
        }
    }

    /// Remove a part
    pub fn remove_part(&mut self, part_id: Uuid) {
        if let Some(idx) = self.part_to_mesh.remove(&part_id) {
            self.meshes.remove(idx);
            // Update indices
            for (_, mesh_idx) in self.part_to_mesh.iter_mut() {
                if *mesh_idx > idx {
                    *mesh_idx -= 1;
                }
            }
            if self.selected_mesh == Some(idx) {
                self.selected_mesh = None;
            } else if let Some(sel) = self.selected_mesh {
                if sel > idx {
                    self.selected_mesh = Some(sel - 1);
                }
            }
        }
    }

    /// Clear all parts
    pub fn clear_parts(&mut self) {
        self.meshes.clear();
        self.part_to_mesh.clear();
        self.selected_mesh = None;
    }

    /// Update axis display
    pub fn update_axes(&mut self, queue: &wgpu::Queue, instances: &[AxisInstance]) {
        self.axis_renderer.update_instances(queue, instances);
    }

    /// Update marker display
    pub fn update_markers(&mut self, queue: &wgpu::Queue, instances: &[MarkerInstance]) {
        self.marker_renderer.update_instances(queue, instances);
    }

    /// Render the scene
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
    ) {
        self.update_camera(queue);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.15,
                        g: 0.15,
                        b: 0.18,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Render grid
        if self.show_grid {
            self.grid_renderer.render(&mut render_pass);
        }

        // Render meshes
        for entry in &self.meshes {
            self.mesh_renderer.render(&mut render_pass, &entry.data, &entry.bind_group);
        }

        // Render axes
        if self.show_axes {
            self.axis_renderer.render(&mut render_pass);
        }

        // Render markers
        if self.show_markers {
            self.marker_renderer.render(&mut render_pass);
        }
    }

    /// Get mesh index for a part
    pub fn get_mesh_index(&self, part_id: Uuid) -> Option<usize> {
        self.part_to_mesh.get(&part_id).copied()
    }

    /// Get camera bind group layout for external use
    pub fn camera_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_group_layout
    }
}
