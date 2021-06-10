//! Example: RecoilCallback
//! ------------------------
//! This example shows how use_recoil_callback can be used to abstract over sets/gets.
//! This hook provides a way to capture the RecoilApi object. In this case, we capture
//! it in a closure an abstract the set/get functionality behind the update_title function.
//!
//! It should be straightforward to build a complex app with recoil_callback.
use dioxus_core::prelude::*;
use recoil::*;

const TITLE: Atom<&str> = |_| "red";

fn update_title(api: &RecoilApi) {
    match *api.get(&TITLE) {
        "green" => api.set(&TITLE, "yellow"),
        "yellow" => api.set(&TITLE, "red"),
        "red" => api.set(&TITLE, "green"),
        _ => {}
    }
}

static App: FC<()> = |ctx| {
    let title = use_read(&ctx, &TITLE);
    let next_light = use_recoil_api(ctx, |api| move |_| update_title(&api));

    rsx! { in ctx,
        div {
            "{title}"
            button { onclick: {next_light}, "Next light" }
        }
    }
};

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}
