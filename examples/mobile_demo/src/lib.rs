use anyhow::Result;
use dioxus::desktop::Config;
use dioxus::prelude::*;
#[cfg(target_os = "android")]
use wry::android_binding;

#[cfg(target_os = "android")]
fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_min_level(log::Level::Trace)
            .with_tag("mobile-demo"),
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
    stop_unwind(|| main().unwrap());
}

#[no_mangle]
#[inline(never)]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub extern "C" fn start_app() {
    #[cfg(target_os = "android")]
    android_binding!(com_example, mobile_demo, _start_app);
    #[cfg(target_os = "ios")]
    _start_app()
}

pub fn main() -> Result<()> {
    init_logging();

    // Right now we're going through dioxus-desktop but we'd like to go through dioxus-mobile
    // That will seed the index.html with some fixes that prevent the page from scrolling/zooming etc
    LaunchBuilder::new().cfg(
        // Note that we have to disable the viewport goofiness of the browser.
        // Dioxus_mobile should do this for us
        Config::default().with_custom_index(include_str!("index.html").to_string()),
    );

    Ok(())
}

fn app() -> Element {
    let items = cx.use_hook(|| vec![1, 2, 3]);

    log::debug!("Hello from the app");

    rsx! {
        div {
            h1 { "Hello, Mobile" }
            div {
                margin_left: "auto",
                margin_right: "auto",
                width: "200px",
                padding: "10px",
                border: "1px solid black",
                button {
                    onclick: move |_| {
                        println!("Clicked!");
                        items.push(items.len());
                        cx.needs_update_any(ScopeId::ROOT);
                        println!("Requested update");
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
