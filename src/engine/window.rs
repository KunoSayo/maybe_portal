use std::cell::RefCell;
use std::collections::HashSet;
use std::default::Default;
use std::ops::DerefMut;

use egui::Context;
use egui::epaint::ahash::{HashMap, HashMapExt};
use egui_wgpu::renderer::ScreenDescriptor;
use log::info;
use specs::World;
use wgpu::{Color, CommandEncoderDescriptor, Extent3d, ImageCopyTexture, LoadOp,
           Operations, Origin3d, RenderPassColorAttachment, RenderPassDescriptor, TextureAspect};
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, DeviceEventFilter, EventLoop, EventLoopProxy, EventLoopWindowTarget};
use winit::window::{Window, WindowBuilder, WindowId};

use crate::engine::{GameState, GlobalData, LoopState, MainRendererData, Pointer, StateEvent, Trans, WgpuData};
use crate::engine::app::AppInstance;

#[derive(Default)]
struct LoopInfo {
    pressed_keys: HashSet<VirtualKeyCode>,
    released_keys: HashSet<VirtualKeyCode>,
    loop_state: LoopState,
    got_event: bool,
}

impl LoopInfo {
    pub(crate) fn updated(&mut self) {
        self.got_event = false;
    }
}

pub struct WindowInstance {
    pub id: WindowId,
    pub app: AppInstance,
    pub states: Vec<Box<dyn GameState>>,
    running: bool,
    loop_info: LoopInfo,
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EventLoopMessage {
    WakeUp(WindowId),
}

pub type EventLoopTargetType = EventLoopWindowTarget<EventLoopMessage>;
pub type EventLoopProxyType = EventLoopProxy<EventLoopMessage>;


impl WindowInstance {
    pub fn is_running(&self) -> bool {
        self.running
    }
    #[allow(unused)]
    pub fn stop_run(&mut self) {
        self.running = false;
    }
}


#[allow(unused)]
impl WindowInstance {
    pub fn new_with_gpu(title: &str, setup: impl FnOnce(WindowBuilder) -> WindowBuilder, el: &EventLoopTargetType, gpu: &WgpuData) -> anyhow::Result<Self> {
        let window = setup(WindowBuilder::new()
            .with_title(title))
            .build(el)
            .unwrap();
        let id = window.id();
        let app = AppInstance::create_from_gpu(window, el, gpu)?;
        Ok(Self {
            id,
            app,
            states: vec![],
            running: true,
            loop_info: Default::default(),
        })
    }

    pub fn new(title: &str, setup: impl FnOnce(WindowBuilder) -> WindowBuilder, el: &EventLoopTargetType) -> anyhow::Result<Self> {
        let window = setup(WindowBuilder::new()
            .with_title(title))
            .build(el)
            .unwrap();
        let id = window.id();
        let app = AppInstance::new(window, el)?;
        Ok(Self {
            id,
            app,
            states: vec![],
            running: true,
            loop_info: Default::default(),
        })
    }

    pub fn new_from_window(window: Window, el: &EventLoopTargetType) -> anyhow::Result<Self> {
        Ok(Self {
            id: window.id(),
            app: AppInstance::new(window, el)?,
            states: vec![],
            running: true,
            loop_info: Default::default(),
        })
    }
}
/// put app and el here
macro_rules! get_state {
    ($app: expr, $el: expr) => {{

        crate::engine::state::StateData {
            app: &mut $app,
            wd: $el,
            dt: 0.0
        }
    }};
}

impl WindowInstance {
    fn loop_once(&mut self, wd: &mut GlobalData) {
        profiling::scope!("Loop logic once");
        self.loop_info.loop_state = LoopState::WAIT_ALL;

        self.app.inputs.swap_frame();
        {
            for x in &mut self.states {
                self.loop_info.loop_state |= x.shadow_update();
            }
            if let Some(last) = self.states.last_mut() {
                let ((tran, l), wd) = {
                    let mut state_data = get_state!(self.app, wd);
                    (last.update(&mut state_data), state_data.wd)
                };
                self.process_tran(tran, wd);
                self.loop_info.loop_state |= l;
            }
        }
    }


    fn process_tran(&mut self, tran: Trans, el: &mut GlobalData) {
        let last = self.states.last_mut().unwrap();
        let mut state_data = get_state!(self.app, el);
        match tran {
            Trans::Push(mut x) => {
                x.start(&mut state_data);
                self.states.push(x);
            }
            Trans::Pop => {
                last.stop(&mut state_data);
                self.states.pop().unwrap();
            }
            Trans::Switch(x) => {
                last.stop(&mut state_data);
                *last = x;
                last.start(&mut state_data);
            }
            Trans::Exit => {
                while let Some(mut last) = self.states.pop() {
                    last.stop(&mut state_data);
                }
                self.running = false;
            }
            Trans::Vec(ts) => {
                for t in ts {
                    self.process_tran(t, el);
                }
            }
            Trans::None => {}
        }
    }


    fn render_once(&mut self, el: &mut GlobalData) {
        if let (Some(gpu), ) = (&self.app.gpu, ) {
            profiling::scope!("Render pth once");
            let render_now = std::time::Instant::now();
            let render_dur = render_now.duration_since(self.app.last_render_time);
            let dt = render_dur.as_secs_f32();
            let swap_chain_frame = if let Ok(s) = gpu.surface.get_current_texture() { s } else {
                // it is normal.
                return;
            };
            let surface_output = &swap_chain_frame;
            {
                let mut encoder = gpu.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Clear Encoder") });
                let _ = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &gpu.views.get_screen().view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });
                gpu.queue.submit(Some(encoder.finish()));
            }

            let egui_ctx = &self.app.egui_ctx.clone();
            let full_output = egui_ctx.run(self.app.egui_state.take_egui_input(&self.app.window), |egui_ctx| {
                let mut state_data = get_state!(self.app, el);
                state_data.dt = dt;


                for game_state in &mut self.states {
                    game_state.shadow_render(&mut state_data, egui_ctx);
                }
                if let Some(g) = self.states.last_mut() {
                    let tran = g.render(&mut state_data, egui_ctx);
                    self.process_tran(tran, el);
                }
            });
            let gpu = self.app.gpu.as_ref().unwrap();
            let render = self.app.render.as_mut().unwrap();
            // render ui output to main screen
            {
                let device = gpu.device.as_ref();
                let queue = gpu.queue.as_ref();
                let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("encoder for egui"),
                });

                let screen_descriptor = ScreenDescriptor {
                    size_in_pixels: [gpu.surface_cfg.width, gpu.surface_cfg.height],
                    pixels_per_point: self.app.window.scale_factor() as f32,
                };
                // Upload all resources for the GPU.

                let egui_renderer = &mut render.egui_rpass;
                let paint_jobs = self.app.egui_ctx.tessellate(full_output.shapes);
                for (id, delta) in &full_output.textures_delta.set {
                    egui_renderer.update_texture(device, queue, *id, &delta);
                }
                egui_renderer.update_buffers(&device, &queue, &mut encoder, &paint_jobs, &screen_descriptor);
                {
                    let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &gpu.views.get_screen().view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Load,
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });
                    egui_renderer.render(
                        &mut rp,
                        &paint_jobs,
                        &screen_descriptor,
                    );
                }

                // Submit the commands.
                queue.submit(std::iter::once(encoder.finish()));
                full_output.textures_delta.free.iter().for_each(|id| egui_renderer.free_texture(id));
            }
            {
                let mut sd = get_state!(self.app, el);
                sd.dt = dt;
                self.states.iter_mut().for_each(|s| s.on_event(&mut sd, StateEvent::PostUiRender));
            }
            let gpu = self.app.gpu.as_ref().unwrap();

            {
                let mut encoder = gpu.device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("Copy buffer to screen commands")
                });
                let size = gpu.get_screen_size();
                encoder.copy_texture_to_texture(ImageCopyTexture {
                    texture: &gpu.views.get_screen().texture,
                    mip_level: 0,
                    origin: Origin3d::default(),
                    aspect: TextureAspect::All,
                }, ImageCopyTexture {
                    texture: &surface_output.texture,
                    mip_level: 0,
                    origin: Default::default(),
                    aspect: TextureAspect::All,
                }, Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                });
                gpu.queue.submit(Some(encoder.finish()));
            }

            // if self.window.inputs.is_pressed(&[VirtualKeyCode::F11]) {
            //     self.window.save_screen_shots();
            // }
            //
            // self.window.pools.render_pool.try_run_one();

            self.app.last_render_time = render_now;
            swap_chain_frame.present();
            self.app.egui_state.handle_platform_output(&self.app.window, &self.app.egui_ctx, full_output.platform_output);
        } else {
            // no gpu but we need render it...
            // well...
            // no idea.
        }
    }

    fn start(&mut self, mut start: impl GameState, wd: &mut GlobalData) {
        start.start(&mut get_state!(self.app, wd));
        info!("Started the start state.");
        self.states.push(Box::new(start));
    }

    fn on_window_event(&mut self, we: &WindowEvent, wd: &mut GlobalData) {
        self.loop_info.got_event = true;
        let _ = self.app.egui_state.on_event(&self.app.egui_ctx, we);
        let sd = &mut get_state!(self.app, wd);
        for x in &mut self.states {
            x.on_event(sd, StateEvent::Window(we));
        }
        match we {
            WindowEvent::Touch(touch) => {
                self.app.inputs.points.insert(touch.id, Pointer::from(*touch));
            }
            WindowEvent::KeyboardInput {
                input,
                is_synthetic,
                ..
            } => {
                if !is_synthetic {
                    if let Some(key) = input.virtual_keycode {
                        match input.state {
                            ElementState::Pressed => {
                                self.loop_info.pressed_keys.insert(key);
                            }
                            ElementState::Released => {
                                self.loop_info.released_keys.insert(key);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}


pub struct WindowManager {
    root: WindowId,
    windows: HashMap<WindowId, RefCell<Box<WindowInstance>>>,
}

impl WindowManager {
    pub(crate) fn new(window: Window, el: &EventLoopTargetType) -> anyhow::Result<Self> {
        let root = window.id();
        let mut windows = HashMap::new();
        windows.insert(root, RefCell::new(Box::new(WindowInstance::new_from_window(window, el)?)));
        Ok(Self {
            root,
            windows,
        })
    }


    pub(crate) fn run_loop(mut self, event_loop: EventLoop<EventLoopMessage>, start: impl GameState) {
        let proxy = event_loop.create_proxy();
        let mut world = World::default();
        {
            let mut created_windows = Vec::new();
            let mut wd = GlobalData { el: &event_loop, elp: &proxy, windows: &self.windows, new_windows: &mut created_windows, world: &mut world };
            let root_window_ins = self.windows.get(&self.root).unwrap();
            root_window_ins.borrow_mut().start(start, &mut wd);
            for x in created_windows {
                let id = x.app.window.id();
                self.windows.insert(id, RefCell::new(Box::new(x)));
            }
        }
        event_loop.set_device_event_filter(DeviceEventFilter::Always);
        event_loop.run(move |event, el, control_flow| {
            log::trace!(target: "winit_event", "{:?}", event);

            if *control_flow == ControlFlow::Exit {
                // Although exit, there are some events.
                return;
            }

            let mut created_windows = Vec::new();

            {
                if let Event::WindowEvent { window_id, event, } = &event {
                    if let Some(window) = self.windows.get(window_id) {
                        let mut wd = GlobalData { el, elp: &proxy, windows: &self.windows, new_windows: &mut created_windows, world: &mut world };
                        window.borrow_mut().on_window_event(event, &mut wd);
                    }
                }
            }
            match event {
                Event::NewEvents(_) => {
                    profiling::finish_frame!();
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                } => {
                    self.windows.remove(&window_id);
                    if window_id == self.root {
                        *control_flow = ControlFlow::Exit
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Destroyed,
                    window_id
                } => {
                    self.windows.remove(&window_id);
                    if window_id == self.root {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Event::UserEvent(user_event) => {
                    match user_event {
                        EventLoopMessage::WakeUp(id) => {
                            if let Some(this) = self.windows.get_mut(&id) {
                                *control_flow = ControlFlow::Poll;
                                this.get_mut().loop_info.got_event = true;
                            }
                        }
                    }
                }
                Event::Suspended => {
                    #[cfg(target_os = "android")]
                    {
                        self.window.gpu = None;
                    }
                }
                Event::Resumed => {
                    for (_, this) in &self.windows {
                        let mut this = this.borrow_mut();
                        if this.app.gpu.is_none() {
                            info!("gpu not found, try to init");
                            this.app.gpu = WgpuData::new(&this.app.window).ok();
                            if let Some(gpu) = &this.app.gpu {
                                this.app.render = Some(MainRendererData::new(gpu, &this.app.res));
                                let mut gd = GlobalData { el, elp: &proxy, windows: &self.windows, new_windows: &mut created_windows, world: &mut world };
                                let WindowInstance {
                                    ref mut app,
                                    ref mut states,
                                    ..
                                } = this.deref_mut().deref_mut();
                                let sd = &mut get_state!(*app, &mut gd);
                                states.iter_mut().for_each(|x| x.on_event(sd, StateEvent::ReloadGPU));
                            }
                            this.app.egui_ctx = Context::default();
                            let size = this.app.window.inner_size();
                            this.app.egui_ctx.set_pixels_per_point(this.app.window.scale_factor() as f32);
                            let WindowInstance {
                                ref mut app,
                                ..
                            } = this.deref_mut().deref_mut();
                            let _ = app.egui_state.on_event(&app.egui_ctx, &WindowEvent::Resized(size));
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(size), window_id
                } => {
                    if let Some(this) = self.windows.get_mut(&window_id) {
                        if !this.get_mut().is_running() {
                            self.windows.remove(&window_id);
                        } else if size.width > 1 && size.height > 1 {
                            let this = this.get_mut();
                            if let Some(gpu) = &mut this.app.gpu {
                                info!("Window resized, telling gpu data");
                                gpu.resize(size.width, size.height);
                                match &mut this.app.render {
                                    Some(_) => {}
                                    _ => {
                                        this.app.render = Some(MainRendererData::new(gpu, &this.app.res));
                                    }
                                }
                            }
                        }
                    }
                }
                Event::RedrawRequested(w) => {
                    if let Some(this) = self.windows.get(&w) {
                        let mut this = this.borrow_mut();
                        let mut wd = GlobalData { el, elp: &proxy, windows: &self.windows, new_windows: &mut created_windows, world: &mut world };
                        this.render_once(&mut wd);
                    }
                }
                Event::MainEventsCleared => {
                    let mut not_running = vec![];
                    let mut f_ls = LoopState::WAIT_ALL;
                    for (id, this) in &self.windows {
                        let mut this = this.borrow_mut();
                        let this = this.deref_mut();
                        if !this.loop_info.got_event && this.loop_info.loop_state.control_flow == ControlFlow::Wait {
                            continue;
                        }
                        if !this.loop_info.pressed_keys.is_empty() || !this.loop_info.released_keys.is_empty() {
                            log::trace!(target: "InputTrace", "process window {:?} pressed_key {:?} and released {:?}", id, this.loop_info.pressed_keys, this.loop_info.released_keys);
                            this.app.inputs.process(&this.loop_info.pressed_keys, &this.loop_info.released_keys);
                            this.loop_info.pressed_keys.clear();
                            this.loop_info.released_keys.clear();
                        }
                        if this.states.is_empty() {
                            this.running = false;
                        }
                        if this.running {
                            let mut wd = GlobalData { el, elp: &proxy, windows: &self.windows, new_windows: &mut created_windows, world: &mut world };
                            this.loop_once(&mut wd);
                            let ls = this.loop_info.loop_state;
                            if ls.render {
                                this.app.window.request_redraw();
                            }
                            this.loop_info.loop_state = ls;
                            f_ls |= ls;
                        } else {
                            not_running.push(*id);
                            if id == &self.root {
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                        this.loop_info.updated();
                    }
                    *control_flow = f_ls.control_flow;
                    for id in not_running {
                        self.windows.remove(&id);
                    }
                }
                _ => {}
            }

            for x in created_windows {
                self.windows.insert(x.id, RefCell::new(Box::new(x)));
            }
        });
    }
}


