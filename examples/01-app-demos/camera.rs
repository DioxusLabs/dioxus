//! Camera
//!
//! This example demonstrates how to use camera APIs through web-sys-x. On web it uses the browser
//! bindings, and on desktop it uses the web-sys-compatible webview bindings provided by web-sys-x.

use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        Camera {}
    }
}

#[component]
fn Camera() -> Element {
    let mut status = use_signal(|| "Camera is stopped.".to_string());
    let mut streaming = use_signal(|| false);
    let mut video = use_signal(|| None::<web_sys::HtmlVideoElement>);

    let start = move |_| {
        let Some(video) = video.cloned() else {
            status.set("Camera preview is not mounted yet.".to_string());
            return;
        };

        status.set("Requesting camera access...".to_string());

        spawn(async move {
            match start_camera(video).await {
                Ok(()) => {
                    streaming.set(true);
                    status.set("Camera stream is active.".to_string());
                }
                Err(err) => {
                    streaming.set(false);
                    status.set(format!(
                        "Could not start the camera: {}",
                        js_error_message(err)
                    ));
                }
            }
        });
    };

    let stop = move |_| {
        if let Some(video) = video.cloned() {
            stop_camera(video);
            streaming.set(false);
            status.set("Camera is stopped.".to_string());
        }
    };

    rsx! {
        main {
            style: "font-family: system-ui, sans-serif; min-height: 100vh; display: grid; place-items: center; background: #f5f7fa; color: #1f2937;",
            section {
                style: "width: min(720px, calc(100vw - 32px)); display: grid; gap: 16px;",
                h1 { style: "font-size: 2rem; margin: 0;", "Camera bindings" }
                p { style: "margin: 0; color: #4b5563;", "{status}" }
                video {
                    onmounted: move |event| {
                        let element = event.as_web_event();
                        match element.dyn_into::<web_sys::HtmlVideoElement>() {
                            Ok(element) => video.set(Some(element)),
                            Err(_) => status.set("Mounted element is not a video element.".to_string()),
                        }
                    },
                    autoplay: true,
                    muted: true,
                    playsinline: true,
                    style: "width: 100%; aspect-ratio: 16 / 9; background: #111827; border-radius: 8px; object-fit: cover;",
                }
                div {
                    style: "display: flex; gap: 8px; flex-wrap: wrap;",
                    button {
                        disabled: streaming(),
                        onclick: start,
                        style: "padding: 10px 14px; border: 0; border-radius: 6px; background: #2563eb; color: white; font-weight: 600; cursor: pointer;",
                        "Start camera"
                    }
                    button {
                        disabled: !streaming(),
                        onclick: stop,
                        style: "padding: 10px 14px; border: 0; border-radius: 6px; background: #374151; color: white; font-weight: 600; cursor: pointer;",
                        "Stop camera"
                    }
                }
            }
        }
    }
}

async fn start_camera(video: web_sys::HtmlVideoElement) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is not available"))?;
    let media_devices = window.navigator().media_devices()?;

    let constraints = web_sys::MediaStreamConstraints::new();
    constraints.set_video_bool(true);
    constraints.set_audio_bool(false);

    let stream = JsFuture::from(media_devices.get_user_media_with_constraints(&constraints)?)
        .await?
        .dyn_into::<web_sys::MediaStream>()?;

    video.set_src_object(Some(&stream));
    JsFuture::from(video.play()?).await?;

    Ok(())
}

fn stop_camera(video: web_sys::HtmlVideoElement) {
    if let Some(stream) = video.src_object() {
        for track in stream.get_tracks().iter() {
            if let Ok(track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
                track.stop();
            }
        }
    }

    video.set_src_object(None);
}

fn js_error_message(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}
