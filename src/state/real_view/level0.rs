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

fn normal_level(p: &mut RapierData, gpu: &WgpuData, pr: &mut PlaneRenderer, res: &ResourceManager) -> anyhow::Result<Level> {
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


    // short tunnel outside long inside
    add_plane(p, &mut bfs, &vector![0.0, 5.0, 1.0], 1.0, &Vector2::zeros(), 0.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, 3.0, 1.0], 1.0, &Vector2::zeros(), 0.5, &-Vector3::y(), &Vector3::x());


    // long tunnel outside short inside
    add_plane(p, &mut bfs, &vector![0.0, 8.0, -3.0], 5.0, &Vector2::zeros(), 2.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, 6.0, -3.0], 5.0, &Vector2::zeros(), 2.5, &-Vector3::y(), &Vector3::x());



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
    Ok(Level {
        portals: vec![],
        objs: planes,
        bundle,
    })
}

fn long_tunnel(p: &mut RapierData, gpu: &WgpuData, pr: &mut PlaneRenderer, res: &ResourceManager) -> anyhow::Result<Level> {
    let gf = res.textures.get("gf").ok_or(anyhow!("NO TEXTURE"))?;
    let bf = res.textures.get("bf").ok_or(anyhow!("NO TEXTURE"))?;
    let mut gfs = pr.create_plane(&gpu.device, Some(&gf.view));

    // we are in -1 ~ 1
    // but in facts 5
    // so -5 ~ 5
    add_plane(p, &mut gfs, &vector![0.0, 0.0, Z_OFFSET * 2.0], 10.0, &Vector2::zeros(), 25.0, &Vector3::z(), &Vector3::x());

    let mut bfs = pr.create_plane(&gpu.device, Some(&bf.view));
    add_plane(p, &mut bfs, &vector![0.0, 1.0, 5.0 + Z_OFFSET * 2.0], 5.0, &Vector2::zeros(), 2.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, -1.0, 5.0 + Z_OFFSET * 2.0], 5.0, &vector![0.5, 0.0], 2.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, 0.0, 2.0 + Z_OFFSET * 2.0], 5.0, &vector![0.5, 0.0], 2.5, &-Vector3::z(), &Vector3::x());


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
    Ok(Level {
        portals: vec![],
        objs: planes,
        bundle,
    })
}

fn long_inside(p: &mut RapierData, gpu: &WgpuData, pr: &mut PlaneRenderer, res: &ResourceManager) -> anyhow::Result<Level> {
    let gf = res.textures.get("gf").ok_or(anyhow!("NO TEXTURE"))?;
    let bf = res.textures.get("bf").ok_or(anyhow!("NO TEXTURE"))?;
    let mut gfs = pr.create_plane(&gpu.device, Some(&gf.view));

    // we are in -1 ~ 1
    // but in facts 5
    // so -5 ~ 5
    add_plane(p, &mut gfs, &vector![0.0, 0.0, Z_OFFSET * 10.0], 5.0, &Vector2::zeros(), 2.5, &Vector3::z(), &Vector3::x());

    let mut bfs = pr.create_plane(&gpu.device, Some(&bf.view));
    add_plane(p, &mut bfs, &vector![0.0, 1.0, 5.0 + Z_OFFSET * 10.0], 5.0, &Vector2::zeros(), 2.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, -1.0, 5.0 + Z_OFFSET * 10.0], 5.0, &vector![0.5, 0.0], 2.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, 0.0, 2.0 + Z_OFFSET * 10.0], 5.0, &vector![0.5, 0.0], 2.5, &-Vector3::z(), &Vector3::x());


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
    Ok(Level {
        portals: vec![],
        objs: planes,
        bundle,
    })
}

fn short_inside(p: &mut RapierData, gpu: &WgpuData, pr: &mut PlaneRenderer, res: &ResourceManager) -> anyhow::Result<Level> {
    let gf = res.textures.get("gf").ok_or(anyhow!("NO TEXTURE"))?;
    let bf = res.textures.get("bf").ok_or(anyhow!("NO TEXTURE"))?;
    let mut gfs = pr.create_plane(&gpu.device, Some(&gf.view));


    add_plane(p, &mut gfs, &vector![0.0, 0.0, Z_OFFSET * 15.0], 1.0, &vector![0.5, 0.0], 0.5, &Vector3::z(), &Vector3::x());

    let mut bfs = pr.create_plane(&gpu.device, Some(&bf.view));
    add_plane(p, &mut bfs, &vector![0.0, 1.0, 1.0 + Z_OFFSET * 15.0], 1.0, &Vector2::zeros(), 0.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, -1.0, 1.0 + Z_OFFSET * 15.0], 1.0, &vector![0.5, 0.0], 0.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, 0.0, 2.0 + Z_OFFSET * 15.0], 1.0, &vector![0.5, 0.0], 0.5, &-Vector3::z(), &Vector3::x());


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
    Ok(Level {
        portals: vec![],
        objs: planes,
        bundle,
    })
}

fn fat_tunnel(p: &mut RapierData, gpu: &WgpuData, pr: &mut PlaneRenderer, res: &ResourceManager) -> anyhow::Result<Level> {
    let gf = res.textures.get("gf").ok_or(anyhow!("NO TEXTURE"))?;
    let bf = res.textures.get("bf").ok_or(anyhow!("NO TEXTURE"))?;
    let pf = res.textures.get("pf").ok_or(anyhow!("NO TEXTURE"))?;
    let mut gfs = pr.create_plane(&gpu.device, Some(&gf.view));

    // we are in -1 ~ 1
    // but in facts 5
    // so -5 ~ 5
    add_plane(p, &mut gfs, &vector![0.0, 0.0, Z_OFFSET], 20.0, &Vector2::zeros(), 20.0, &Vector3::z(), &Vector3::x());

    let mut bfs = pr.create_plane(&gpu.device, Some(&bf.view));
    add_plane(p, &mut bfs, &vector![0.0, 5.0, 5.0 + Z_OFFSET], 5.0, &Vector2::zeros(), 2.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, -5.0, 5.0 + Z_OFFSET], 5.0, &vector![0.5, 0.0], 2.5, &Vector3::y(), &Vector3::x());
    add_plane(p, &mut bfs, &vector![0.0, 0.0, 10.0 + Z_OFFSET], 5.0, &vector![0.5, 0.0], 2.5, &-Vector3::z(), &Vector3::x());

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
    Ok(Level {
        portals: vec![],
        objs: planes,
        bundle,
    })
}

impl MagicLevel {
    pub fn level0(gpu: &WgpuData, pr: &mut PlaneRenderer, portal_renderer: &PortalRenderer, res: &ResourceManager) -> anyhow::Result<Self> {
        let mut levels = vec![];
        let mut p = RapierData::new();
        p.g.set_zero();

        levels.push(normal_level(&mut p, gpu, pr, res)?);
        levels.push(fat_tunnel(&mut p, gpu, pr, res)?);
        levels.push(long_tunnel(&mut p, gpu, pr, res)?);
        levels.push(long_inside(&mut p, gpu, pr, res)?);
        levels.push(short_inside(&mut p, gpu, pr, res)?);
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
            world: 1,
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
            world: 2,
            pos: vector![-5.0, 0.0, 1.0 + Z_OFFSET * 2.0],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 5.0,
        }, 1.0, 0.5, 1.0, 0.5, 1.0);

        // ^^^^^^^^^^^^^^^^^^^^^^^^^^^ end

        // long inside
        this.add_portal(gpu, pr, PortalPos {
            world: 0,
            pos: vector![-1.0, 4.0, 1.0],
            out_normal: -Vector3::x(),
            up: Vector3::z(),
            width: 1.0,
        }, PortalPos {
            world: 3,
            pos: vector![-5.0, 0.0, 1.0 + Z_OFFSET * 10.0],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 5.0,
        }, 1.0, 0.5, 1.0, 0.5, 1.0);
        this.add_portal(gpu, pr, PortalPos {
            world: 0,
            pos: vector![1.0, 4.0, 1.0],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 1.0,
        }, PortalPos {
            world: 3,
            pos: vector![5.0, 0.0, 1.0 + Z_OFFSET * 10.0],
            out_normal: -Vector3::x(),
            up: Vector3::z(),
            width: 5.0,
        }, 1.0, 0.5, 1.0, 0.5, 1.0);

        // short inside

        this.add_portal(gpu, pr, PortalPos {
            world: 0,
            pos: vector![-5.0, 7.0, 1.0],
            out_normal: -Vector3::x(),
            up: Vector3::z(),
            width: 1.0,
        }, PortalPos {
            world: 4,
            pos: vector![-1.0, 0.0, 1.0 + Z_OFFSET * 15.0],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 5.0,
        }, 1.0, 0.5, 1.0, 0.5, 1.0);
        this.add_portal(gpu, pr, PortalPos {
            world: 0,
            pos: vector![5.0, 7.0, 1.0],
            out_normal: Vector3::x(),
            up: Vector3::z(),
            width: 1.0,
        }, PortalPos {
            world: 4,
            pos: vector![1.0, 0.0, 1.0 + Z_OFFSET * 15.0],
            out_normal: -Vector3::x(),
            up: Vector3::z(),
            width: 5.0,
        }, 1.0, 0.5, 1.0, 0.5, 1.0);

        Ok(this)
    }
}