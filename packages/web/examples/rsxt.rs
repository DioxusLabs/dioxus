#![allow(non_snake_case)]
use dioxus_core as dioxus;
use dioxus::prelude::*;
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
    let (name, set_name) = use_state(&ctx, move || props.initial_name.to_string());

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
            
            CustomButton { name: "Jack!", set_name: set_name }
            CustomButton { name: "Jill!", set_name: set_name }
            CustomButton { name: "Bob!", set_name: set_name }
        }
    })
};

#[derive(Props)]
struct ButtonProps<'src> {
    name: &'src str,
    set_name: &'src dyn Fn(String)
}

/// this is an awesome component
fn CustomButton<'a>(ctx: Context<'a>, props: &'a ButtonProps<'a>) -> DomTree {
    ctx.render(rsx!{
        button {  
            class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
            onmouseover: move |evt| (props.set_name)(props.name.to_string())
            "{props.name}"
        }
    })
}


impl PartialEq for ButtonProps<'_> {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
