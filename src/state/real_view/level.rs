use std::array::from_ref;
use std::collections::HashMap;

use anyhow::anyhow;
use egui::epaint::ahash::HashSet;
use log::{debug, info, trace};
use nalgebra::{Matrix4, Point3, vector, Vector2, Vector3};
use num::Zero;
use rapier3d::dynamics::LockedAxes;
use rapier3d::pipeline::ActiveEvents;
use rapier3d::prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder};
use wgpu::{Color, CommandEncoder, LoadOp, Operations, RenderBundle, RenderBundleDepthStencil, RenderBundleDescriptor, RenderBundleEncoderDescriptor, RenderPass, RenderPassDepthStencilAttachment, RenderPassDescriptor, TextureFormat};
use wgpu::util::StagingBelt;
use winit::event::VirtualKeyCode;

use crate::engine::{ResourceManager, StateData, WgpuData};
use crate::engine::physics::obj::Object;
use crate::engine::physics::state::RapierData;
use crate::engine::render::camera::Camera;
use crate::engine::render_ext::CommandEncoderExt;
use crate::engine::renderer3d::renderer3d::{PlaneObject, PlaneRenderer, Planes, StaticPlanes};
use crate::state::real_view::renderer::portal::{PortalRenderer, PortalView};

pub struct Level {
    portals: Vec<Portal>,
    objs: Vec<StaticPlanes>,
    bundle: RenderBundle,
}

#[derive(Copy, Clone, Debug)]
struct PortalPos {
    world: usize,
    pos: Vector3<f32>,
    /// from the door to outside normal
    out_normal: Vector3<f32>,
    up: Vector3<f32>,
    width: f32,
}

#[derive(Debug)]
struct Portal {
    plane: PlaneObject,
    portal_render: StaticPlanes,
    this: PortalPos,
    /// (world, portal index)
    connecting: (usize, usize),
    scale: f32,
}

const Z_OFFSET: f32 = -15.0;


fn add_plane(p: &mut RapierData, planes: &mut Planes, center: &Vector3<f32>, r: f32, tex: &Vector2<f32>, tex_delta: f32, up: &Vector3<f32>, right: &Vector3<f32>) {
    let v = (vector![1.0, 1.0, 1.0] - up.abs()) * r;
    let f = if up.dot(&Vector3::z()).is_zero() { 0.0 } else { 1.0 };
    p.collider_set.insert(ColliderBuilder::cuboid(v.x, v.y, v.z)
        .translation(*center)
        .friction(f)
        .build());
    planes.objs.push(PlaneObject::new(center, r, tex, tex_delta, up, right));
}


impl Level {
    pub fn normal_level(p: &mut RapierData, gpu: &WgpuData, pr: &mut PlaneRenderer, res: &ResourceManager) -> anyhow::Result<Self> {
        let gf = res.textures.get("gf").ok_or(anyhow!("NO TEXTURE"))?;
        let bf = res.textures.get("bf").ok_or(anyhow!("NO TEXTURE"))?;
        let pf = res.textures.get("pf").ok_or(anyhow!("NO TEXTURE"))?;
        let mut gfs = pr.create_plane(&gpu.device, Some(&gf.view));

        add_plane(p, &mut gfs, &Vector3::zeros(), 10.0, &Vector2::zeros(), 5.0, &Vector3::z(), &Vector3::x());

        let mut bfs = pr.create_plane(&gpu.device, Some(&bf.view));
        add_plane(p, &mut bfs, &vector![0.0, 1.0, 1.0], 1.0, &Vector2::zeros(), 0.5, &Vector3::y(), &Vector3::x());
        add_plane(p, &mut bfs, &vector![0.0, -1.0, 1.0], 1.0, &Vector2::zeros(), 0.5, &-Vector3::y(), &Vector3::x());
        add_plane(p, &mut bfs, &vector![0.0, 0.0, 2.0], 1.0, &Vector2::zeros(), 0.5, &Vector3::z(), &Vector3::x());

        // long tunnel wall
        add_plane(p, &mut bfs, &vector![4.0, 2.0, 1.0], 1.0, &Vector2::zeros(), 0.5, &Vector3::y(), &Vector3::x());
        add_plane(p, &mut bfs, &vector![4.0, 0.0, 1.0], 1.0, &Vector2::zeros(), 0.5, &-Vector3::y(), &Vector3::x());

        let mut pfs = pr.create_plane(&gpu.device, Some(&pf.view));
        pfs.objs.push(PlaneObject::new(&vector![-1.0, 0.0, 1.0], 1.0, &Vector2::zeros(), 0.5, &-Vector3::x(), &Vector3::y()));

        let mut planes = vec![];
        planes.push(gfs.to_static(&gpu.device));
        planes.push(bfs.to_static(&gpu.device));

        let mut bundle = gpu.device.create_render_bundle_encoder(&RenderBundleEncoderDescriptor {
            label: None,
            color_formats: &[Some(gpu.surface_cfg.format)],
            depth_stencil: Some(RenderBundleDepthStencil {
                format: TextureFormat::Depth32Float,
                depth_read_only: false,
                stencil_read_only: false,
            }),
            sample_count: 1,
            multiview: None,
        });
        bundle.set_pipeline(&pr.normal_rp);
        pr.bind(&mut bundle);
        pr.render_static(&mut bundle, gpu, &planes[..]);
        let bundle = bundle.finish(&RenderBundleDescriptor {
            label: None,
        });
        Ok(Self {
            portals: vec![],
            objs: planes,
            bundle,
        })
    }

    pub fn long_tunnel(p: &mut RapierData, gpu: &WgpuData, pr: &mut PlaneRenderer, res: &ResourceManager) -> anyhow::Result<Self> {
        let gf = res.textures.get("gf").ok_or(anyhow!("NO TEXTURE"))?;
        let bf = res.textures.get("bf").ok_or(anyhow!("NO TEXTURE"))?;
        let pf = res.textures.get("pf").ok_or(anyhow!("NO TEXTURE"))?;
        let mut gfs = pr.create_plane(&gpu.device, Some(&gf.view));

        // we are in -1 ~ 1
        // but in facts 5
        // so -5 ~ 5
        add_plane(p, &mut gfs, &vector![0.0, 0.0, Z_OFFSET * 2.0], 10.0, &Vector2::zeros(), 25.0, &Vector3::z(), &Vector3::x());

        let mut bfs = pr.create_plane(&gpu.device, Some(&bf.view));
        add_plane(p, &mut bfs, &vector![0.0, 1.0, 5.0 + Z_OFFSET * 2.0], 5.0, &Vector2::zeros(), 2.5, &Vector3::y(), &Vector3::x());
        add_plane(p, &mut bfs, &vector![0.0, -1.0, 5.0 + Z_OFFSET * 2.0], 5.0, &Vector2::zeros(), 2.5, &Vector3::y(), &Vector3::x());
        add_plane(p, &mut bfs, &vector![0.0, 0.0, 10.0 + Z_OFFSET * 2.0], 5.0, &Vector2::zeros(), 2.5, &-Vector3::z(), &Vector3::x());

        let mut pfs = pr.create_plane(&gpu.device, Some(&pf.view));
        pfs.objs.push(PlaneObject::new(&vector![-1.0, 0.0, 1.0 + Z_OFFSET * 2.0 - 9.0], 5.0, &Vector2::zeros(), 2.5, &Vector3::x(), &Vector3::y()));

        let mut planes = vec![];
        planes.push(gfs.to_static(&gpu.device));
        planes.push(bfs.to_static(&gpu.device));

        let mut bundle = gpu.device.create_render_bundle_encoder(&RenderBundleEncoderDescriptor {
            label: None,
            color_formats: &[Some(gpu.surface_cfg.format)],
            depth_stencil: Some(RenderBundleDepthStencil {
                format: TextureFormat::Depth32Float,
                depth_read_only: false,
                stencil_read_only: false,
            }),
            sample_count: 1,
            multiview: None,
        });
        bundle.set_pipeline(&pr.no_cull_rp);
        pr.bind(&mut bundle);
        pr.render_static(&mut bundle, gpu, &planes[..]);
        let bundle = bundle.finish(&RenderBundleDescriptor {
            label: None,
        });
        Ok(Self {
            portals: vec![],
            objs: planes,
            bundle,
        })
    }


    pub fn fat_tunnel(p: &mut RapierData, gpu: &WgpuData, pr: &mut PlaneRenderer, res: &ResourceManager) -> anyhow::Result<Self> {
        let gf = res.textures.get("gf").ok_or(anyhow!("NO TEXTURE"))?;
        let bf = res.textures.get("bf").ok_or(anyhow!("NO TEXTURE"))?;
        let pf = res.textures.get("pf").ok_or(anyhow!("NO TEXTURE"))?;
        let mut gfs = pr.create_plane(&gpu.device, Some(&gf.view));

        // we are in -1 ~ 1
        // but in facts 5
        // so -5 ~ 5
        add_plane(p, &mut gfs, &vector![0.0, 0.0, Z_OFFSET], 10.0, &Vector2::zeros(), 25.0, &Vector3::z(), &Vector3::x());

        let mut bfs = pr.create_plane(&gpu.device, Some(&bf.view));
        add_plane(p, &mut bfs, &vector![0.0, 5.0, 5.0 + Z_OFFSET], 5.0, &Vector2::zeros(), 2.5, &Vector3::y(), &Vector3::x());
        add_plane(p, &mut bfs, &vector![0.0, -5.0, 5.0 + Z_OFFSET], 5.0, &Vector2::zeros(), 2.5, &Vector3::y(), &Vector3::x());
        add_plane(p, &mut bfs, &vector![0.0, 0.0, 10.0 + Z_OFFSET], 5.0, &Vector2::zeros(), 2.5, &-Vector3::z(), &Vector3::x());

        let mut pfs = pr.create_plane(&gpu.device, Some(&pf.view));
        pfs.objs.push(PlaneObject::new(&vector![-1.0, 0.0, 1.0 + Z_OFFSET], 5.0, &Vector2::zeros(), 2.5, &Vector3::x(), &Vector3::y()));

        let mut planes = vec![];
        planes.push(gfs.to_static(&gpu.device));
        planes.push(bfs.to_static(&gpu.device));

        let mut bundle = gpu.device.create_render_bundle_encoder(&RenderBundleEncoderDescriptor {
            label: None,
            color_formats: &[Some(gpu.surface_cfg.format)],
            depth_stencil: Some(RenderBundleDepthStencil {
                format: TextureFormat::Depth32Float,
                depth_read_only: false,
                stencil_read_only: false,
            }),
            sample_count: 1,
            multiview: None,
        });
        bundle.set_pipeline(&pr.no_cull_rp);
        pr.bind(&mut bundle);
        pr.render_static(&mut bundle, gpu, &planes[..]);
        let bundle = bundle.finish(&RenderBundleDescriptor {
            label: None,
        });
        Ok(Self {
            portals: vec![],
            objs: planes,
            bundle,
        })
    }

    pub fn render<'a>(&'a self, rp: &mut RenderPass<'a>, _: &WgpuData, _: &mut PlaneRenderer) {
        rp.execute_bundles(std::iter::once(&self.bundle));
    }

    fn add_portal(&mut self, p: &mut RapierData, gpu: &WgpuData, _pr: &PlaneRenderer, this: PortalPos, r: f32, tex_delta: f32, scale: f32) -> (ColliderHandle, usize) {
        let right = if this.out_normal.xy().is_zero() {
            Vector3::x()
        } else {
            vector![this.out_normal.y, -this.out_normal.x, 0.0]
        };

        let plane = PlaneObject::new(&this.pos, r, &Vector2::zeros(), tex_delta, &this.out_normal, &right);
        let planes = Planes { objs: vec![plane], texture_bind: None }.to_static(&gpu.device);

        let v = (vector![1.0, 1.0, 1.0] - this.out_normal.abs()) * (r - 0.0625);
        let handle = p.collider_set.insert(ColliderBuilder::cuboid(v.x, v.y, v.z)
            .sensor(true)
            .translation(this.pos)
            .active_events(ActiveEvents::all())
            .build());
        let idx = self.portals.len();
        self.portals.push(Portal {
            plane,
            portal_render: planes,
            this,
            connecting: (0, 0),
            scale,
        });
        (handle, idx)
    }
}

pub struct MagicLevel {
    pub levels: Vec<Level>,
    pub p: RapierData,
    pub me: Object,
    pub me_world: usize,
    /// (Col world, portal index)
    pub portals_map: HashMap<ColliderHandle, (usize, usize)>,
    staging_belt: StagingBelt,
    portal_views: Vec<PortalView>,
}

#[derive(Debug, Copy, Clone)]
struct Coord {
    forward: f32,
    up: f32,
    right: f32,

    target_forward: f32,
    target_up: f32,
    target_right: f32,
}

fn will_see_face(view: &Matrix4<f32>, plane: &PlaneObject) -> bool {
    let mut mn_x = 2.0;
    let mut mx_x = -2.0;
    let mut mn_y = 2.0;
    let mut mx_y = -2.0;
    let mut front = false;
    for x in plane.vertex {
        let mut result = view * vector![x.pos.x, x.pos.y, x.pos.z, 1.0];
        result /= result.w;
        if result.z >= 0.0 && result.z <= 1.0 {
            front = true;
        }
        mn_x = result.x.min(mn_x);
        mx_x = result.x.max(mx_x);
        mn_y = result.y.min(mn_y);
        mx_y = result.y.max(mx_y);
    }

    if front {
        true
    } else {
        false
    }
}

impl Coord {
    /// Get the coord in the portal view
    fn from_camera_portal(camera: &Camera, portal: &Portal) -> Coord {
        let dis = (camera.eye - portal.this.pos) * portal.scale;
        let forward = portal.this.out_normal.dot(&dis.coords);
        let up = portal.this.up.dot(&dis.coords);
        let right = portal.this.up.cross(&portal.this.out_normal).dot(&dis.coords);


        let target_forward = portal.this.out_normal.dot(&camera.target);
        let target_up = portal.this.up.dot(&camera.target);
        let target_right = portal.this.up.cross(&portal.this.out_normal).dot(&camera.target);
        Coord {
            forward,
            up,
            right,
            target_forward,
            target_up,
            target_right,
        }
    }

    fn from_camera_portal_for_view(camera: &Camera, portal: &Portal) -> Coord {
        let dis = camera.eye - portal.this.pos;
        let forward = portal.this.out_normal.dot(&dis.coords);
        let up = portal.this.up.dot(&dis.coords) * portal.scale;

        let right = {
            let right = portal.this.up.cross(&portal.this.out_normal).dot(&dis.coords);
            if right.abs() >= portal.this.width {
                let delta = right.abs() - portal.this.width;
                right.signum() * (portal.this.width * portal.scale + delta)
            } else {
                right * portal.scale
            }
        };

        let target_forward = portal.this.out_normal.dot(&camera.target);
        let target_up = portal.this.up.dot(&camera.target);
        let target_right = portal.this.up.cross(&portal.this.out_normal).dot(&camera.target);
        Coord {
            forward,
            up,
            right,
            target_forward,
            target_up,
            target_right,
        }
    }

    fn change_camera_without_forward(&self, camera: &mut Camera, portal: &PortalPos) {
        let result = portal.up * self.up
            // + portal.out_normal * self.forward
            // - for we changed the right.
            - portal.up.cross(&portal.out_normal) * self.right
            + portal.pos;
        camera.eye = result.into();

        let result = portal.up * self.target_up
            - portal.out_normal * self.target_forward
            + portal.up.cross(&-portal.out_normal) * self.target_right;
        camera.target = result;
    }

    fn change_camera_for_portal(&self, camera: &mut Camera, portal: &PortalPos) {
        let result = portal.up * self.up
            - portal.out_normal * self.forward
            // - for we changed the right.
            - portal.up.cross(&portal.out_normal) * self.right
            + portal.pos;
        camera.eye = result.into();

        let result = portal.up * self.target_up
            - portal.out_normal * self.target_forward
            + portal.up.cross(&-portal.out_normal) * self.target_right;
        camera.target = result;
    }
}


impl MagicLevel {
    fn add_portal(&mut self, gpu: &WgpuData, pr: &PlaneRenderer, p1: PortalPos, p2: PortalPos, r1: f32, tex_delta1: f32, r2: f32, tex_delta2: f32, scale: f32) {
        let (handle, idx) = self.levels[p1.world].add_portal(&mut self.p, gpu, pr, p1, r1, tex_delta1, scale);
        let (handle2, idx2) = self.levels[p2.world].add_portal(&mut self.p, gpu, pr, p2, r2, tex_delta2, 1.0 / scale);

        self.levels[p1.world].portals[idx].connecting = (p2.world, idx2);
        self.levels[p2.world].portals[idx2].connecting = (p1.world, idx);

        self.portals_map.insert(handle, (p1.world, idx));
        self.portals_map.insert(handle2, (p2.world, idx2));
    }

    pub fn new(gpu: &WgpuData, pr: &mut PlaneRenderer, portal_renderer: &PortalRenderer, res: &ResourceManager) -> anyhow::Result<Self> {
        let mut levels = vec![];
        let mut p = RapierData::new();
        p.g.set_zero();

        levels.push(Level::normal_level(&mut p, gpu, pr, res)?);
        levels.push(Level::fat_tunnel(&mut p, gpu, pr, res)?);
        levels.push(Level::long_tunnel(&mut p, gpu, pr, res)?);
        let me = RigidBodyBuilder::dynamic()
            .translation(vector![-3.0, 3.0, 1.0])
            .locked_axes(LockedAxes::ROTATION_LOCKED)
            .ccd_enabled(true)
            .build();
        let me_col = ColliderBuilder::cuboid(0.01, 0.01, 1.0)
            .translation(vector![0.0, 0.0, 0.0])
            .friction(0.0)
            .build();

        let me = Object::new(&mut p, me, me_col);

        let mut this = Self {
            levels,
            p,
            me,
            me_world: 0,
            portals_map: Default::default(),
            staging_belt: StagingBelt::new(32768 * 2),
            portal_views: (0..5).map(|_| PortalView::new(gpu, pr, portal_renderer)).collect(),
        };
        // -------------- from normal level to fat level
        this.add_portal(gpu, pr, PortalPos {
            world: 0,
            pos: vector![1.0, 0.0, 1.0],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 1.0,
        }, PortalPos {
            world: 1,
            pos: vector![5.0, 0.0, 1.0 + Z_OFFSET],
            out_normal: -Vector3::x(),
            up: Vector3::z(),
            width: 5.0,
        }, 1.0, 0.5, 5.0, 2.5, 5.0);

        this.add_portal(gpu, pr, PortalPos {
            world: 0,
            pos: vector![-1.0, 0.0, 1.0],
            out_normal: -Vector3::x(),
            up: Vector3::z(),
            width: 1.0,
        }, PortalPos {
            world: 1,
            pos: vector![-5.0, 0.0, 1.0 + Z_OFFSET],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 5.0,
        }, 1.0, 0.5, 5.0, 2.5, 5.0);
        // ^^^^^^^^^^^^^^^^^^^^^^^^^^^ end

        // -------------- from normal level to long tunnel
        this.add_portal(gpu, pr, PortalPos {
            world: 0,
            pos: vector![5.0, 1.0, 1.0],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 1.0,
        }, PortalPos {
            world: 2,
            pos: vector![5.0, 0.0, 1.0 + Z_OFFSET * 2.0],
            out_normal: -Vector3::x(),
            up: Vector3::z(),
            width: 1.0,
        }, 1.0, 0.5, 1.0, 0.5, 1.0);

        this.add_portal(gpu, pr, PortalPos {
            world: 0,
            pos: vector![3.0, 1.0, 1.0],
            out_normal: -Vector3::x(),
            up: Vector3::z(),
            width: 1.0,
        }, PortalPos {
            world: 1,
            pos: vector![-5.0, 0.0, 1.0 + Z_OFFSET * 2.0],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 5.0,
        }, 1.0, 0.5, 1.0, 0.5, 1.0);

        // ^^^^^^^^^^^^^^^^^^^^^^^^^^^ end
        Ok(this)
    }


    pub fn update(&mut self, s: &mut StateData, dt: f32, camera: &mut Camera, ddr: &Vector3<f32>) {
        self.p.integration_parameters.dt = dt;

        self.me.calc_vel(&mut self.p, ddr, s.app.inputs.cur_frame_input.pressing.contains(&VirtualKeyCode::LShift));
        self.p.step(dt);
        let mut coled = HashSet::default();
        while let Ok(event) = self.p.col_events.try_recv() {
            trace!(target:"level::col", "Got col event {:?}", event);
            if event.stopped() {
                continue;
            }
            let portal_handle = if event.collider1() == self.me.collider_handle {
                event.collider2()
            } else {
                event.collider1()
            };
            if let Some((world, idx)) = self.portals_map.get(&portal_handle) {
                if !coled.insert((*world, *idx)) {
                    continue;
                }
                let portal = &self.levels[*world].portals[*idx];
                let before = camera.eye;
                let camera_view = Coord::from_camera_portal(camera, portal);
                let connecting = &self.levels[portal.connecting.0].portals[portal.connecting.1].this;
                camera_view.change_camera_without_forward(camera, connecting);

                camera.eye.z = connecting.pos.z;
                camera.eye += connecting.out_normal * 0.02;

                self.p.rigid_body_set[self.me.handle].set_translation(camera.eye.coords, true);
                if let Some(c) = self.p.collider_set[self.me.body_bounding].shape_mut().as_cuboid_mut() {
                    c.half_extents.x *= portal.scale;
                    c.half_extents.y *= portal.scale;
                }
                info!(target: "level", "From world {} to world {}", self.me_world, connecting.world);
                self.me_world = connecting.world;
                debug!(target:"level", "{:?} with {:?} => {:?}", before, camera_view, camera.eye);
            }
        }

        camera.eye = Point3::from(*self.p.rigid_body_set[self.me.handle].translation());
    }
    //
    pub fn render_in_portal(&mut self, (world, idx): (usize, usize), rec_dep: usize,
                            camera: Camera,
                            ce: &mut CommandEncoder,
                            gpu: &mut WgpuData,
                            pr: &mut PlaneRenderer,
                            portal_renderer: &mut PortalRenderer)
    {
        gpu.uniforms.data.camera.update_view_proj(&camera);
        gpu.uniforms.update_staging(&gpu.device, ce, &mut self.staging_belt);

        let pv = &self.portal_views[rec_dep];
        let level = &self.levels[world];
        let portal = &level.portals[idx];
        // first render the portal frame
        {
            let mut rp = ce.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render portal depth pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &pv.pd.texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1000.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            pr.bind(&mut rp);
            rp.set_pipeline(&pr.depth_only_rp);
            pr.render_static(&mut rp, gpu, from_ref(&portal.portal_render));
        }
        {
            // then render scenes
            let mut rp = ce.begin_with_depth(&pv.color.view, LoadOp::Clear(Color::BLACK),
                                             &pv.depth.view, LoadOp::Clear(1.0));
            pr.bind(&mut rp);
            rp.set_pipeline(&portal_renderer.portal_view_rp);
            rp.set_bind_group(2, &pv.pd.bindgroup, &[]);
            pr.render_static(&mut rp, gpu, &level.objs);
        }


        // next dep will overflow
        if rec_dep + 1 >= self.portal_views.len() {
            return;
        }
        for p_world in 0..self.levels.len() {
            for portal_idx in 0..self.levels[p_world].portals.len() {
                if idx == portal_idx && p_world == world {
                    continue;
                }

                let this_portal = &self.levels[p_world].portals[portal_idx];
                if (this_portal.this.pos.z - camera.eye.z).abs() > 5.0 {
                    continue;
                }
                if !will_see_face(&gpu.uniforms.data.camera.view_proj, &this_portal.plane) {
                    continue;
                }

                // check this is not the portal between me && view
                let portal_me = this_portal.this.pos - camera.eye.coords;
                let portal_view = this_portal.this.pos - self.levels[world].portals[idx].this.pos;
                if portal_me.normalize().dot(&portal_view.normalize()) < 0.0 {
                    continue;
                }

                trace!(target:"level", "We can see portal at world {p_world} [{portal_idx}] (dep={})", rec_dep);

                let connecting = &self.levels[this_portal.connecting.0].portals[this_portal.connecting.1];
                let camera_coord = Coord::from_camera_portal_for_view(&camera, &this_portal);
                let mut portal_camera = camera;
                camera_coord.change_camera_for_portal(&mut portal_camera, &connecting.this);


                self.render_in_portal(this_portal.connecting, rec_dep + 1, portal_camera, ce, gpu, pr, portal_renderer);

                gpu.uniforms.data.camera.update_view_proj(&camera);
                gpu.uniforms.update_staging(&gpu.device, ce, &mut self.staging_belt);

                // render the result to screen
                let cpv = &self.portal_views[rec_dep];
                let mut rp = ce.begin_with_depth(&cpv.color.view, LoadOp::Load,
                                                 &cpv.depth.view, LoadOp::Load);
                let this_portal = &self.levels[p_world].portals[portal_idx];

                pr.bind(&mut rp);
                rp.set_bind_group(1, &self.portal_views[rec_dep + 1].color_bind, &[]);
                rp.set_bind_group(2, &cpv.pd.bindgroup, &[]);
                rp.set_pipeline(&portal_renderer.render_portal_view_rp);
                pr.render_static(&mut rp, gpu, from_ref(&this_portal.portal_render));
            }
        }
    }

    pub fn render<'a>(&'a mut self, camera: Camera,
                      ce: &mut CommandEncoder,
                      gpu: &mut WgpuData,
                      pr: &mut PlaneRenderer,
                      portal_renderer: &mut PortalRenderer) {
        self.staging_belt.recall();
        if self.portal_views[0].color.info.width != gpu.surface_cfg.width || self.portal_views[0].color.info.height != gpu.surface_cfg.height {
            for x in &mut self.portal_views {
                *x = PortalView::new(gpu, pr, portal_renderer);
            }
        }


        {
            let mut rp = ce.begin_with_depth(&gpu.views.get_screen().view, LoadOp::Clear(Color::BLACK),
                                             &gpu.views.get_depth_view().view, LoadOp::Clear(1.0));
            let level = &self.levels[self.me_world];
            level.render(&mut rp, gpu, pr);
        }

        for world in 0..self.levels.len() {
            for portal_idx in 0..self.levels[world].portals.len() {
                let this_portal = &self.levels[world].portals[portal_idx];

                if !will_see_face(&gpu.uniforms.data.camera.view_proj, &this_portal.plane) {
                    continue;
                }
                if (this_portal.this.pos.z - camera.eye.z).abs() > 5.0 {
                    continue;
                }

                trace!(target:"level", "We can see portal at world {} [{portal_idx}]", world);
                let connecting = &self.levels[this_portal.connecting.0].portals[this_portal.connecting.1];
                let camera_coord = Coord::from_camera_portal_for_view(&camera, &this_portal);
                let mut portal_camera = camera;
                camera_coord.change_camera_for_portal(&mut portal_camera, &connecting.this);


                self.render_in_portal(this_portal.connecting, 0, portal_camera, ce, gpu, pr, portal_renderer);

                gpu.uniforms.data.camera.update_view_proj(&camera);
                gpu.uniforms.update_staging(&gpu.device, ce, &mut self.staging_belt);

                // render the result to screen

                let mut rp = ce.begin_with_depth(&gpu.views.get_screen().view, LoadOp::Load,
                                                 &gpu.views.get_depth_view().view, LoadOp::Load);
                let this_portal = &self.levels[world].portals[portal_idx];

                pr.bind(&mut rp);
                rp.set_bind_group(1, &self.portal_views[0].color_bind, &[]);
                rp.set_pipeline(&pr.screen_tex_no_cull_rp);
                pr.render_static(&mut rp, gpu, from_ref(&this_portal.portal_render));
            }
        }
        self.staging_belt.finish();
    }
}