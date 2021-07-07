use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;
use dioxus_web::{prelude::*, WebsysRenderer};

// mod filtertoggles;
// mod recoil;
// mod state;
// mod todoitem;
// mod todolist;

static APP_STYLE: &'static str = include_str!("./style.css");

fn main() {
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(|cx| {
        cx.render(rsx! {
            div {
                id: "app"
                // style { "{APP_STYLE}" }

                // list
                // todolist::TodoList {}

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
