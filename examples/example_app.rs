//! Example App
//! --------------
//!
//! This example shows how to use the cross-platform abilities of dioxus to generate everything a dioxus app might need.
//! All of your apps will look like this.
//!
//! cargo run --features dioxus/static

fn main() {
    // DioxusApp::new(launch)
}

mod launches {
    #[cfg(feature = "wasm")]
    fn launch() {
        // launch the wasm_rednerer
    }

    #[cfg(feature = "static")]
    fn launch() {
        // render the tree to text
    }

    // #[cfg(features = "server")]
    // fn launch() {
    //     // launch the app
    // }

    // #[cfg(features = "liveview")]
    // fn launch() {
    //     // launch the app
    // }

    // #[cfg(features = "desktop")]
    // fn launch() {
    //     // launch the app
    // }

    // #[cfg(features = "android")]
    // fn launch() {
    //     // launch a simulator in dev mode
    // }

    // #[cfg(features = "ios")]
    // fn launch() {
    //     // launch a simulator in dev mode
    // }
}
