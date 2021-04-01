use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;

mod filtertoggles;
mod recoil;
mod state;
mod todoitem;
mod todolist;

use todolist::TodoList;

static APP_STYLE: &'static str = include_str!("./style.css");

fn main() {
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(|ctx, _| {
        ctx.render(rsx! {
            div {
                id: "app"
                style { "{APP_STYLE}" }

                // list
                TodoList {}

                // footer
                footer {
                    class: "info"
                    p {"Double-click to edit a todo"}
                    p {
                        "Created by "
                        a { "jkelleyrtp", href: "http://github.com/jkelleyrtp/" }
                    }
                    p {
                        "Part of "
                        a { "TodoMVC", href: "http://todomvc.com" }
                    }
                }
            }
        })
    }))
}
