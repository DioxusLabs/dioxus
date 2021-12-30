#![allow(non_upper_case_globals, non_snake_case)]

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;

use std::collections::HashMap;

fn main() {
    dioxus_desktop::launch(App)
}

#[derive(PartialEq)]
pub enum FilterState {
    All,
    Active,
    Completed,
}

#[derive(Debug, PartialEq)]
pub struct TodoItem {
    pub id: u32,
    pub checked: bool,
    pub contents: String,
}
pub type Todos = HashMap<u32, TodoItem>;

pub static App: Component = |cx| {
    // Share our TodoList to the todos themselves
    use_provide_state(cx, Todos::new);

    // Save state for the draft, filter
    let draft = use_state(&cx, || "".to_string());
    let filter = use_state(&cx, || FilterState::All);
    let mut todo_id = use_state(&cx, || 0);

    // Consume the todos
    let todos = use_shared_state::<Todos>(cx)?;

    // Filter the todos based on the filter state
    let mut filtered_todos = todos
        .read()
        .iter()
        .filter(|(_, item)| match *filter {
            FilterState::All => true,
            FilterState::Active => !item.checked,
            FilterState::Completed => item.checked,
        })
        .map(|f| *f.0)
        .collect::<Vec<_>>();
    filtered_todos.sort_unstable();

    // Define the actions to manage the todolist
    let mut submit_todo = move || {
        if !draft.is_empty() {
            todos.write().insert(
                *todo_id,
                TodoItem {
                    id: *todo_id,
                    checked: false,
                    contents: draft.get().clone(),
                },
            );
            todo_id += 1;
            draft.set("".to_string());
        }
    };
    let clear_completed = move || {
        todos.write().retain(|_, todo| todo.checked == false);
    };

    // Some assists in actually rendering the content
    let show_clear_completed = todos.read().values().any(|todo| todo.checked);
    let items_left = filtered_todos.len();
    let item_text = match items_left {
        1 => "item",
        _ => "items",
    };

    cx.render(rsx!{
        section { class: "todoapp"
            style { {[include_str!("./todomvc.css")]} }
            div {
                header { class: "header"
                    h1 {"todos"}
                    input {
                        class: "new-todo"
                        placeholder: "What needs to be done?"
                        value: "{draft}"
                        autofocus: "true"
                        oninput: move |evt| draft.set(evt.value)
                        onkeydown: move |evt| {
                            if evt.key == "Enter" {
                                submit_todo();
                            }
                        }
                    }
                }
                ul { class: "todo-list",
                    filtered_todos.iter().map(|id| rsx!(TodoEntry { key: "{id}", id: *id }))
                }
                (!todos.read().is_empty()).then(|| rsx!(
                    footer { class: "footer",
                        span { class: "todo-count" strong {"{items_left} "} span {"{item_text} left"} }
                        ul { class: "filters"
                            li { class: "All", a { onclick: move |_| filter.set(FilterState::All), "All" }}
                            li { class: "Active", a { onclick: move |_| filter.set(FilterState::Active), "Active" }}
                            li { class: "Completed", a { onclick: move |_| filter.set(FilterState::Completed), "Completed" }}
                        }
                        (show_clear_completed).then(|| rsx!(
                            button { class: "clear-completed", onclick: move |_| clear_completed(),
                                "Clear completed"
                            }
                        ))
                    }
                ))
            }
        }
        footer { class: "info"
            p {"Double-click to edit a todo"}
            p { "Created by ", a { "jkelleyrtp", href: "http://github.com/jkelleyrtp/" }}
            p { "Part of ", a { "TodoMVC", href: "http://todomvc.com" }}
        }
    })
};

#[derive(PartialEq, Props)]
pub struct TodoEntryProps {
    id: u32,
}

pub fn TodoEntry((cx, props): Scope<TodoEntryProps>) -> Element {
    let todos = use_shared_state::<Todos>(cx)?;

    let _todos = todos.read();
    let todo = _todos.get(&cx.props.id)?;

    let is_editing = use_state(&cx, || false);
    let completed = if todo.checked { "completed" } else { "" };

    cx.render(rsx!{
        li { class: "{completed}"
            div { class: "view"
                input { class: "toggle" r#type: "checkbox" id: "cbg-{todo.id}" checked: "{todo.checked}"
                    onchange: move |evt| {
                        if let Some(todo) = todos.write().get_mut(&cx.props.id) { 
                            todo.checked = evt.value.parse().unwrap()
                        }
                    }
                }

                label { r#for: "cbg-{todo.id}" pointer_events: "none"
                    "{todo.contents}"
                }

               {is_editing.then(|| rsx!{
                    input { value: "{todo.contents}"
                        oninput: move |evt| {
                            if let Some(todo) = todos.write().get_mut(&cx.props.id) { 
                                todo.contents = evt.value
                            }
                        },
                    }
                })}
            }
        }
    })
}
