use nalgebra::{Vector, vector, Vector3};
use num::Zero;
use rapier3d::control::{CharacterLength, KinematicCharacterController};
use rapier3d::prelude::{Collider, ColliderBuilder, ColliderHandle, RigidBody, RigidBodyHandle};

use crate::engine::physics::state::RapierData;

pub struct KinematicObject {
    pub controller: KinematicCharacterController,
    pub handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
}

#[allow(unused)]
impl KinematicObject {
    pub fn new(p: &mut RapierData, r: RigidBody, c: Collider) -> Self {
        let controller = KinematicCharacterController {
            up: Vector::z_axis(),
            offset: CharacterLength::Absolute(0.125),
            ..Default::default()
        };
        let handle = p.rigid_body_set.insert(r);
        let collider_handle = p.collider_set.insert_with_parent(c, handle, &mut p.rigid_body_set);
        Self { controller, collider_handle, handle }
    }
}


pub struct Object {
    pub handle: RigidBodyHandle,
    pub body_bounding: ColliderHandle,
    pub collider_handle: ColliderHandle,
}

#[allow(unused)]
impl Object {
    pub fn new(p: &mut RapierData, r: RigidBody, c: Collider) -> Self {
        let handle = p.rigid_body_set.insert(r);
        let body_bounding = p.collider_set
            .insert_with_parent(ColliderBuilder::cuboid(0.125, 0.125, 1.0),
                                handle, &mut p.rigid_body_set);
        let collider_handle = p.collider_set.insert_with_parent(c, handle, &mut p.rigid_body_set);
        Self { collider_handle, handle, body_bounding }
    }

    pub fn calc_vel(&self, p: &mut RapierData, camera_mov: &Vector3<f32>, running: bool) {
        let ddr = camera_mov.component_mul(&vector![1.0, 1.0, 0.0]);
        let me = &mut p.rigid_body_set[self.handle];
        if !ddr.is_zero() {
            let speed = if running {
                4.0
            } else {
                2.0
            };
            me.set_linvel((speed * ddr.normalize()) + vector![0.0, 0.0, 0.0], true);
        } else {
            me.set_linvel(Vector3::zeros(), true);
        }
    }

    pub fn add_vel(&self, p: &mut RapierData, delta: &Vector3<f32>) {
        let me = &mut p.rigid_body_set[self.handle];
        me.set_linvel(me.linvel() + delta, true);
    }
}