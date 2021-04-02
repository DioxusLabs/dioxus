#![allow(non_snake_case)]
use dioxus_core as dioxus;
use dioxus::{events::on::MouseEvent, prelude::*};
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    
    wasm_bindgen_futures::spawn_local(async {
        let props = ExampleProps { initial_name: "..?"};
        WebsysRenderer::new_with_props(Example, props)
            .run()
            .await
            .unwrap()
    });
}

#[derive(PartialEq, Props)]
struct ExampleProps {
    initial_name: &'static str,
}

static Example: FC<ExampleProps> = |ctx, props| {
    let name = use_state_new(&ctx, move || props.initial_name.to_string());

    ctx.render(rsx! {
        div { 
            class: "py-12 px-4 text-center w-full max-w-2xl mx-auto"
            span { 
                class: "text-sm font-semibold"
                "Dioxus Example: Jack and Jill"
            }
            h2 { 
                class: "text-5xl mt-2 mb-6 leading-tight font-semibold font-heading"   
                "Hello, {name}"
            }
            
            CustomButton { name: "Jack!", handler: move |_| name.set("Jack".to_string()) }
            CustomButton { name: "Jill!", handler: move |_| name.set("Jill".to_string()) }
            CustomButton { name: "Bob!", handler: move |_| name.set("Bob".to_string())}
        }
    })
};



#[derive(Props)]
struct ButtonProps<'src, F: Fn(MouseEvent)> {
    name: &'src str,
    handler: F
}

fn CustomButton<'b, 'a, F: Fn(MouseEvent)>(ctx: Context<'a>, props: &'b ButtonProps<'b, F>) -> DomTree {
    ctx.render(rsx!{
        button {  
            class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
            onmouseover: {&props.handler}
            "{props.name}"
        }
    })
}

impl<F: Fn(MouseEvent)> PartialEq for ButtonProps<'_, F> {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
