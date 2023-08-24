use wgpu::{Device, Queue};

use crate::engine::glft::instance::GltfInstance;
use crate::engine::glft::renderer::Locals;

pub mod model;
pub mod renderer;
pub mod instance;


/// Uniform buffer pool
/// Used by render passes to keep track of each objects local uniforms
/// and provides a way to update uniforms to render pipeline
#[allow(unused)]
pub struct UniformPool {
    label: &'static str,
    pub buffers: Vec<wgpu::Buffer>,
    size: u64,
}

#[allow(unused)]
impl UniformPool {
    pub fn new(label: &'static str, size: u64) -> Self {
        Self {
            label,
            buffers: Vec::new(),
            size,
        }
    }

    pub fn alloc_buffers(&mut self, count: usize, device: &Device) {
        self.buffers = Vec::new();

        for _ in 0..count {
            let local_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&self.label),
                size: self.size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.buffers.push(local_uniform_buffer);
        }
    }

    pub fn update_uniform<T: bytemuck::Pod>(&self, index: usize, data: T, queue: &Queue) {
        if &self.buffers.len() > &0 {
            queue.write_buffer(&self.buffers[index], 0, bytemuck::cast_slice(&[data]));
        }
    }
}

// This represents a 3D model in a scene.
// It contains the 3D model, instance data, and a parent ID (TBD)
pub struct ModelObject {
    // ID of parent Node
    pub parent: u32,
    // local: Matrix?
    // Local position of model (for relative calculations)
    pub locals: Locals,
    // The vertex buffers and texture data
    pub model: model::Model,
    // An array of positional data for each instance (can just pass 1 instance)
    pub instances: Vec<GltfInstance>,
}