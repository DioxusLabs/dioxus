//! A nested todo app driven by a single `Signal` root through `dioxus-optics`.
//!
//! The whole application state lives in one `Signal<AppState>`. Every widget
//! projects into some part of that state through the same `map_ref_mut` /
//! `map_some` / `each` / `each_hash_map` helpers, without creating per-field
//! signals. Mutating a deeply-nested optic path re-renders the rest of the UI
//! because every read registers a subscription on the root signal.
//!
//! Run with: `cargo run --example nested_todo -p dioxus-optics`.

#![allow(non_snake_case)]

use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_optics::prelude::*;

fn main() {
    dioxus::launch(app);
}

#[derive(Debug, Clone, PartialEq)]
struct AppState {
    user: Option<User>,
    todos: Vec<Todo>,
    tags: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
struct User {
    name: String,
    active: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Todo {
    title: String,
    done: bool,
}

// Paired read/write accessors — the only boilerplate the optics helpers need.
fn state_user(s: &AppState) -> &Option<User> { &s.user }
fn state_user_mut(s: &mut AppState) -> &mut Option<User> { &mut s.user }
fn state_todos(s: &AppState) -> &Vec<Todo> { &s.todos }
fn state_todos_mut(s: &mut AppState) -> &mut Vec<Todo> { &mut s.todos }
fn state_tags(s: &AppState) -> &HashMap<String, String> { &s.tags }
fn state_tags_mut(s: &mut AppState) -> &mut HashMap<String, String> { &mut s.tags }
fn user_active(u: &User) -> &bool { &u.active }
fn user_active_mut(u: &mut User) -> &mut bool { &mut u.active }
fn user_name(u: &User) -> &String { &u.name }
fn user_name_mut(u: &mut User) -> &mut String { &mut u.name }
fn todo_title(t: &Todo) -> &String { &t.title }
fn todo_title_mut(t: &mut Todo) -> &mut String { &mut t.title }
fn todo_done(t: &Todo) -> &bool { &t.done }
fn todo_done_mut(t: &mut Todo) -> &mut bool { &mut t.done }

fn initial_state() -> AppState {
    AppState {
        user: Some(User { name: "Alice".into(), active: true }),
        todos: vec![
            Todo { title: "Learn optics".into(), done: false },
            Todo { title: "Ship a demo".into(), done: false },
        ],
        tags: HashMap::from([
            ("urgent".to_string(), "#e33".to_string()),
            ("nice-to-have".to_string(), "#39a".to_string()),
        ]),
    }
}

fn app() -> Element {
    // Single reactive source of truth. Every widget below mutates this signal
    // through an optic path instead of pulling it apart into per-field signals.
    let mut state = use_signal(initial_state);
    let root = Optic::from_access(state);

    // Option<User> -> User via `map_some`, then projections onto `.active` / `.name`.
    let user = root.clone().map_ref_mut(state_user, state_user_mut);
    let active = user.clone().map_some().map_ref_mut(user_active, user_active_mut);
    let name = user.map_some().map_ref_mut(user_name, user_name_mut);

    // Vec<Todo> -> per-item child optics via `each`.
    let todos = root
        .clone()
        .map_ref_mut(state_todos, state_todos_mut)
        .each();

    // HashMap<String, String> -> keyed child optics via `each_hash_map`.
    let tags = root
        .map_ref_mut(state_tags, state_tags_mut)
        .each_hash_map();

    // Each of these `read_opt`/`read` calls registers a subscription on the
    // root signal, so writes from anywhere below re-render the whole tree.
    let is_active = active.read_opt().map(|b| *b).unwrap_or(false);
    let current_name = name
        .read_opt()
        .map(|n| n.clone())
        .unwrap_or_else(|| "<none>".into());

    rsx! {
        style { {CSS} }
        div { class: "app",
            h1 { "dioxus-optics demo" }

            section {
                h2 { "User (Option<User>)" }
                p { "Current user: " b { "{current_name}" } }
                label {
                    input {
                        r#type: "checkbox",
                        checked: is_active,
                        oninput: move |evt| {
                            if let Some(mut w) = active.write_opt() {
                                *w = evt.checked();
                            }
                        },
                    }
                    " Active"
                }
                input {
                    r#type: "text",
                    value: "{current_name}",
                    oninput: move |evt| {
                        if let Some(mut w) = name.write_opt() {
                            *w = evt.value();
                        }
                    },
                }
                button {
                    onclick: move |_| { state.write().user = None; },
                    "Log out"
                }
                button {
                    onclick: move |_| {
                        state.write().user = Some(User {
                            name: "New user".into(),
                            active: true,
                        });
                    },
                    "Log in"
                }
            }

            section {
                h2 { "Todos (Vec<Todo>)" }
                p { "Count: " b { "{todos.len()}" } }
                ul {
                    for (idx, todo) in todos.iter().enumerate() {
                        {
                            // Build the per-field optic chains for this row.
                            // Each chain still reads & writes through the root signal.
                            let done = todo.clone().map_ref_mut(todo_done, todo_done_mut);
                            let title = todo.map_ref_mut(todo_title, todo_title_mut);
                            let done_read = *done.read();
                            let title_read = title.read().clone();
                            let todos_for_remove = todos.clone();
                            rsx! {
                                li { key: "{idx}",
                                    input {
                                        r#type: "checkbox",
                                        checked: done_read,
                                        oninput: move |evt| { *done.write() = evt.checked(); },
                                    }
                                    input {
                                        r#type: "text",
                                        value: "{title_read}",
                                        oninput: move |evt| { *title.write() = evt.value(); },
                                    }
                                    button {
                                        onclick: move |_| { todos_for_remove.remove(idx); },
                                        "✕"
                                    }
                                }
                            }
                        }
                    }
                }
                button {
                    onclick: {
                        let todos = todos.clone();
                        move |_| {
                            let next = todos.len() + 1;
                            todos.push(Todo {
                                title: format!("New todo #{next}"),
                                done: false,
                            });
                        }
                    },
                    "Add todo"
                }
                button {
                    onclick: {
                        let todos = todos.clone();
                        move |_| { todos.retain(|t| !t.done); }
                    },
                    "Clear completed"
                }
            }

            section {
                h2 { "Tags (HashMap<String, String>)" }
                ul {
                    for (key, tag) in tags.iter() {
                        {
                            let color = tag.read().clone();
                            rsx! {
                                li { key: "{key}",
                                    span {
                                        class: "swatch",
                                        style: "background:{color}",
                                    }
                                    " {key}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

const CSS: &str = r#"
body { font-family: system-ui, sans-serif; max-width: 620px; margin: 2rem auto; padding: 0 1rem; }
.app section { margin: 1.5rem 0; padding: 1rem; border: 1px solid #ddd; border-radius: 8px; }
h2 { margin-top: 0; }
input[type=text] { margin: 0 .5rem; padding: .25rem .5rem; }
button { margin: .25rem; padding: .25rem .75rem; }
ul { list-style: none; padding: 0; }
li { display: flex; align-items: center; gap: .25rem; margin: .25rem 0; }
.swatch { display:inline-block; width:1em; height:1em; border-radius: 2px; }
"#;
