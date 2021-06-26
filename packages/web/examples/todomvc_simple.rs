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

// =======================
// Components
// =======================
pub fn App(cx: Context<()>) -> VNode {
    cx.render(rsx! {
        div {
            id: "app"

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
}

pub fn TodoList(cx: Context<()>) -> VNode {
    let (draft, set_draft) = use_state(&cx, || "".to_string());
    let (todos, set_todos) = use_state(&cx, || HashMap::<uuid::Uuid, Rc<TodoItem>>::new());
    let (filter, set_filter) = use_state(&cx, || FilterState::All);

    cx.render(rsx! {
        div {
            header {
                class: "header"
                h1 {"todos"}
                input {
                    class: "new-todo"
                    placeholder: "What needs to be done?"
                    value: "{draft}"
                    oninput: move |evt| set_draft(evt.value())
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
                .map(|(id, item)| {
                    // TodoEntry!();
                    todo!()
                    // rsx!(TodoEntry {
                    //     key: "{order}",
                    //     item: item.clone()
                    // })
                })
            }

            // filter toggle (show only if the list isn't empty)
            {(!todos.is_empty()).then(||
                rsx!( FilterToggles {})
            )}
        }
    })
}

#[derive(PartialEq, Props)]
pub struct TodoEntryProps {
    item: Rc<TodoItem>,
}

pub fn TodoEntry(cx: Context<TodoEntryProps>) -> VNode {
    let (is_editing, set_is_editing) = use_state(&cx, || false);
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

pub fn FilterToggles(cx: Context<()>) -> VNode {
    let toggles = [
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
                    // onclick: move |_| reducer.set_filter(&filter)
                    "{name}"
                }
            }
        )
    });

    // todo
    let item_text = "";
    let items_left = "";

    cx.render(rsx! {
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
