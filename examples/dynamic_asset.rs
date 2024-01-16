use dioxus::prelude::*;
use dioxus_desktop::{use_asset_handler, wry::http::Response};

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    use_asset_handler("logos", |request, response| {
        // Note that the "logos" prefix is stripped from the URI
        //
        // However, the asset is absolute to its "virtual folder" - meaning it starts with a leading slash
        if request.uri().path() != "/logo.png" {
            return;
        }

        response.respond(Response::new(include_bytes!("./assets/logo.png").to_vec()));
    });

    render! {
        div {
            img { src: "/logos/logo.png" }
        }
    }
}
