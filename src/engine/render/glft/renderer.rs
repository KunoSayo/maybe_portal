use std::collections::HashMap;

use wgpu::*;
use wgpu::util::{DeviceExt, RenderEncoder};

use crate::engine::{TextureWrapper, Vertex, WgpuData};
use crate::engine::glft::{ModelObject, UniformPool};
use crate::engine::glft::instance::{GltfInstance, InstanceRaw};
use crate::engine::glft::model::{DrawModel, ModelVertex};
use crate::engine::render::camera::{Camera, CameraUniform};
use crate::engine::renderer::Renderer;

// Global uniform data
// aka camera position and ambient light color
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
    ambient: [f32; 4],
}

// Local uniform data
// aka the individual model's data
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Locals {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub normal: [f32; 4],
    pub lights: [f32; 4],
}

// Uniform for light data (position + color)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding: u32,
    pub color: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding2: u32,
}

pub struct RendererConfig {
    pub max_lights: usize,
    pub ambient: [u32; 4],
    pub point_frame: bool,
}

#[allow(unused)]
pub struct ModelRenderer {
    // Uniforms
    global_bind_group_layout: BindGroupLayout,
    global_uniform_buffer: Buffer,
    global_bind_group: BindGroup,
    local_bind_group_layout: BindGroupLayout,
    // pub local_uniform_buffer: wgpu::Buffer,
    local_bind_groups: HashMap<usize, BindGroup>,
    uniform_pool: UniformPool,
    // Render pipeline
    render_pipeline: RenderPipeline,
    // Lighting
    light_uniform: LightUniform,
    light_buffer: Buffer,
    // pub light_bind_group: wgpu::BindGroup,
    light_render_pipeline: RenderPipeline,
    // Camera
    pub(crate) camera_uniform: CameraUniform,
    // Instances
    instance_buffers: HashMap<usize, Buffer>,
}

#[allow(unused)]
impl ModelRenderer {
    pub fn new(
        renderer_config: &RendererConfig,
        device: &Device,
        _queue: &Queue,
        config: &SurfaceConfiguration,
        camera: &Camera,
    ) -> ModelRenderer {
        use std::mem;
        // Setup the shader
        // We use specific shaders for each pass to define visual effect
        // and also to have the right shader for the uniforms we pass
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Normal Shader"),
            source: ShaderSource::Wgsl(include_str!("model_shader.wgsl").into()),
        });

        // Setup global uniforms
        // Global bind group layout
        let light_size = mem::size_of::<LightUniform>() as BufferAddress;
        let global_size = mem::size_of::<Globals>() as BufferAddress;
        let global_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("[Gltf] Globals"),
                entries: &[
                    // Global uniforms
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(global_size),
                        },
                        count: None,
                    },
                    // Lights
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(light_size),
                        },
                        count: None,
                    },
                    // Sampler for textures
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Global uniform buffer
        let global_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Globals"),
            size: global_size,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Create light uniforms and setup buffer for them
        let light_uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };
        let light_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Lights"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        // We also need a sampler for our textures
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("sampler"),
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        });
        // Combine the global uniform, the lights, and the texture sampler into one bind group
        let global_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Globals"),
            layout: &global_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: global_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Setup local uniforms
        // Local bind group layout
        let local_size = mem::size_of::<Locals>() as BufferAddress;
        let local_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Locals"),
                entries: &[
                    // Local uniforms
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(local_size),
                        },
                        count: None,
                    },
                    // Mesh texture
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        // Setup the render pipeline
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Pipeline"),
            bind_group_layouts: &[&global_bind_group_layout, &local_bind_group_layout],
            push_constant_ranges: &[],
        });
        let vertex_buffers = [ModelVertex::desc(), InstanceRaw::desc()];
        let depth_stencil = Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        });

        // Enable/disable wireframe mode
        let topology = if renderer_config.point_frame {
            PrimitiveTopology::PointList
        } else {
            PrimitiveTopology::TriangleList
        };

        let primitive = PrimitiveState {
            cull_mode: None,
            topology,
            ..Default::default()
        };
        let multisample = MultisampleState {
            ..Default::default()
        };

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("glft renderer pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &vertex_buffers,
            },
            primitive,
            depth_stencil: depth_stencil.clone(),
            multisample,
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState {
                        alpha: BlendComponent::REPLACE,
                        color: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        // Create depth texture
        let depth_texture =
            TextureWrapper::create_depth_texture(&device, &config, "depth_texture");

        // Setup camera uniform
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let light_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Light Shader"),
            source: ShaderSource::Wgsl(include_str!("light.wgsl").into()),
        });

        let light_render_pipeline =
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Light Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &light_shader,
                    entry_point: "vs_main",
                    buffers: &[ModelVertex::desc()],
                },
                primitive,
                depth_stencil,
                multisample,
                fragment: Some(FragmentState {
                    module: &light_shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: config.format,
                        blend: Some(BlendState {
                            alpha: BlendComponent::REPLACE,
                            color: BlendComponent::REPLACE,
                        }),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                multiview: None,
            });

        // Create instance buffer
        let instance_buffers = HashMap::new();

        let uniform_pool = UniformPool::new("Locals", local_size);

        ModelRenderer {
            global_bind_group_layout,
            global_uniform_buffer,
            global_bind_group,
            local_bind_group_layout,
            local_bind_groups: Default::default(),
            uniform_pool,
            render_pipeline,
            camera_uniform,
            light_uniform,
            light_buffer,
            light_render_pipeline,
            instance_buffers,
        }
    }

    pub fn update_camera(&mut self, camera: &Camera) {
        self.camera_uniform.update_view_proj(camera);
    }
}

impl Renderer<ModelObject> for ModelRenderer {
    fn render<'a, T: RenderEncoder<'a>>(&'a mut self, encoder: &mut T, wgpu: &WgpuData, nodes: &'a [ModelObject]) {
        let device = wgpu.device.as_ref();
        let queue = wgpu.queue.as_ref();
        let views = &wgpu.views;


        queue.write_buffer(&self.global_uniform_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
        {

            // Allocate buffers for local uniforms
            if self.uniform_pool.buffers.len() < nodes.len() {
                self.uniform_pool.alloc_buffers(nodes.len(), &device);
            }

            // Loop over the nodes/models in a scene and setup the specific models
            // local uniform bind group and instance buffers to send to shader
            // This is separate loop from the render because of Rust ownership
            // (can prob wrap in block instead to limit mutable use)
            let mut model_index = 0;
            for node in nodes {
                let local_buffer = &self.uniform_pool.buffers[model_index];
                queue.write_buffer(local_buffer, 0, bytemuck::cast_slice(&[node.locals]));
                // We create a bind group for each model's local uniform data
                // and store it in a hash map to look up later

                self.local_bind_groups
                    .entry(model_index)
                    .or_insert_with(|| {
                        let view = node.model.materials.iter().filter(|x| x.diffuse_texture.is_some())
                            .map(|x| &x.diffuse_texture.as_ref().unwrap().view)
                            .next().unwrap_or(&views.get_off_screen().view);
                        device.create_bind_group(&BindGroupDescriptor {
                            label: Some("Locals"),
                            layout: &self.local_bind_group_layout,
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: local_buffer.as_entire_binding(),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::TextureView(
                                        view,
                                    ),
                                },
                            ],
                        })
                    });

                // Setup instance buffer for the model
                // similar process as above using HashMap
                self.instance_buffers.entry(model_index).or_insert_with(|| {
                    // We condense the matrix properties into a flat array (aka "raw data")
                    // (which is how buffers work - so we can "stride" over chunks)
                    let instance_data = node
                        .instances
                        .iter()
                        .map(GltfInstance::to_raw)
                        .collect::<Vec<_>>();
                    // Create the instance buffer with our data
                    let instance_buffer =
                        device.create_buffer_init(&util::BufferInitDescriptor {
                            label: Some("Instance Buffer"),
                            contents: bytemuck::cast_slice(&instance_data),
                            usage: BufferUsages::VERTEX,
                        });

                    instance_buffer
                });

                model_index += 1;
            }

            // Setup lighting pipeline
            encoder.set_pipeline(&self.light_render_pipeline);
            // Draw/calculate the lighting on models
            // render_pass.draw_light_model(
            //     &nodes[0].model,
            //     &self.global_bind_group,
            //     &self
            //         .local_bind_groups
            //         .get(&0)
            //         .expect("No local bind group found for lighting"),
            // );

            // Setup render pipeline
            encoder.set_pipeline(&self.render_pipeline);
            encoder.set_bind_group(0, &self.global_bind_group, &[]);

            // Render/draw all nodes/models
            // We reset index here to use again
            model_index = 0;
            for node in nodes {
                // if node.model.materials.len() > 0 {
                // Set the instance buffer unique to the model
                encoder.set_vertex_buffer(1, self.instance_buffers[&model_index].slice(..));

                // Draw all the model instances
                encoder.draw_model_instanced(
                    &node.model,
                    0..node.instances.len() as u32,
                    &self.local_bind_groups[&model_index],
                );
                // }

                model_index += 1;
            }
        }
    }
}