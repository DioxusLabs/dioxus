use dioxus::prelude::bumpalo;
use dioxus::prelude::format_args_f;
use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core::prelude::html;
use dioxus_web::WebsysRenderer;

fn main() {
    pretty_env_logger::init();
    log::info!("Hello!");

    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example))
}

static Example: FC<()> = |ctx, props| {
    let (name, set_name) = use_state(&ctx, || "...?");

    ctx.view(html! {
        <div>
            <h1> "Hello, {name}" </h1>
            <button onclick={move |_| set_name("jack")}> "jack!" </button>
            <button onclick={move |_| set_name("jill")}> "jill!" </button>
        </div>
    })
};
