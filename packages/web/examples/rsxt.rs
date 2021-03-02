use bumpalo::Bump;
use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example))
}

static Example: FC<()> = |ctx, props| {
    let (name, set_name) = use_state(&ctx, || "...?");

    ctx.render(rsx! {
        div { class: "py-12 px-4 text-center w-full max-w-2xl mx-auto"
            span { "Dioxus Example: Jack and Jill",
                class: "text-sm font-semibold"
            }
            h2 { "Hello, {name}", 
                class: "text-5xl mt-2 mb-6 leading-tight font-semibold font-heading"   
            }
            div {
                button { "Jack!"
                    class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
                    onclick: {move |_| set_name("jack")}
                }
                
                button { "Jill!"
                    class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
                    onclick: {move |_| set_name("jill")}
                    onclick: {move |_| set_name("jill")}
                }
            }
        }
    })
};
