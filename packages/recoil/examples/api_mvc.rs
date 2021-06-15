//! RecoilAPI Pattern
//! ----------------
//! This example demonstrates how the use_recoil_callback hook can be used to build view controllers.
//! These view controllers are cheap to make and can be easily shared across the app to provide global
//! state logic to many components.
//!
//! This pattern is meant to replace the typical use_dispatch pattern used in Redux apps.

use dioxus_core::prelude::*;
use recoil::*;

const TITLE: Atom<String> = |_| format!("Heading");
const SUBTITLE: Atom<String> = |_| format!("Subheading");

struct TitleController(RecoilApi);
impl TitleController {
    fn new(api: RecoilApi) -> Self {
        Self(api)
    }
    fn uppercase(&self) {
        self.0.modify(&TITLE, |f| *f = f.to_uppercase());
        self.0.modify(&SUBTITLE, |f| *f = f.to_uppercase());
    }
    fn lowercase(&self) {
        self.0.modify(&TITLE, |f| *f = f.to_lowercase());
        self.0.modify(&SUBTITLE, |f| *f = f.to_lowercase());
    }
}

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(|ctx| {
        let title = use_read(&ctx, &TITLE);
        let subtitle = use_read(&ctx, &SUBTITLE);
        let controller = TitleController::new(use_recoil_api(&ctx));

        rsx! { in ctx,
            div {
                "{title}"
                "{subtitle}"
                button { onclick: move |_| controller.uppercase(), "Uppercase" }
                button { onclick: move |_| controller.lowercase(), "Lowercase" }
            }
        }
    }))
}
