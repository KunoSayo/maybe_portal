use nalgebra::matrix;

use crate::engine::Vertex;

// Instances
// Lets us duplicate objects in a scene with less cost
#[allow(unused)]
pub struct GltfInstance {
    pub position: nalgebra::Vector3<f32>,
    pub rotation: nalgebra::Quaternion<f32>,
}


fn rotation_to_matrix4(rotation: &nalgebra::Quaternion<f32>) -> nalgebra::Matrix4<f32> {
    let inv = 2.0;
    let [x, y, z, w] = rotation.coords.data.0[0];
    let xs = inv * x;
    let ys = inv * y;
    let zs = inv * z;

    let xx = x * xs;
    let xy = x * ys;
    let xz = x * zs;
    let xw = xs * w;
    let yy = y * ys;
    let yz = y * zs;
    let yw = ys * w;
    let zz = z * zs;
    let zw = zs * w;

    let m11 = 1.0 - (yy + zz);
    let m21 = xy - zw;
    let m31 = xz + yw;

    let m12 = xy + zw;
    let m22 = 1.0 - (xx + zz);
    let m32 = yz - xw;

    let m13 = xz - yw;
    let m23 = yz + xw;
    let m33 = 1.0 - (xx + yy);
    matrix![m11, m21, m31, 0.0;
            m12, m22, m32, 0.0;
            m13, m23, m33, 0.0;
            0.0, 0.0, 0.0, 1.0]
}

fn rotation_to_matrix3(rotation: &nalgebra::Quaternion<f32>) -> nalgebra::Matrix3<f32> {
    let inv = 2.0;
    let [x, y, z, w] = rotation.coords.data.0[0];
    let xs = inv * x;
    let ys = inv * y;
    let zs = inv * z;

    let xx = x * xs;
    let xy = x * ys;
    let xz = x * zs;
    let xw = xs * w;
    let yy = y * ys;
    let yz = y * zs;
    let yw = ys * w;
    let zz = z * zs;
    let zw = zs * w;

    let m11 = 1.0 - (yy + zz);
    let m21 = xy - zw;
    let m31 = xz + yw;

    let m12 = xy + zw;
    let m22 = 1.0 - (xx + zz);
    let m32 = yz - xw;

    let m13 = xz - yw;
    let m23 = yz + xw;
    let m33 = 1.0 - (xx + yy);
    matrix![m11, m21, m31;
            m12, m22, m32;
            m13, m23, m33]
}

#[allow(unused)]
impl GltfInstance {
    pub fn to_raw(&self) -> InstanceRaw {
        let model =
            nalgebra::Matrix4::new_translation(&self.position) * rotation_to_matrix4(&self.rotation);
        InstanceRaw {
            model: model.into(),
            normal: rotation_to_matrix3(&self.rotation).into(),
        }
    }
}

#[allow(unused)]
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal: [[f32; 3]; 3],
}

impl Vertex for InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code though.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}