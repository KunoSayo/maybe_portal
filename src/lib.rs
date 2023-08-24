use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopBuilder;
use winit::window::WindowBuilder;

use crate::engine::window::WindowManager;
use crate::state::real_view::test_view::Test3DState;

mod engine;
mod state;


pub fn real_main() {
    println!("[Std Stream] Joined the real main");
    eprintln!("[Err Stream] Joined the real main");
    log::info!("[Log Info] Joined the real main");
    let event_loop = EventLoopBuilder::with_user_event().build();
    let is_3d = std::env::var("3d").map(|x| x == "1").unwrap_or(true);
    let window = WindowBuilder::new()
        .with_title(if is_3d { "3D" } else { "RustMeeting" })
        .with_inner_size(PhysicalSize::new(1600, 900))
        .build(&event_loop)
        .unwrap();

    log::info!("Got the window");

    match WindowManager::new(window, &event_loop) {
        Ok(am) => {
            log::info!("Got the main application");
            am.run_loop(event_loop, state::InitState::new(Box::new(Test3DState::default())));
        }
        Err(e) => {
            log::error!("Init the app manager failed for {:?}", e);
            eprintln!("Init the app manager failed for {:?}", e);
        }
    }
}


#[cfg_attr(target_os = "android", ndk_glue::main(logger(level = "info", tag = "andy")))]
pub fn main() {
    std::env::set_var("RUST_BACKTRACE", "full");
    real_main();
}
