//! The typical TodoMVC app, implemented in Dioxus.

use dioxus::prelude::*;
use dioxus_stores::*;
use std::{collections::HashMap, vec};

const STYLE: Asset = asset!("/examples/assets/todomvc.css");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // We store the todos in a HashMap in a Signal.
    // Each key is the id of the todo, and the value is the todo itself.
    let todos = use_store(|| TodoState {
        todos: HashMap::new(),
        filter: FilterState::All,
    });

    let todo_list = todos.todos();
    let filter = todos.filter();

    // We use a simple memoized signal to calculate the number of active todos.
    // Whenever the todos change, the active_todo_count will be recalculated.
    let active_todo_count = use_memo(move || {
        todo_list
            .values()
            .filter(|item| !item.checked().cloned())
            .count()
    });

    // We use a memoized signal to filter the todos based on the current filter state.
    // Whenever the todos or filter change, the filtered_todos will be recalculated.
    // Note that we're only storing the IDs of the todos, not the todos themselves.
    let filtered_todos = use_memo(move || {
        let mut filtered_todos = todo_list
            .iter()
            .filter(|(_, item)| match filter() {
                FilterState::All => true,
                FilterState::Active => !item.checked().cloned(),
                FilterState::Completed => item.checked().cloned(),
            })
            .map(|(i, _)| i as u32)
            .collect::<Vec<_>>();

        filtered_todos.sort_unstable();

        filtered_todos
    });

    // Toggle all the todos to the opposite of the current state.
    // If all todos are checked, uncheck them all. If any are unchecked, check them all.
    let toggle_all = move |_| {
        let check = active_todo_count() != 0;
        for item in todo_list.values() {
            item.checked().set(check);
        }
    };

    rsx! {
        document::Link { rel: "stylesheet", href: STYLE }
        section { class: "todoapp",
            TodoHeader { todos }
            section { class: "main",
                if !todo_list.is_empty() {
                    input {
                        id: "toggle-all",
                        class: "toggle-all",
                        r#type: "checkbox",
                        onchange: toggle_all,
                        checked: active_todo_count() == 0
                    }
                    label { r#for: "toggle-all" }
                }

                // Render the todos using the filtered_todos signal
                // We pass the ID into the TodoEntry component so it can access the todo from the todos signal.
                // Since we store the todos in a signal too, we also need to send down the todo list
                ul { class: "todo-list",
                    for id in filtered_todos() {
                        TodoEntry { key: "{id}", id, todos }
                    }
                }

                // We only show the footer if there are todos.
                if !todo_list.is_empty() {
                    ListFooter { active_todo_count, todos }
                }
            }
        }

        // A simple info footer
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

#[component]
fn TodoHeader(mut todos: Store<TodoState>) -> Element {
    let mut draft = use_signal(|| "".to_string());
    let mut todo_id = use_signal(|| 0);

    let onkeydown = move |evt: KeyboardEvent| {
        if evt.key() == Key::Enter && !draft.read().is_empty() {
            let id = todo_id();
            let todo = TodoItem::new(draft.take());
            todos.todos().insert(id, todo);
            todo_id += 1;
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
                oninput: move |evt| draft.set(evt.value()),
                onkeydown
            }
        }
    }
}

/// A single todo entry
/// This takes the ID of the todo and the todos signal as props
/// We can use these together to memoize the todo contents and checked state
#[component]
fn TodoEntry(mut todos: Store<TodoState>, id: u32) -> Element {
    let mut is_editing = use_signal(|| false);

    let entry = todos.todos().get(id).unwrap();
    let checked = entry.checked();
    let contents = entry.contents();

    rsx! {
        li {
            // Dioxus lets you use if statements in rsx to conditionally render attributes
            // These will get merged into a single class attribute
            class: if checked() { "completed" },
            class: if is_editing() { "editing" },

            // Some basic controls for the todo
            div { class: "view",
                input {
                    class: "toggle",
                    r#type: "checkbox",
                    id: "cbg-{id}",
                    checked: "{checked}",
                    oninput: move |evt| entry.checked().set(evt.checked())
                }
                label {
                    r#for: "cbg-{id}",
                    ondoubleclick: move |_| is_editing.set(true),
                    onclick: |evt| evt.prevent_default(),
                    "{contents}"
                }
                button {
                    class: "destroy",
                    onclick: move |evt| {
                        evt.prevent_default();
                        todos.todos().remove(&id);
                    },
                }
            }

            // Only render the actual input if we're editing
            if is_editing() {
                input {
                    class: "edit",
                    value: "{contents}",
                    oninput: move |evt| entry.contents().set(evt.value()),
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
fn ListFooter(mut todos: Store<TodoState>, active_todo_count: ReadSignal<usize>) -> Element {
    // We use a memoized signal to calculate whether we should show the "Clear completed" button.
    // This will recompute whenever the todos change, and if the value is true, the button will be shown.
    let show_clear_completed =
        use_memo(move || todos.todos().values().any(|todo| todo.checked().cloned()));
    let mut filter = todos.filter();

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
                            onclick: move |evt| {
                                evt.prevent_default();
                                filter.set(state)
                            },
                            {state_text}
                        }
                    }
                }
            }
            if show_clear_completed() {
                button {
                    class: "clear-completed",
                    onclick: move |_| todos.todos().retain(|_, todo| !todo.checked),
                    "Clear completed"
                }
            }
        }
    }
}

#[derive(Store, PartialEq, Clone, Debug)]
struct TodoState {
    todos: HashMap<u32, TodoItem>,
    filter: FilterState,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum FilterState {
    All,
    Active,
    Completed,
}

#[derive(Store, PartialEq, Clone, Debug)]
struct TodoItem {
    checked: bool,
    contents: String,
}

impl TodoItem {
    fn new(contents: impl ToString) -> Self {
        Self {
            checked: false,
            contents: contents.to_string(),
        }
    }
}
