use anyhow::anyhow;
use crate::engine::physics::state::RapierData;
use crate::state::real_view::level::*;
use crate::engine::prelude::*;
use crate::engine::renderer3d::renderer3d::*;

use nalgebra::*;
use num::Zero;
use rapier3d::prelude::*;
use wgpu::util::StagingBelt;
use crate::engine::physics::obj::Object;
use crate::state::real_view::renderer::portal::{PortalRenderer, PortalView};

// green
// blue
// purple

pub fn get_color_level(color: &str, zo: f32, p: &mut RapierData, gpu: &WgpuData, pr: &mut PlaneRenderer, res: &ResourceManager) -> anyhow::Result<Level> {
    let gf = res.textures.get(color).ok_or(anyhow!("NO TEXTURE"))?;
    let mut gfs = pr.create_plane(&gpu.device, Some(&gf.view));

    // floor
    add_plane(p, &mut gfs, &vector![0.0, 0.0, zo], 5.0, &Vector2::zeros(), 2.5, &Vector3::z(), &Vector3::x());
    // wall (or portal)
    add_plane(p, &mut gfs, &vector![0.0, 5.0, 5.0 + zo], 5.0, &Vector2::zeros(), 2.5, &-Vector3::y(), &Vector3::x());
    add_plane(p, &mut gfs, &vector![0.0, -5.0, 5.0 + zo], 5.0, &Vector2::zeros(), 2.5, &Vector3::y(), &Vector3::x());


    // // in fact we can add large
    // // floor
    // add_plane(p, &mut gfs, &vector![0.0, 0.0, zo], 5.0 * 1e1, &Vector2::zeros(), 2.5 * 1e1, &Vector3::z(), &Vector3::x());
    // // wall (or portal)
    // add_plane(p, &mut gfs, &vector![0.0, 5.0, 5.0 + zo], 5.0 * 1e1, &Vector2::zeros(), 2.5 * 1e1, &-Vector3::y(), &Vector3::x());
    // add_plane(p, &mut gfs, &vector![0.0, -5.0, 5.0 + zo], 5.0 * 1e1, &Vector2::zeros(), 2.5 * 1e1, &Vector3::y(), &Vector3::x());

    let mut planes = vec![];
    planes.push(gfs.to_static(&gpu.device));

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
    Ok(Level {
        portals: vec![],
        objs: planes,
        bundle,
    })
}


impl MagicLevel {
    pub fn level_loop(gpu: &WgpuData, pr: &mut PlaneRenderer, portal_renderer: &PortalRenderer, res: &ResourceManager) -> anyhow::Result<Self> {
        let mut levels = vec![];
        let mut p = RapierData::new();
        p.g.set_zero();

        levels.push(get_color_level("gf", 0.0, &mut p, gpu, pr, res)?);
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
            portal_views: (0..10).map(|_| PortalView::new(gpu, pr, portal_renderer)).collect(),
        };

        this.add_portal(gpu, pr, PortalPos {
            world: 0,
            pos: vector![5.0, 0.0, 1.0],
            out_normal: -Vector3::x(),
            up: Vector3::z(),
            width: 10.0,
        }, PortalPos {
            world: 0,
            pos: vector![-5.0, 0.0, 1.0],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 10.0,
        }, 10.0, 5.0, 10.0, 5.0, 1.0);

        // this.add_portal(gpu, pr, PortalPos {
        //     world: 0,
        //     pos: vector![0.0, 5.0, 1.0],
        //     out_normal: -Vector3::y(),
        //     up: Vector3::z(),
        //     width: 10.0,
        // }, PortalPos {
        //     world: 0,
        //     pos: vector![0.0, -5.0, 1.0],
        //     out_normal: Vector3::y(),
        //     up: Vector3::z(),
        //     width: 10.0,
        // }, 10.0, 5.0, 10.0, 5.0, 1.0);


        Ok(this)
    }
}