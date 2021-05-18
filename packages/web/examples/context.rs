
use std::fmt::Display;

use dioxus::{events::on::MouseEvent, prelude::*};
use dioxus_core as dioxus;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        WebsysRenderer::new_with_props(Example, ())
            .run()
            .await
            .unwrap()
    });
}


#[derive(Debug)]
struct CustomContext([&'static str; 3]);


static Example: FC<()> = |ctx, props| {
    ctx.create_context(|| CustomContext(["Jack", "Jill", "Bob"]));

        let names = ctx.use_context::<CustomContext>();
    // let name = names.0[props.id as usize];

    ctx.render(rsx! {
        div {
            class: "py-12 px-4 text-center w-full max-w-2xl mx-auto"
            span {
                class: "text-sm font-semibold"
                "Dioxus Example: Jack and Jill"
            }
            h2 {
                class: "text-5xl mt-2 mb-6 leading-tight font-semibold font-heading"
                "Hello"
            }

            CustomButton { id: 0 }
            CustomButton { id: 1 }
            CustomButton { id: 2 }
        }
    })
};


#[derive(Props, PartialEq)]
struct ButtonProps {
    id: u8,
}

fn CustomButton<'b, 'a,>(ctx: Context<'a>, props: &'b ButtonProps) -> DomTree {
    let names = ctx.use_context::<CustomContext>();
    let name = names.0[props.id as usize];

    ctx.render(rsx!{
        button {  
            class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
            "{name}"
        }
    })
}
