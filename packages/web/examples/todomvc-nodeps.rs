use std::{collections::HashMap, rc::Rc};

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;

static APP_STYLE: &'static str = include_str!("./todomvc/style.css");

fn main() {
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

#[derive(PartialEq)]
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

pub fn App(cx: Context<()>) -> VNode {
    let (draft, set_draft) = use_state_classic(&cx, || "".to_string());
    let (todos, set_todos) = use_state_classic(&cx, || HashMap::<uuid::Uuid, Rc<TodoItem>>::new());
    let (filter, set_filter) = use_state_classic(&cx, || FilterState::All);

    let filtered_todos = todos.iter().filter(move |(id, item)| match filter {
        FilterState::All => true,
        FilterState::Active => !item.checked,
        FilterState::Completed => item.checked,
    });
    let items_left = filtered_todos.clone().count();
    let item_text = match items_left {
        1 => "item",
        _ => "items",
    };

    cx.render(rsx! {
        div { id: "app"
            div {
                header { class: "header"
                    h1 {"todos"}
                    input {
                        class: "new-todo"
                        placeholder: "What needs to be done?"
                        value: "{draft}"
                        oninput: move |evt| set_draft(evt.value())
                    }
                }

                {filtered_todos.map(|(id, item)| {
                    rsx!(TodoEntry {
                        key: "{id}",
                        item: item.clone()
                    })
                })}

                // filter toggle (show only if the list isn't empty)
                {(!todos.is_empty()).then(|| rsx!(
                    footer {
                        span {
                            strong {"{items_left}"}
                            span {"{item_text} left"}
                        }
                        ul {
                            class: "filters"
                            li { class: "All", a { href: "", onclick: move |_| set_filter(FilterState::All), "All" }}
                            li { class: "Active", a { href: "active", onclick: move |_| set_filter(FilterState::Active), "Active" }}
                            li { class: "Completed", a { href: "completed", onclick: move |_| set_filter(FilterState::Completed), "Completed" }}
                        }
                    }
                ))}
            }
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
}

#[derive(PartialEq, Props)]
pub struct TodoEntryProps {
    item: Rc<TodoItem>,
}

pub fn TodoEntry(cx: Context<TodoEntryProps>) -> VNode {
    let (is_editing, set_is_editing) = use_state_classic(&cx, || false);
    let contents = "";
    let todo = TodoItem {
        checked: false,
        contents: "asd".to_string(),
        id: uuid::Uuid::new_v4(),
    };

    cx.render(rsx! (
        li {
            "{todo.id}"
            input {
                class: "toggle"
                type: "checkbox"
                "{todo.checked}"
            }
           {is_editing.then(|| rsx!{
                input {
                    value: "{contents}"
                }
            })}
        }
    ))
}
