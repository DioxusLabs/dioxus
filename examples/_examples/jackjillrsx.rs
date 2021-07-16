use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example))
}

pub static Example: FC<()> = |cx| {
    let (name, set_name) = use_state_classic(cx, || "...?");
    cx.render(rsx! {
        section { class: "py-12 px-4 text-center"
            div { class: "w-full max-w-2xl mx-auto"
                span { class: "text-sm font-semibold"
                    "Dioxus Example: Jack and Jill"
                }
                h2 { class: "text-5xl mt-2 mb-6 leading-tight font-semibold font-heading"
                    "Hello, {name}"
                }
                div {
                    button {
                        class:"inline-block py-4 px-8 m-2 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
                        onclick: move |_| set_name("jack")
                        "Jack!"
                    }
                    button {
                        class:"inline-block py-4 px-8 m-2 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
                        onclick: move |_| set_name("jill")
                        "Jill!"
                    }
                }
            }
        }
    })
};
