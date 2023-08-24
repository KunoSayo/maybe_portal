use crossbeam::channel::{Receiver, unbounded};
use log::trace;
use nalgebra::Vector3;
use rapier3d::control::EffectiveCharacterMovement;
use rapier3d::prelude::*;

use crate::engine::physics::obj::KinematicObject;

pub struct RapierData {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub query_pipeline: QueryPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub g: Vector3<Real>,
    pub col_events: Receiver<CollisionEvent>,
    pub contact_events: Receiver<ContactForceEvent>,
    collector: ChannelEventCollector,
}

#[allow(unused)]
impl RapierData {
    pub fn new() -> Self {
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();
        let integration_parameters = IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = BroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();

        let (s1, col_events) = unbounded();
        let (s2, contact_events) = unbounded();
        let collector = ChannelEventCollector::new(s1, s2);
        Self {
            rigid_body_set,
            collider_set,
            integration_parameters,
            physics_pipeline,
            query_pipeline: Default::default(),
            island_manager,
            broad_phase,
            narrow_phase,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
            g: vector![0.0, 0.0, -9.81],
            col_events,
            contact_events,
            collector,
        }
    }

    pub fn step(&mut self, dt: Real) {
        self.integration_parameters.dt = dt;
        while let Ok(e) = self.col_events.try_recv() {
            trace!(target: "physics", "unused col event: {:?}", e);
        }
        while let Ok(e) = self.contact_events.try_recv() {}
        self.physics_pipeline.step(&self.g, &self.integration_parameters,
                                   &mut self.island_manager,
                                   &mut self.broad_phase,
                                   &mut self.narrow_phase,
                                   &mut self.rigid_body_set,
                                   &mut self.collider_set,
                                   &mut self.impulse_joint_set,
                                   &mut self.multibody_joint_set,
                                   &mut self.ccd_solver,
                                   Some(&mut self.query_pipeline),
                                   &(),
                                   &self.collector);
    }

    pub fn move_obj(&mut self, dt: Real, obj: &KinematicObject, target: Vector<Real>) -> EffectiveCharacterMovement {
        let me = &self.rigid_body_set[obj.handle];
        let collider = &self.collider_set[obj.collider_handle];
        let filter = QueryFilter::default().exclude_rigid_body(obj.handle);
        let mut ecm = obj.controller.move_shape(dt,
                                                &self.rigid_body_set,
                                                &self.collider_set,
                                                &self.query_pipeline,
                                                collider.shape(),
                                                me.position(),
                                                target,
                                                filter,
                                                |_| {},
        );
        ecm
    }
}