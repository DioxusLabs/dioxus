use dioxus::{events::on::MouseEvent, prelude::*};
use dioxus_core as dioxus;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

fn App(ctx: Context<()>) -> VNode {
    let cansee = use_state_new(&ctx, || false);
    rsx! { in ctx,
        div {
            "Shadow of the child:"
            button {
                "Gaze into the void"
                onclick: move |_| cansee.set(!**cansee)
            }
            {cansee.then(|| rsx!{ Child {} })}
        }
    }
}

fn Child(ctx: Context<()>) -> VNode {
    rsx! { in ctx,
        section { class: "py-6 bg-coolGray-100 text-coolGray-900"
            div { class: "container mx-auto flex flex-col items-center justify-center p-4 space-y-8 md:p-10 md:px-24 xl:px-48"
                h1 { class: "text-5xl font-bold leading-none text-center",
                    "Sign up now"
                }
                p { class: "text-xl font-medium text-center",
                    "At a assumenda quas cum earum ut itaque commodi saepe rem aspernatur quam natus quis nihil quod, hic explicabo doloribus magnam neque, exercitationem eius sunt!"
                }
                div { class: "flex flex-col space-y-4 sm:space-y-0 sm:flex-row sm:space-x-8"
                    button { class: "px-8 py-3 text-lg font-semibold rounded bg-violet-600 text-coolGray-50",
                        "Get started"
                    }
                    button { class: "px-8 py-3 text-lg font-normal border rounded bg-coolGray-800 text-coolGray-50 border-coolGray-700",
                        "Learn more"
                    }
                }
            }
        }
    }
}
