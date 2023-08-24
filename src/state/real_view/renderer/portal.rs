use crate::engine::prelude::*;
use crate::engine::renderer3d::renderer3d::{PlaneRenderer, PlaneVertex};

/// Extends normal 3d renderer
/// render view on the portal
///
pub struct PortalRenderer {
    pub depth_bind_layout: BindGroupLayout,
    /// Render the scenes in the portal view
    pub portal_view_rp: RenderPipeline,
}

impl PortalRenderer {
    pub fn new(gpu: &WgpuData, pr: &PlaneRenderer) -> Self {
        let device = &gpu.device;
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Portal 3d renderer"),
            source: ShaderSource::Wgsl(include_str!("portal.wgsl").into()),
        });

        let depth_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Depth,
                    view_dimension: Default::default(),
                    multisampled: false,
                },
                count: None,
            }],
        });

        let rp_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&pr.base_bind_layout, &pr.obj_layout, &depth_bind_layout],
            push_constant_ranges: &[],
        });


        let portal_view_rp = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&rp_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "plane_vs",
                buffers: &[PlaneVertex::desc()],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "portal_fs",
                targets: &[Some(ColorTargetState {
                    format: gpu.surface_cfg.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });
        Self {
            depth_bind_layout,
            portal_view_rp,
        }
    }
}

pub struct PortalDepthTexture {
    pub texture: TextureWrapper,
    pub bindgroup: BindGroup,
}

impl PortalDepthTexture {
    pub fn new(gpu: &WgpuData, pr: &PortalRenderer) -> Self {
        let texture = TextureWrapper::create_depth_texture(&gpu.device, &gpu.surface_cfg, "portal depth");
        let bindgroup = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("portal depth bind"),
            layout: &pr.depth_bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture.view),
            }],
        });
        Self {
            texture,
            bindgroup,
        }
    }
}

pub struct PortalView {
    pub color: TextureWrapper,
    /// the depth for the scene
    pub depth: TextureWrapper,
    pub pd: PortalDepthTexture,
    /// The bindgroup for plane 3d renderer group 1 (object)
    pub color_bind: BindGroup,
}

impl PortalView {
    pub fn new(gpu: &WgpuData, pr: &PlaneRenderer, apr: &PortalRenderer) -> Self {
        let color = TextureWrapper::new_with_size(&gpu.device, gpu.surface_cfg.format, (gpu.surface_cfg.width, gpu.surface_cfg.height));
        let depth = TextureWrapper::new_with_size(&gpu.device, TextureFormat::Depth32Float, (gpu.surface_cfg.width, gpu.surface_cfg.height));
        let color_bind = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("portal color bind"),
            layout: &pr.obj_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&color.view),
            }],
        });
        let pd = PortalDepthTexture::new(gpu, apr);
        Self {
            color,
            depth,
            color_bind,
            pd,
        }
    }
}