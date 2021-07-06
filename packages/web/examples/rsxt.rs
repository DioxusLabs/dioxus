#![allow(non_snake_case)]
use std::rc::Rc;

use dioxus::{events::on::MouseEvent, prelude::*};
use dioxus_core as dioxus;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        let props = ExampleProps {
            initial_name: "..?",
        };
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

static Example: FC<ExampleProps> = |cx| {
    let name = use_state(&cx, move || cx.initial_name);

    cx.render(rsx! {
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

            CustomButton { name: "Jack!", handler: move |_| name.set("Jack") }
            CustomButton { name: "Jill!", handler: move |_| name.set("Jill") }
            CustomButton { name: "Bob!", handler: move |_| name.set("Bob")}
            Placeholder {val: name}
            Placeholder {val: name}
        }
    })
};

#[derive(Props)]
struct ButtonProps<'src, F: Fn(Rc<dyn MouseEvent>)> {
    name: &'src str,
    handler: F,
}

fn CustomButton<'a, F: Fn(MouseEvent)>(cx: Context<'a, ButtonProps<'a, F>>) -> VNode {
    cx.render(rsx!{
        button {  
            class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
            onmouseover: {&cx.handler}
            "{cx.name}"
        }
    })
}

impl<F: Fn(MouseEvent)> PartialEq for ButtonProps<'_, F> {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}

#[derive(Props, PartialEq)]
struct PlaceholderProps {
    val: &'static str,
}
fn Placeholder(cx: Context<PlaceholderProps>) -> VNode {
    cx.render(rsx! {
        div {
            "child: {cx.val}"
        }
    })
}
