#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_elements::input_data::keyboard_types::Key;

fn main() {
    dioxus_desktop::launch(app);
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum FilterState {
    All,
    Active,
    Completed,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TodoItem {
    pub id: u32,
    pub checked: bool,
    pub contents: String,
}

pub fn app(cx: Scope<()>) -> Element {
    let todos = use_state(cx, im_rc::HashMap::<u32, TodoItem>::default);
    let filter = use_state(cx, || FilterState::All);

    // Filter the todos based on the filter state
    let mut filtered_todos = todos
        .iter()
        .filter(|(_, item)| match **filter {
            FilterState::All => true,
            FilterState::Active => !item.checked,
            FilterState::Completed => item.checked,
        })
        .map(|f| *f.0)
        .collect::<Vec<_>>();
    filtered_todos.sort_unstable();

    let active_todo_count = todos.values().filter(|item| !item.checked).count();
    let active_todo_text = match active_todo_count {
        1 => "item",
        _ => "items",
    };

    let show_clear_completed = todos.values().any(|todo| todo.checked);

    cx.render(rsx! {
        section { class: "todoapp",
            style { include_str!("./assets/todomvc.css") }
            TodoHeader {
                todos: todos,
            }
            section {
                class: "main",
                if !todos.is_empty() {
                    rsx! {
                        input {
                            id: "toggle-all",
                            class: "toggle-all",
                            r#type: "checkbox",
                            onchange: move |_| {
                                let check = active_todo_count != 0;
                                for (_, item) in todos.make_mut().iter_mut() {
                                    item.checked = check;
                                }
                            },
                            checked: if active_todo_count == 0 { "true" } else { "false" },
                        }
                        label { r#for: "toggle-all" }
                    }
                }
                ul { class: "todo-list",
                    filtered_todos.iter().map(|id| rsx!(TodoEntry {
                        key: "{id}",
                        id: *id,
                        todos: todos,
                    }))
                }
                (!todos.is_empty()).then(|| rsx!(
                    ListFooter {
                        active_todo_count: active_todo_count,
                        active_todo_text: active_todo_text,
                        show_clear_completed: show_clear_completed,
                        todos: todos,
                        filter: filter,
                    }
                ))
            }
        }
        PageFooter {}
    })
}

#[derive(Props)]
pub struct TodoHeaderProps<'a> {
    todos: &'a UseState<im_rc::HashMap<u32, TodoItem>>,
}

pub fn TodoHeader<'a>(cx: Scope<'a, TodoHeaderProps<'a>>) -> Element {
    let draft = use_state(cx, || "".to_string());
    let todo_id = use_state(cx, || 0);

    cx.render(rsx! {
        header { class: "header",
        h1 {"todos"}
        input {
            class: "new-todo",
            placeholder: "What needs to be done?",
            value: "{draft}",
            autofocus: "true",
            oninput: move |evt| {
                draft.set(evt.value.clone());
            },
            onkeydown: move |evt| {
                if evt.key() == Key::Enter && !draft.is_empty() {
                    cx.props.todos.make_mut().insert(
                        **todo_id,
                        TodoItem {
                            id: **todo_id,
                            checked: false,
                            contents: draft.to_string(),
                        },
                    );
                    *todo_id.make_mut() += 1;
                    draft.set("".to_string());
                }
            }
        }
    }
    })
}

#[derive(Props)]
pub struct TodoEntryProps<'a> {
    todos: &'a UseState<im_rc::HashMap<u32, TodoItem>>,
    id: u32,
}

pub fn TodoEntry<'a>(cx: Scope<'a, TodoEntryProps<'a>>) -> Element {
    let is_editing = use_state(cx, || false);

    let todos = cx.props.todos.get();
    let todo = &todos[&cx.props.id];
    let completed = if todo.checked { "completed" } else { "" };
    let editing = if **is_editing { "editing" } else { "" };

    cx.render(rsx!{
        li {
            class: "{completed} {editing}",
            div { class: "view",
                input {
                    class: "toggle",
                    r#type: "checkbox",
                    id: "cbg-{todo.id}",
                    checked: "{todo.checked}",
                    oninput: move |evt| {
                        cx.props.todos.make_mut()[&cx.props.id].checked = evt.value.parse().unwrap();
                    }
                }
                label {
                    r#for: "cbg-{todo.id}",
                    ondblclick: move |_| is_editing.set(true),
                    prevent_default: "onclick",
                    "{todo.contents}"
                }
                button {
                    class: "destroy",
                    onclick: move |_| { cx.props.todos.make_mut().remove(&todo.id); },
                    prevent_default: "onclick",
                }
            }
            is_editing.then(|| rsx!{
                input {
                    class: "edit",
                    value: "{todo.contents}",
                    oninput: move |evt| cx.props.todos.make_mut()[&cx.props.id].contents = evt.value.clone(),
                    autofocus: "true",
                    onfocusout: move |_| is_editing.set(false),
                    onkeydown: move |evt| {
                        match evt.key() {
                            Key::Enter | Key::Escape | Key::Tab => is_editing.set(false),
                            _ => {}
                        }
                    },
                }
            })
        }
    })
}

#[derive(Props)]
pub struct ListFooterProps<'a> {
    todos: &'a UseState<im_rc::HashMap<u32, TodoItem>>,
    active_todo_count: usize,
    active_todo_text: &'a str,
    show_clear_completed: bool,
    filter: &'a UseState<FilterState>,
}

pub fn ListFooter<'a>(cx: Scope<'a, ListFooterProps<'a>>) -> Element {
    let active_todo_count = cx.props.active_todo_count;
    let active_todo_text = cx.props.active_todo_text;

    let selected = |state| {
        if *cx.props.filter == state {
            "selected"
        } else {
            "false"
        }
    };

    cx.render(rsx! {
        footer { class: "footer",
            span { class: "todo-count",
                strong {"{active_todo_count} "}
                span {"{active_todo_text} left"}
            }
            ul { class: "filters",
                for (state, state_text, url) in [
                    (FilterState::All, "All", "#/"),
                    (FilterState::Active, "Active", "#/active"),
                    (FilterState::Completed, "Completed", "#/completed"),
                ] {
                    li {
                        a {
                            href: url,
                            class: selected(state),
                            onclick: move |_| cx.props.filter.set(state),
                            prevent_default: "onclick",
                            state_text
                        }
                    }
                }
            }
            if cx.props.show_clear_completed {
                cx.render(rsx! {
                    button {
                        class: "clear-completed",
                        onclick: move |_| cx.props.todos.make_mut().retain(|_, todo| !todo.checked),
                        "Clear completed"
                    }
                })
            }
        }
    })
}

pub fn PageFooter(cx: Scope) -> Element {
    cx.render(rsx! {
        footer { class: "info",
            p { "Double-click to edit a todo" }
            p { "Created by ", a { href: "http://github.com/jkelleyrtp/", "jkelleyrtp" }}
            p { "Part of ", a { href: "http://todomvc.com", "TodoMVC" }}
        }
    })
}
