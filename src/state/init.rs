use std::sync::Arc;
use std::sync::atomic::Ordering;

use futures::task::SpawnExt;
use log::error;
use wgpu::{Device, Queue};

use crate::engine::{GameState, LoopState, ResourceManager, StateData, StateEvent, Trans, WaitFutureState, WaitResult};
use crate::engine::global::{INITED, IO_POOL};

pub struct InitState {
    start_state: Option<Box<dyn GameState + Send + 'static>>,
}

impl InitState {
    pub fn new(state: Box<dyn GameState + Send + 'static>) -> Self {
        Self {
            start_state: Some(state),
        }
    }
}

async fn load_texture(a_d: Arc<Device>, a_q: Arc<Queue>, a_r: Arc<ResourceManager>) -> anyhow::Result<()> {
    let device = unsafe { std::mem::transmute::<_, &'static _>(a_d.as_ref()) };
    let queue = unsafe { std::mem::transmute::<_, &'static _>(a_q.as_ref()) };
    let res = unsafe { std::mem::transmute::<_, &'static ResourceManager>(a_r.as_ref()) };
    for x in [
        res.load_texture_async(device, queue, "bf".into(), "texture/floor/blue.png"),
        res.load_texture_async(device, queue, "gf".into(), "texture/floor/green.png"),
        res.load_texture_async(device, queue, "pf".into(), "texture/floor/purple.png"),
        res.load_texture_async(device, queue, "rf".into(), "texture/floor/red.png")
    ]
        .map(|task| IO_POOL.spawn_with_handle(task))
    {
        x?.await?;
    }

    anyhow::Ok(())
}


impl GameState for InitState {
    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        if let Some(gpu) = s.app.gpu.as_ref() {
            let state = self.start_state.take().unwrap();
            let device = gpu.device.clone();
            let queue = gpu.queue.clone();
            let res = s.app.res.clone();
            let handle = IO_POOL.spawn_with_handle(async move {
                let device = device;
                let queue = queue;
                let res = res;
                let task = async move {
                    if !INITED.load(Ordering::Acquire) {
                        // Lazy::force(&GLOBAL_DATA);
                    }
                    load_texture(device, queue, res).await?;

                    anyhow::Ok(())
                };
                if let Err(e) = task.await {
                    error!("Load failed for {:?}", e);
                    WaitResult::Exit
                } else {
                    WaitResult::Function(Box::new(|_| {
                        // s.app.egui_ctx.set_fonts(GLOBAL_DATA.font.clone());
                        Trans::Switch(state)
                    }))
                }
            }).expect("Spawn init task failed");


            (Trans::Push(WaitFutureState::from_wait_thing(handle)), LoopState::POLL_WITHOUT_RENDER)
        } else {
            (Trans::None, LoopState::WAIT_ALL)
        }
    }

    fn on_event(&mut self, s: &mut StateData, e: StateEvent) {
        if matches!(e, StateEvent::ReloadGPU) {
            let gpu = s.app.gpu.as_ref().expect("I FOUND GPU");
            println!("block on loading");
            futures::executor::block_on(load_texture(gpu.device.clone(), gpu.queue.clone(), s.app.res.clone()))
                .expect("Load texture failed");
            println!("block end");
        }
    }
}
