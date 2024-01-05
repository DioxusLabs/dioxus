use dioxus::prelude::*;
use dioxus_desktop::wry::http::Response;
use dioxus_desktop::{use_asset_handler, AssetRequest};
use std::path::Path;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    use_asset_handler(cx, |request: &AssetRequest| {
        let path = request.path().to_path_buf();
        async move {
            if path != Path::new("logo.png") {
                return None;
            }
            let image_data: &[u8] = include_bytes!("./assets/logo.png");
            Some(Response::new(image_data.into()))
        }
    });

    cx.render(rsx! {
        div {
            img {
                src: "logo.png"
            }
        }
    })
}
