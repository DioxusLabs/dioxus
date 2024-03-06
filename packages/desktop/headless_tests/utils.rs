#![allow(unused)] // for whatever reason, the compiler is not recognizing the use of these functions

use dioxus::prelude::*;
use dioxus_core::Element;

pub fn check_app_exits(app: fn() -> Element) {
    use dioxus_desktop::tao::window::WindowBuilder;
    use dioxus_desktop::Config;
    // This is a deadman's switch to ensure that the app exits
    let should_panic = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let should_panic_clone = should_panic.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(60));
        if should_panic_clone.load(std::sync::atomic::Ordering::SeqCst) {
            eprintln!("App did not exit in time");
            std::process::exit(exitcode::SOFTWARE);
        }
    });

    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(WindowBuilder::new().with_visible(true)))
        .launch(app);

    // Stop deadman's switch
    should_panic.store(false, std::sync::atomic::Ordering::SeqCst);
}

pub static EXPECTED_EVENTS: GlobalSignal<usize> = Signal::global(|| 0);

pub fn mock_event(id: &'static str, value: &'static str) {
    mock_event_with_extra(id, value, "");
}

pub fn mock_event_with_extra(id: &'static str, value: &'static str, extra: &'static str) {
    use_hook(move || {
        EXPECTED_EVENTS.with_mut(|x| *x += 1);

        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let js = format!(
                r#"
                let event = {value};
                let element = document.getElementById('{id}');
                {extra}
                element.dispatchEvent(event);
                "#
            );

            eval(&js).await.unwrap();
        });
    })
}
