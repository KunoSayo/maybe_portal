use std::ops::Range;

use anyhow::anyhow;
use gltf::{Gltf, Node};
use gltf::buffer::Source;
use log::trace;
use nalgebra::vector;
use wgpu::util::{DeviceExt, RenderEncoder};

use crate::engine::{TextureWrapper, WgpuData};
use crate::engine::Vertex;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<TextureWrapper>,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

#[allow(unused)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

#[allow(unused)]
impl Model {
    pub fn load(wgpu: &WgpuData, mut gltf: Gltf, label: Option<&str>) -> anyhow::Result<Self> {
        let mut buffer_data = Vec::new();
        if let Some(blob) = gltf.blob.take() {
            buffer_data.push(blob);
        }
        for buffer in gltf.buffers() {
            match buffer.source() {
                Source::Uri(_) => {
                    return Err(anyhow!("This model has uri source but not impl yet!"));
                }
                _ => {}
            }
        }

        let mut meshes = Vec::new();
        let mut materials = Vec::new();

        struct NodeData<'a> {
            buffer_data: &'a [Vec<u8>],
            wgpu: &'a WgpuData,
            meshes: &'a mut Vec<Mesh>,
            materials: &'a mut Vec<Material>,
        }

        impl NodeData<'_> {
            fn load_node(&mut self, node: Node) {
                log::trace!(target: "gltf_load", "Node {}", node.index());
                for x in node.children() {
                    self.load_node(x);
                }
                let buffer_data = &self.buffer_data;
                let wgpu = &self.wgpu;
                let meshes = &mut self.meshes;
                let materials = &mut self.materials;

                let trans = nalgebra::Matrix4::from(node.transform().matrix());
                if let Some(mesh) = node.mesh() {
                    let primitives = mesh.primitives();
                    for primitive in primitives {
                        let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));

                        let material = primitive.material().index();

                        let mut vertices = Vec::new();
                        if let Some(vertex_attribute) = reader.read_positions() {
                            vertex_attribute.for_each(|vertex| {
                                let position = trans * vector![vertex[0], vertex[1], vertex[2], 1.0];
                                vertices.push(ModelVertex {
                                    position: position.xyz().into(),
                                    tex_coords: Default::default(),
                                    normal: Default::default(),
                                })
                            });
                        }
                        if let Some(normal_attribute) = reader.read_normals() {
                            let mut normal_index = 0;
                            normal_attribute.for_each(|normal| {
                                vertices[normal_index].normal = normal;

                                normal_index += 1;
                            });
                        }
                        if let Some(tex_coord_attribute) = reader.read_tex_coords(0).map(|v| v.into_f32()) {
                            let mut tex_coord_index = 0;
                            tex_coord_attribute.for_each(|tex_coord| {
                                vertices[tex_coord_index].tex_coords = tex_coord;

                                tex_coord_index += 1;
                            });
                        }

                        let mut indices = Vec::new();
                        if let Some(indices_raw) = reader.read_indices() {
                            indices.append(&mut indices_raw.into_u32().collect::<Vec<u32>>());
                        }

                        let mesh_name = mesh.name().unwrap_or("default_mesh_name").into();
                        let vertex_buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("{} Vertex Buffer", mesh_name)),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                        let index_buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("{} Index Buffer", mesh_name)),
                            contents: bytemuck::cast_slice(&indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                        meshes.push(Mesh {
                            name: mesh_name,
                            vertex_buffer,
                            index_buffer,
                            num_elements: indices.len() as u32,
                            material: material.unwrap_or(0),
                        })
                    }
                }
            }
        }

        let mut node_data = NodeData {
            buffer_data: &buffer_data[..],
            wgpu,
            meshes: &mut meshes,
            materials: &mut materials,
        };

        for scene in gltf.scenes() {
            for node in scene.nodes() {
                node_data.load_node(node);
            }
        }

        for material in gltf.materials() {
            let pbr = material.pbr_metallic_roughness();
            let base_color_texture = pbr.base_color_texture();
            let name = material.name().unwrap_or("Default Material").to_string();
            if let Some(texture_source) = base_color_texture.map(|tex| { tex.texture().source().source() }) {
                match texture_source {
                    gltf::image::Source::View { view, mime_type: mt } => {
                        trace!(target: "gltf_load", "Loading texture for type: {mt}");
                        let diffuse_texture = Some(TextureWrapper::from_bytes(
                            &wgpu.device, &wgpu.queue,
                            &buffer_data[view.buffer().index()][view.offset()..view.offset() + view.length()],
                            label, false)?);

                        materials.push(Material {
                            name,
                            diffuse_texture,
                        });
                    }
                    gltf::image::Source::Uri { uri: _, mime_type: _ } => {
                        return Err(anyhow!("This model has uri source for image but not impl yet!"));
                    }
                };
            } else {
                materials.push(Material {
                    name,
                    diffuse_texture: None,
                });
            }
        }

        Ok(Self { meshes, materials })
    }
}


impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}


pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        local_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
        local_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_model(&mut self, model: &'a Model, local_bind_group: &'a wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        local_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b, T: RenderEncoder<'a>> DrawModel<'b> for T
    where
        'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        local_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, 0..1, local_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        local_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(1, local_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(&mut self, model: &'b Model, local_bind_group: &'b wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, local_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        local_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_mesh_instanced(mesh, instances.clone(), local_bind_group);
        }
    }
}

pub trait DrawLight<'a> {
    fn draw_light_mesh(
        &mut self,
        mesh: &'a Mesh,
        global_bind_group: &'a wgpu::BindGroup,
        local_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
        global_bind_group: &'a wgpu::BindGroup,
        local_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_light_model(
        &mut self,
        model: &'a Model,
        global_bind_group: &'a wgpu::BindGroup,
        local_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_light_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        global_bind_group: &'a wgpu::BindGroup,
        local_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLight<'b> for wgpu::RenderPass<'a>
    where
        'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        global_bind_group: &'b wgpu::BindGroup,
        local_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_light_mesh_instanced(mesh, 0..1, global_bind_group, local_bind_group);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        global_bind_group: &'b wgpu::BindGroup,
        local_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, global_bind_group, &[]);
        self.set_bind_group(1, local_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        global_bind_group: &'b wgpu::BindGroup,
        local_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_light_model_instanced(model, 0..1, global_bind_group, local_bind_group);
    }
    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        global_bind_group: &'b wgpu::BindGroup,
        local_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_light_mesh_instanced(
                mesh,
                instances.clone(),
                global_bind_group,
                local_bind_group,
            );
        }
    }
}




