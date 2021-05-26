//! Example: TODOVMC - One file
//! ---------------------------
//! This example shows how to build a one-file TODO MVC app with Dioxus and Recoil.
//! This project is confined to a single file to showcase the suggested patterns
//! for building a small but mighty UI with Dioxus without worrying about project structure.
//!
//! If you want an example on recommended project structure, check out the TodoMVC folder
//!
//! Here, we show to use Dioxus' Recoil state management solution to simplify app logic
#![allow(non_snake_case)]
use dioxus_web::dioxus::prelude::*;
use recoil::*;
use std::collections::HashMap;
use uuid::Uuid;

const APP_STYLE: &'static str = include_str!("./todomvc/style.css");

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App));
}

// Declare our global app state
const TODO_LIST: Atom<HashMap<Uuid, TodoItem>> = atom(|_| Default::default());
const FILTER: Atom<FilterState> = atom(|_| FilterState::All);
const TODOS_LEFT: selector<usize> = selector(|api| api.get(&TODO_LIST).len());

// Implement a simple abstraction over sets/gets of multiple atoms
struct TodoManager(RecoilApi);
impl TodoManager {
    fn add_todo(&self, contents: String) {
        let item = TodoItem {
            checked: false,
            contents,
            id: Uuid::new_v4(),
        };
        self.0.modify(&TODO_LIST, move |list| {
            list.insert(item.id, item);
        });
    }
    fn remove_todo(&self, id: &Uuid) {
        self.0.modify(&TODO_LIST, move |list| {
            list.remove(id);
        })
    }
    fn select_all_todos(&self) {
        self.0.modify(&TODO_LIST, move |list| {
            for item in list.values_mut() {
                item.checked = true;
            }
        })
    }
    fn toggle_todo(&self, id: &Uuid) {
        self.0.modify(&TODO_LIST, move |list| {
            list.get_mut(id).map(|item| item.checked = !item.checked)
        });
    }
    fn clear_completed(&self) {
        self.0.modify(&TODO_LIST, move |list| {
            *list = list.drain().filter(|(_, item)| !item.checked).collect();
        })
    }
    fn set_filter(&self, filter: &FilterState) {
        self.0.modify(&FILTER, move |f| *f = *filter);
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum FilterState {
    All,
    Active,
    Completed,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TodoItem {
    pub id: Uuid,
    pub checked: bool,
    pub contents: String,
}

fn App(ctx: Context, props: &()) -> DomTree {
    use_init_recoil_root(ctx);

    rsx! { in ctx,
        div { id: "app", style { "{APP_STYLE}" }
            TodoList {}
            Footer {}
        }
    }
}

pub fn TodoList(ctx: Context, props: &()) -> DomTree {
    let draft = use_state_new(&ctx, || "".to_string());
    let todos = use_recoil_value(&ctx, &TODO_LIST);
    let filter = use_recoil_value(&ctx, &FILTER);

    let todolist = todos
        .values()
        .filter(|item| match filter {
            FilterState::All => true,
            FilterState::Active => !item.checked,
            FilterState::Completed => item.checked,
        })
        .map(|item| {
            rsx!(TodoEntry {
                key: "{order}",
                id: item.id,
            })
        });

    rsx! { in ctx,
        div {
            header {
                class: "header"
                h1 {"todos"}
                input {
                    class: "new-todo"
                    placeholder: "What needs to be done?"
                    value: "{draft}"
                    oninput: move |evt| draft.set(evt.value)
                }
            }
            {todolist}

            // rsx! accepts optionals, so we suggest the `then` method in place of ternary
            {(!todos.is_empty()).then(|| rsx!( FilterToggles {}) )}
        }
    }
}

#[derive(PartialEq, Props)]
pub struct TodoEntryProps {
    id: Uuid,
}

pub fn TodoEntry(ctx: Context, props: &TodoEntryProps) -> DomTree {
    let (is_editing, set_is_editing) = use_state(&ctx, || false);
    let todo = use_recoil_value(&ctx, &TODO_LIST).get(&props.id).unwrap();

    ctx.render(rsx! (
        li {
            "{todo.id}"
            input {
                class: "toggle"
                type: "checkbox"
                "{todo.checked}"
            }
           {is_editing.then(|| rsx!(
                input {
                    value: "{todo.contents}"
                }
           ))}
        }
    ))
}

pub fn FilterToggles(ctx: Context, props: &()) -> DomTree {
    let reducer = use_recoil_context::<TodoManager>(ctx);
    let items_left = use_selector(ctx, &TODOS_LEFT);

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
                    href: "{path}",
                    onclick: move |_| reducer.set_filter(&filter),
                    "{name}"
                }
            }
        )
    });

    let item_text = match items_left {
        1 => "item",
        _ => "items",
    };

    rsx! { in ctx,
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
    }
}

pub fn Footer(ctx: Context, props: &()) -> DomTree {
    rsx! { in ctx,
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
}
