use anyhow::Result;
use dioxus::prelude::*;

pub fn main() -> Result<()> {
    init_logging();

    // Right now we're going through dioxus-desktop but we'd like to go through dioxus-mobile
    // That will seed the index.html with some fixes that prevent the page from scrolling/zooming etc
    dioxus_desktop::launch_cfg(
        app,
        // Note that we have to disable the viewport goofiness of the browser.
        // Dioxus_mobile should do this for us
        Config::default().with_custom_index(include_str!("index.html").to_string()),
    );

    Ok(())
}

fn app(cx: Scope) -> Element {
    let items = use_state(cx, || vec![1, 2, 3]);

    render! {
        div {
            h1 { "Hello, Mobile"}
            div { margin_left: "auto", margin_right: "auto", width: "200px", padding: "10px", border: "1px solid black",
                button {
                    onclick: move|_| {
                        let mut _items = items.make_mut();
                        let len = _items.len() + 1;
                        _items.push(len);
                    },
                    "Add item"
                }
                for item in items.iter() {
                    div { "- {item}" }
                }
            }
        }
    }
}

#[cfg(target_os = "android")]
fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_min_level(log::Level::Trace)
            .with_tag("rustnl-ios"),
    );
}

#[cfg(not(target_os = "android"))]
fn init_logging() {
    env_logger::init();
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn stop_unwind<F: FnOnce() -> T, T>(f: F) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("attempt to unwind out of `rust` with err: {:?}", err);
            std::process::abort()
        }
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn _start_app() {
    main().unwrap();
}

use dioxus_desktop::Config;
#[cfg(target_os = "android")]
use wry::android_binding;

#[no_mangle]
#[inline(never)]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub extern "C" fn start_app() {
    #[cfg(target_os = "android")]
    android_binding!(com_example, rustnl_ios, _start_app);
    #[cfg(target_os = "ios")]
    _start_app()
}
