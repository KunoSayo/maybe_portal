use std::time::{Duration, Instant};

use egui::{Context, Frame};
use nalgebra::{point, vector};
use wgpu::{CommandEncoderDescriptor, TextureFormat};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, VirtualKeyCode, WindowEvent};
use winit::event::VirtualKeyCode::{Numpad0, Numpad8, Numpad9};
use winit::window::WindowLevel;

use crate::engine::{GameState, LoopState, StateData, StateEvent, Trans};
use crate::engine::render::camera::{Camera, CameraController};
use crate::engine::renderer3d::renderer3d::{General3DRenderer, LightUniform, PlaneRenderer};
use crate::engine::window::WindowInstance;
use crate::state::real_view::level::MagicLevel;
use crate::state::real_view::renderer::portal::PortalRenderer;

pub struct Test3DState {
    last_update: Option<Instant>,
    camera: Camera,
    controller: CameraController,
    level: Option<MagicLevel>,
    pr: Option<PortalRenderer>,
    size: (u32, u32),
    loc: PhysicalPosition<i32>,
    speed: f32,
}

pub struct OverlayView {
    state: &'static Test3DState,
}

impl Default for Test3DState {
    fn default() -> Self {
        Self {
            last_update: None,
            camera: Camera::new(point![-3.0, 0.0, 1.0]),
            controller: CameraController::new(),
            size: (0, 0),
            loc: Default::default(),
            speed: 1.0,
            level: None,
            pr: None,
        }
    }
}


impl Test3DState {
    fn load(&mut self, s: &mut StateData) {
        let gpu = s.app.gpu.as_ref().unwrap();
        s.app.world.insert(General3DRenderer::new(&gpu));


        let mut g3d = s.app.world.fetch_mut::<General3DRenderer>();
        let plane_renderer = &mut g3d.plane_renderer;
        plane_renderer.update_light(&gpu.queue, &LightUniform {
            light: vector![1.0, 1.0, 1.0],
            width: gpu.surface_cfg.width as f32,
            dir: -vector![1.0, 0.5, -0.875],
            height: gpu.surface_cfg.height as f32,
        });

        let pr = PortalRenderer::new(gpu, plane_renderer);
        self.level = Some(MagicLevel::new(gpu, plane_renderer, &pr, s.app.res.as_ref()).unwrap());
        self.pr = Some(pr);
    }
}

impl GameState for Test3DState {
    fn start(&mut self, s: &mut StateData) {
        if s.app.gpu.is_some() {
            self.load(s);
        }
    }

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        let now = Instant::now();
        if s.app.inputs.is_pressed(&[Numpad0]) {
            self.speed = 100.0;
        }
        if s.app.inputs.is_pressed(&[Numpad9]) {
            self.speed = 90.0;
        }
        if s.app.inputs.is_pressed(&[Numpad8]) {
            self.speed = 80.0;
        }
        if s.app.inputs.is_pressed(&[VirtualKeyCode::Numpad7]) {
            self.speed = 70.0;
        }
        if s.app.inputs.is_pressed(&[VirtualKeyCode::Numpad6]) {
            self.speed = 60.0;
        }
        if s.app.inputs.is_pressed(&[VirtualKeyCode::Numpad5]) {
            self.speed = 50.0;
        }
        if s.app.inputs.is_pressed(&[VirtualKeyCode::Numpad4]) {
            self.speed = 40.0;
        }
        if s.app.inputs.is_pressed(&[VirtualKeyCode::Numpad3]) {
            self.speed = 30.0;
        }
        if s.app.inputs.is_pressed(&[VirtualKeyCode::Numpad2]) {
            self.speed = 20.0;
        }
        if s.app.inputs.is_pressed(&[VirtualKeyCode::Numpad1]) {
            self.speed = 1.0;
        }
        let dt = self.last_update.map(|x| now.duration_since(x)).unwrap_or(Duration::from_millis(1)).as_secs_f32().min(0.05);
        let ddr = self.controller.update_direction(&mut self.camera);
        if let Some(level) = self.level.as_mut() {
            level.update(s, dt, &mut self.camera, &ddr);
        }

        self.last_update = Some(now);
        if self.controller.is_mouse_right_tracked {
            let size = s.app.window.inner_size();
            let x = self.controller.mouse_initial_position.x * size.width as f32;
            let y = self.controller.mouse_initial_position.y * size.height as f32;
            let _ = s.app.window.set_cursor_position(PhysicalPosition::new(x, y));
        }
        if s.app.inputs.is_pressed(&[VirtualKeyCode::Numpad6]) {
            let mut window = WindowInstance::new_with_gpu("See Point? NOPE!",
                                                          |x| x.with_transparent(true)
                                                              .with_window_level(WindowLevel::AlwaysOnTop),
                                                          s.wd.el, s.app.gpu.as_ref().unwrap()).unwrap();
            let mut sd = StateData {
                app: &mut window.app,
                wd: s.wd,
                dt: 0.0,
            };
            window.states.push(Box::new(OverlayView {
                state: unsafe { std::mem::transmute(self) },
            }));
            window.states.last_mut().unwrap().start(&mut sd);
            s.wd.new_windows.push(window);
        }
        (Trans::None, LoopState::POLL)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let gpu = s.app.gpu.as_mut().unwrap();
        let cfg = &gpu.surface_cfg;
        self.size.0 = cfg.width;
        self.size.1 = cfg.height;
        self.loc = s.app.window.inner_position().unwrap();
        let mut encoder = gpu.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Main Window Encoder") });
        gpu.uniforms.data.camera.update_view_proj(&self.camera);
        gpu.uniforms.update(&gpu.queue);

        if let  Some(mut g3d)= s.app.world.try_fetch_mut::<General3DRenderer>() {
            if let Some(apr) = self.pr.as_mut() {
                if let Some(level) = self.level.as_mut() {
                    egui::CentralPanel::default()
                        .frame(Frame::none())
                        .show(ctx, |ui| {
                            ui.label(format!("Eye: {:?}", self.camera.eye));
                            ui.label(format!("See dir: {:?}", self.camera.target));
                            ui.label(format!("World {}", level.me_world))
                        });
                    level.render(self.camera, &mut encoder, gpu, &mut g3d.plane_renderer, apr);
                }
            }
        }


        gpu.queue.submit(Some(encoder.finish()));


        Trans::None
    }

    fn on_event(&mut self, s: &mut StateData, e: StateEvent) {
        match e {
            StateEvent::ReloadGPU => {
                self.load(s);
            }
            StateEvent::Window(e) => {
                match e {
                    WindowEvent::Focused(false) => {
                        self.controller.is_mouse_right_pressed = false;
                        self.controller.is_mouse_right_tracked = false;
                        s.app.window.set_cursor_visible(true);
                    }
                    WindowEvent::KeyboardInput { device_id: _, input, is_synthetic: _ } => {
                        if let Some(key) = input.virtual_keycode.as_ref() {
                            self.controller.process_events(&input.state, key);
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        self.controller.process_mouse_moved(position, &s.app.window.inner_size());
                    }
                    WindowEvent::Resized(size) => {
                        if size.width > 1 && size.height > 1 {
                            if let Some(gpu) = s.app.gpu.as_ref() {
                                self.camera.aspect = size.width as f32 / size.height as f32;
                                if let Some(mut result) = s.app.world.try_fetch_mut::<PlaneRenderer>() {
                                    result.update_light(&gpu.queue, &LightUniform {
                                        light: vector![1.0, 1.0, 1.0],
                                        width: size.width as f32,
                                        dir: -vector![1.0, 0.5, -0.875],
                                        height: size.height as f32,
                                    })
                                }
                            }
                        }
                    }
                    WindowEvent::MouseInput { device_id, state, button, .. } => {
                        self.controller.process_mouse_input(device_id, state, button);
                        if button == &MouseButton::Right {
                            if state == &ElementState::Released {
                                s.app.window.set_cursor_visible(true);
                                let size = s.app.window.inner_size();
                                let x = self.controller.mouse_initial_position.x * size.width as f32;
                                let y = self.controller.mouse_initial_position.y * size.height as f32;
                                let _ = s.app.window.set_cursor_position(PhysicalPosition::new(x, y));
                            } else {
                                s.app.window.set_cursor_visible(false);
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl GameState for OverlayView {
    fn start(&mut self, _: &mut StateData) {
        todo!("Cool overlay?")
    }


    fn update(&mut self, _: &mut StateData) -> (Trans, LoopState) {
        (Trans::None, LoopState::POLL)
    }

    fn render(&mut self, s: &mut StateData, _: &Context) -> Trans {
        let this = self.state;
        if let Some(render) = s.app.gpu.as_mut() {
            render.views.check_extra_with_size("main screen", &render.device,
                                               (this.size.0, this.size.1), TextureFormat::Bgra8Unorm);
            render.views.check_extra_with_size("main screen depth", &render.device,
                                               (this.size.0, this.size.1), TextureFormat::Depth32Float);

            // if let Some(mut renderer) = s.app.world.try_fetch_mut::<ModelRenderer>() {
            //     let gpu = s.app.gpu.as_ref().unwrap();
            //
            //     let tex = gpu.views.get_extra("main screen").expect("HOW");
            //     let dep = gpu.views.get_extra("main screen depth").expect("HOW");
            //     renderer.update_camera(&this.camera);
            //
            //
            //     let mut encoder = gpu.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("overlay encoder") });
            //
            //     let _ = encoder.begin_clear_color(&gpu.views.get_screen().view, Color::TRANSPARENT, true);
            //     gpu.queue.submit(Some(encoder.finish()));
            //     let mut encoder = gpu.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("overlay encoder") });
            //
            //     let mut rp = encoder.begin_with_depth(&tex.view,
            //                                           LoadOp::Clear(Color {
            //                                               r: 0.0,
            //                                               g: 0.0,
            //                                               b: 0.0,
            //                                               a: 0.75,
            //                                           }),
            //                                           &dep.view,
            //                                           LoadOp::Clear(1.0));
            //     renderer.render(&mut rp, gpu, &this.model[..]);
            //     drop(rp);
            //
            //     let parent_window_loc = this.loc;
            //     let my_loc = s.app.window.inner_position().unwrap();
            //     // the left-top pos in the parent
            //
            //
            //     // the offset from parent to my
            //     let offset = (my_loc.x - parent_window_loc.x, my_loc.y - parent_window_loc.y);
            //
            //     let parent_has_width = (this.size.0 as i32 - offset.0).min(this.size.0 as _);
            //     let parent_has_height = (this.size.1 as i32 - offset.1).min(this.size.1 as _);
            //
            //     // the copy src start point.
            //     let img_start = (offset.0.max(0), offset.1.max(0));
            //
            //     let my_start = (offset.0.min(0).abs(), offset.1.min(0).abs());
            //     let my_width = s.app.window.inner_size().width as i32 - my_start.0;
            //     let my_height = s.app.window.inner_size().height as i32 - my_start.1;
            //
            //     let final_size = (parent_has_width.min(my_width), parent_has_height.min(my_height));
            //
            //     if final_size.0 <= 0 || final_size.1 <= 0 {
            //         return Trans::None;
            //     }
            //
            //
            //     encoder.copy_texture_to_texture(ImageCopyTexture {
            //         texture: &tex.texture,
            //         mip_level: 0,
            //         origin: Origin3d {
            //             x: img_start.0 as _,
            //             y: img_start.1 as _,
            //             z: 0,
            //         },
            //         aspect: Default::default(),
            //     }, ImageCopyTexture {
            //         texture: &gpu.views.get_screen().texture,
            //         mip_level: 0,
            //         origin: Origin3d {
            //             x: my_start.0 as _,
            //             y: my_start.1 as _,
            //             z: 0,
            //         },
            //         aspect: Default::default(),
            //     }, Extent3d {
            //         width: final_size.0 as _,
            //         height: final_size.1 as _,
            //         depth_or_array_layers: 1,
            //     });
            //     gpu.queue.submit(Some(encoder.finish()));
            // }
        }
        Trans::None
    }
}