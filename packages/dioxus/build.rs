fn main() {
    // Warn the user if they enabled the launch feature without any renderers
    if feature_enabled("launch") {
        if feature_enabled("third-party-renderer") {
            return;
        }

        let liveview_renderers = ["liveview", "axum"];
        let fullstack_renderers = ["axum"];
        let client_renderers = ["desktop", "mobile", "web", "tui"];
        let client_renderer_selected = client_renderers
            .iter()
            .any(|renderer| feature_enabled(renderer));
        if feature_enabled("fullstack") {
            let server_fullstack_enabled = fullstack_renderers
                .iter()
                .any(|renderer| feature_enabled(renderer));
            if !server_fullstack_enabled && !client_renderer_selected {
                println!("cargo:warning=You have enabled the launch and fullstack features, but have not enabled any renderers. The application will not be able to launch. Try enabling one of the following renderers: {} for the server or one of the following renderers: {} for the client.", fullstack_renderers.join(", "), client_renderers.join(", "));
            }
        }

        if feature_enabled("liveview") {
            let server_selected = liveview_renderers
                .iter()
                .any(|renderer| feature_enabled(renderer));
            if !server_selected {
                println!("cargo:warning=You have enabled the launch and liveview features, but have not enabled any liveview renderers. The application will not be able to launch. Try enabling one of the following renderers: {}", liveview_renderers.join(", "));
            }
        }

        if !client_renderer_selected {
            println!("cargo:warning=You have enabled the launch feature, but have not enabled any client renderers. The application will not be able to launch. Try enabling one of the following renderers: {}, fullstack or liveview", client_renderers.join(", "));
        }
    }
}

fn feature_enabled(feature: &str) -> bool {
    let feature = "CARGO_FEATURE_".to_owned() + &feature.to_uppercase().replace('-', "_");
    println!("cargo:rerun-if-env-changed={}", feature);
    std::env::var(feature).is_ok()
}
