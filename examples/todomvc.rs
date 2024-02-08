#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_elements::input_data::keyboard_types::Key;
use std::collections::HashMap;

fn main() {
    launch(app);
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum FilterState {
    All,
    Active,
    Completed,
}

#[derive(Debug, PartialEq, Eq)]
struct TodoItem {
    id: u32,
    checked: bool,
    contents: String,
}

const STYLE: &str = include_str!("./assets/todomvc.css");

fn app() -> Element {
    let mut todos = use_signal(HashMap::<u32, TodoItem>::new);
    let filter = use_signal(|| FilterState::All);

    let active_todo_count =
        use_memo(move || todos.read().values().filter(|item| !item.checked).count());

    let filtered_todos = use_memo(move || {
        let mut filtered_todos = todos
            .read()
            .iter()
            .filter(|(_, item)| match filter() {
                FilterState::All => true,
                FilterState::Active => !item.checked,
                FilterState::Completed => item.checked,
            })
            .map(|f| *f.0)
            .collect::<Vec<_>>();

        filtered_todos.sort_unstable();

        filtered_todos
    });

    let toggle_all = move |_| {
        let check = active_todo_count() != 0;
        for (_, item) in todos.write().iter_mut() {
            item.checked = check;
        }
    };

    rsx! {
        section { class: "todoapp",
            style { {STYLE} }
            TodoHeader { todos }
            section { class: "main",
                if !todos.read().is_empty() {
                    input {
                        id: "toggle-all",
                        class: "toggle-all",
                        r#type: "checkbox",
                        onchange: toggle_all,
                        checked: active_todo_count() == 0,
                    }
                    label { r#for: "toggle-all" }
                }
                ul { class: "todo-list",
                    for id in filtered_todos() {
                        TodoEntry { key: "{id}", id, todos }
                    }
                }
                if !todos.read().is_empty() {
                    ListFooter { active_todo_count, todos, filter }
                }
            }
        }
        PageFooter {}
    }
}

#[component]
fn TodoHeader(mut todos: Signal<HashMap<u32, TodoItem>>) -> Element {
    let mut draft = use_signal(|| "".to_string());
    let mut todo_id = use_signal(|| 0);

    let onkeydown = move |evt: KeyboardEvent| {
        if evt.key() == Key::Enter && !draft.read().is_empty() {
            let id = todo_id();
            let todo = TodoItem {
                id,
                checked: false,
                contents: draft.to_string(),
            };
            todos.write().insert(id, todo);
            todo_id += 1;
            draft.set("".to_string());
        }
    };

    rsx! {
        header { class: "header",
            h1 { "todos" }
            input {
                class: "new-todo",
                placeholder: "What needs to be done?",
                value: "{draft}",
                autofocus: "true",
                oninput: move |evt| draft.set(evt.value().clone()),
                onkeydown,
            }
        }
    }
}

#[component]
fn TodoEntry(mut todos: Signal<HashMap<u32, TodoItem>>, id: u32) -> Element {
    let mut is_editing = use_signal(|| false);
    let checked = use_memo(move || todos.read().get(&id).unwrap().checked);
    let contents = use_memo(move || todos.read().get(&id).unwrap().contents.clone());

    rsx! {
        li { class: if checked() { "completed" }, class: if is_editing() { "editing" },
            div { class: "view",
                input {
                    class: "toggle",
                    r#type: "checkbox",
                    id: "cbg-{id}",
                    checked: "{checked}",
                    oninput: move |evt| todos.write().get_mut(&id).unwrap().checked = evt.value().parse().unwrap(),
                }
                label {
                    r#for: "cbg-{id}",
                    ondoubleclick: move |_| is_editing.set(true),
                    prevent_default: "onclick",
                    "{contents}"
                }
                button {
                    class: "destroy",
                    onclick: move |_| { todos.write().remove(&id); },
                    prevent_default: "onclick"
                }
            }
            if is_editing() {
                input {
                    class: "edit",
                    value: "{contents}",
                    oninput: move |evt| todos.write().get_mut(&id).unwrap().contents = evt.value(),
                    autofocus: "true",
                    onfocusout: move |_| is_editing.set(false),
                    onkeydown: move |evt| {
                        match evt.key() {
                            Key::Enter | Key::Escape | Key::Tab => is_editing.set(false),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ListFooter(
    mut todos: Signal<HashMap<u32, TodoItem>>,
    active_todo_count: ReadOnlySignal<usize>,
    mut filter: Signal<FilterState>,
) -> Element {
    let show_clear_completed = use_memo(move || todos.read().values().any(|todo| todo.checked));

    rsx! {
        footer { class: "footer",
            span { class: "todo-count",
                strong { "{active_todo_count} " }
                span {
                    match active_todo_count() {
                        1 => "item",
                        _ => "items",
                    }
                    " left"
                }
            }
            ul { class: "filters",
                for (state , state_text , url) in [
                    (FilterState::All, "All", "#/"),
                    (FilterState::Active, "Active", "#/active"),
                    (FilterState::Completed, "Completed", "#/completed"),
                ] {
                    li {
                        a {
                            href: url,
                            class: if filter() == state { "selected" },
                            onclick: move |_| filter.set(state),
                            prevent_default: "onclick",
                            {state_text}
                        }
                    }
                }
            }
            if show_clear_completed() {
                button {
                    class: "clear-completed",
                    onclick: move |_| todos.write().retain(|_, todo| !todo.checked),
                    "Clear completed"
                }
            }
        }
    }
}

fn PageFooter() -> Element {
    rsx! {
        footer { class: "info",
            p { "Double-click to edit a todo" }
            p {
                "Created by "
                a { href: "http://github.com/jkelleyrtp/", "jkelleyrtp" }
            }
            p {
                "Part of "
                a { href: "http://todomvc.com", "TodoMVC" }
            }
        }
    }
}
