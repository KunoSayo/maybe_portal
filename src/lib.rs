use winit::dpi::PhysicalSize;
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::window::WindowBuilder;

use crate::engine::window::{EventLoopMessage, WindowManager};
use crate::state::real_view::test_view::Test3DState;

mod engine;
mod state;

pub fn real_main() {
    _main(EventLoopBuilder::with_user_event().build());
}

fn _main(event_loop: EventLoop<EventLoopMessage>) {
    println!("[Std Stream] Joined the real main");
    eprintln!("[Err Stream] Joined the real main");
    log::info!("[Log Info] Joined the real main");
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


#[no_mangle]
#[cfg(feature = "android")]
#[cfg(target_os = "android")]
pub fn android_main(app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    use winit::event_loop::EventLoopBuilder;

    std::env::set_var("RUST_BACKTRACE", "full");

    android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Trace));
    let el = EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build();
    _main(el);
}
