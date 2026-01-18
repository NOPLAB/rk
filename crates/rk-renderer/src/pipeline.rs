//! Pipeline builder utilities
//!
//! This module provides utilities for creating render pipelines with
//! common configurations, reducing boilerplate code across renderers.

/// Configuration for creating a render pipeline.
pub struct PipelineConfig<'a> {
    /// Pipeline label for debugging
    pub label: &'a str,
    /// WGSL shader source code
    pub shader_source: &'a str,
    /// Output texture format
    pub format: wgpu::TextureFormat,
    /// Depth texture format
    pub depth_format: wgpu::TextureFormat,
    /// Bind group layouts (camera layout should be first)
    pub bind_group_layouts: &'a [&'a wgpu::BindGroupLayout],
    /// Vertex buffer layouts
    pub vertex_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
    /// Primitive topology
    pub topology: wgpu::PrimitiveTopology,
    /// Face culling mode
    pub cull_mode: Option<wgpu::Face>,
    /// Whether to write to depth buffer
    pub depth_write: bool,
    /// Depth comparison function
    pub depth_compare: wgpu::CompareFunction,
    /// Blend state for color output
    pub blend: Option<wgpu::BlendState>,
    /// MSAA sample count (1 = disabled)
    pub sample_count: u32,
    /// Vertex shader entry point
    pub vs_entry_point: &'a str,
    /// Fragment shader entry point
    pub fs_entry_point: &'a str,
}

impl<'a> PipelineConfig<'a> {
    /// Create a new pipeline config with common defaults.
    ///
    /// Default settings:
    /// - Triangle list topology
    /// - No face culling
    /// - Depth write enabled
    /// - Depth compare: Less
    /// - Alpha blending enabled
    /// - MSAA sample count from viewport constants
    pub fn new(
        label: &'a str,
        shader_source: &'a str,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        bind_group_layouts: &'a [&'a wgpu::BindGroupLayout],
    ) -> Self {
        use crate::constants::viewport::SAMPLE_COUNT;
        Self {
            label,
            shader_source,
            format,
            depth_format,
            bind_group_layouts,
            vertex_layouts: Vec::new(),
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            depth_write: true,
            depth_compare: wgpu::CompareFunction::Less,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            sample_count: SAMPLE_COUNT,
            vs_entry_point: "vs_main",
            fs_entry_point: "fs_main",
        }
    }

    /// Set vertex buffer layouts.
    pub fn with_vertex_layouts(mut self, layouts: Vec<wgpu::VertexBufferLayout<'a>>) -> Self {
        self.vertex_layouts = layouts;
        self
    }

    /// Set primitive topology.
    pub fn with_topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    /// Set face culling mode.
    pub fn with_cull_mode(mut self, cull_mode: Option<wgpu::Face>) -> Self {
        self.cull_mode = cull_mode;
        self
    }

    /// Set depth write and compare settings.
    pub fn with_depth(mut self, write: bool, compare: wgpu::CompareFunction) -> Self {
        self.depth_write = write;
        self.depth_compare = compare;
        self
    }

    /// Disable depth testing (always pass, no write).
    /// Useful for overlay elements like gizmos.
    pub fn without_depth_test(mut self) -> Self {
        self.depth_write = false;
        self.depth_compare = wgpu::CompareFunction::Always;
        self
    }

    /// Set blend state.
    pub fn with_blend(mut self, blend: wgpu::BlendState) -> Self {
        self.blend = Some(blend);
        self
    }

    /// Set depth write enable.
    pub fn with_depth_write(mut self, enabled: bool) -> Self {
        self.depth_write = enabled;
        self
    }

    /// Set custom entry points for vertex and fragment shaders.
    pub fn with_entry_point(mut self, vs: &'a str, fs: &'a str) -> Self {
        self.vs_entry_point = vs;
        self.fs_entry_point = fs;
        self
    }

    /// Build the render pipeline.
    pub fn build(self, device: &wgpu::Device) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} Shader", self.label)),
            source: wgpu::ShaderSource::Wgsl(self.shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{} Pipeline Layout", self.label)),
            bind_group_layouts: self.bind_group_layouts,
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{} Pipeline", self.label)),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(self.vs_entry_point),
                buffers: &self.vertex_layouts,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(self.fs_entry_point),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend: self.blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                cull_mode: self.cull_mode,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: self.depth_format,
                depth_write_enabled: self.depth_write,
                depth_compare: self.depth_compare,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: self.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }
}

/// Create a camera bind group from the layout and buffer.
///
/// This is a common operation used by all sub-renderers.
pub fn create_camera_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
    label: &str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&format!("{} Camera Bind Group", label)),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}
