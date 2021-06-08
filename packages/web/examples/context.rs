//! Example: Context API
//! --------------------
//! This example demonstrates how to use the raw context api for sharing state throughout the VirtualDOM Tree.
//! A custom context must be its own unique type - otherwise use_context will fail. A context may be c
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 

use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
}


#[derive(Debug)]
struct CustomContext([&'static str; 3]);

static Example: FC<()> = |ctx| {
    ctx.use_create_context(|| CustomContext(["Jack", "Jill", "Bob"]));

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

fn CustomButton(ctx: Context<ButtonProps>) -> VNode {
    let names = ctx.use_context::<CustomContext>();
    let name = names.0[ctx.id as usize];

    ctx.render(rsx!{
        button {  
            class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
            "{name}"
        }
    })
}
