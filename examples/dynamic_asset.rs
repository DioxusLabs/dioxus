use dioxus::desktop::{use_asset_handler, wry::http::Response};
use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    use_asset_handler("logos", |request, response| {
        // We get the original path - make sure you handle that!
        if request.uri().path() != "/logos/logo.png" {
            return;
        }

        response.respond(Response::new(include_bytes!("./assets/logo.png").to_vec()));
    });

    rsx! {
        div {
            img { src: "/logos/logo.png" }
        }
    }
}
