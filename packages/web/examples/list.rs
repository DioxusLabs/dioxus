use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;
use std::collections::BTreeMap;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

static APP_STYLE: &'static str = include_str!("./todomvc/style.css");

#[derive(PartialEq, Clone, Copy)]
pub enum FilterState {
    All,
    Active,
    Completed,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TodoItem {
    pub id: uuid::Uuid,
    pub checked: bool,
    pub contents: String,
}

static App: FC<()> = |ctx| {
    let (draft, set_draft) = use_state(&ctx, || "".to_string());
    let (filter, set_filter) = use_state(&ctx, || FilterState::All);
    let todos = use_state_new(&ctx, || BTreeMap::<uuid::Uuid, TodoItem>::new());
    ctx.render(rsx!(
        div {
            id: "app"
            div {
                header {
                    class: "header"
                    h1 {"todos"}
                    button {
                        "press me"
                        onclick: move |evt| {
                            let contents = draft.clone();
                            todos.modify(|f| {
                                let id = uuid::Uuid::new_v4();
                                f.insert(id.clone(), TodoItem {
                                    id,
                                    checked: false,
                                    contents
                                });
                            })
                        }
                    }
                    input {
                        class: "new-todo"
                        placeholder: "What needs to be done?"
                        oninput: move |evt| set_draft(evt.value)
                    }
                }

                { // list
                    todos
                    .iter()
                    .filter(|(id, item)| match filter {
                        FilterState::All => true,
                        FilterState::Active => !item.checked,
                        FilterState::Completed => item.checked,
                    })
                    .map(|(id, todo)| {
                        rsx!{
                            li {
                                key: "{id}"
                                "{todo.contents}"
                                input {
                                    class: "toggle"
                                    type: "checkbox"
                                    "{todo.checked}"
                                }
                            }
                        }
                    })
                }

                // filter toggle (show only if the list isn't empty)
                {(!todos.is_empty()).then(||
                    rsx!{
                        footer {
                            span {
                                strong {"10"}
                                span {"0 items left"}
                            }
                            ul {
                                class: "filters"
                            {[
                                    ("All", "", FilterState::All),
                                    ("Active", "active", FilterState::Active),
                                    ("Completed", "completed", FilterState::Completed),
                                ]
                                .iter()
                                .map(|(name, path, filter)| {
                                    rsx!(
                                        li {
                                            class: "{name}"
                                            a {
                                                href: "{path}"
                                                onclick: move |_| set_filter(filter.clone())
                                                "{name}"
                                            }
                                        }
                                    )
                                })
                            }}
                        }
                    }
                )}
            }


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
    ))
};

pub fn FilterToggles(ctx: Context<()>) -> VNode {
    // let reducer = recoil::use_callback(&ctx, || ());
    // let items_left = recoil::use_atom_family(&ctx, &TODOS, uuid::Uuid::new_v4());

    let toggles = [
        ("All", "", FilterState::All),
        ("Active", "active", FilterState::Active),
        ("Completed", "completed", FilterState::Completed),
    ]
    .iter()
    .map(|(name, path, _filter)| {
        rsx!(
            li {
                class: "{name}"
                a {
                    href: "{path}"
                    // onclick: move |_| reducer.set_filter(&filter)
                    "{name}"
                }
            }
        )
    });

    // todo
    let item_text = "";
    let items_left = "";

    ctx.render(rsx! {
        footer {
            span {
                strong {"{items_left}"}
                span {"{item_text} left"}
            }
            ul {
                class: "filters"
                {toggles}
            }
        }
    })
}
