//! The typical TodoMVC app, implemented in Dioxus with stores. Stores let us
//! share nested reactive state between components. They let us keep our todomvc
//! state in a single struct without wrapping every type in a signal while still
//! maintaining fine grained reactivity.

use dioxus::prelude::*;
use std::{collections::HashMap, vec};

const STYLE: Asset = asset!("/examples/assets/todomvc.css");

/// Deriving the store macro on a struct will automatically generate an extension trait
/// for Store<TodoState> with method to zoom into the fields of the struct.
///
/// For this struct, the macro derives the following methods for Store<TodoState>:
/// - `todos(self) -> Store<HashMap<u32, TodoItem>, _>`
/// - `filter(self) -> Store<FilterState, _>`
#[derive(Store, PartialEq, Clone, Debug)]
struct TodoState {
    todos: HashMap<u32, TodoItem>,
    filter: FilterState,
}

// We can also add custom methods to the store by using the `store` attribute on an impl block.
// The store attribute turns the impl block into an extension trait for Store<TodoState>.
// Methods that take &self will automatically get a bound that Lens: Readable<Target = TodoState>
// Methods that take &mut self will automatically get a bound that Lens: Writable<Target = TodoState>
#[store]
impl<Lens> Store<TodoState, Lens> {
    fn active_items(&self) -> Vec<u32> {
        let filter = self.filter().cloned();
        let mut active_ids: Vec<u32> = self
            .todos()
            .iter()
            .filter_map(|(id, item)| item.active(filter).then_some(id))
            .collect();
        active_ids.sort_unstable();
        active_ids
    }

    fn incomplete_count(&self) -> usize {
        self.todos()
            .values()
            .filter(|item| item.incomplete())
            .count()
    }

    fn toggle_all(&mut self) {
        let check = self.incomplete_count() != 0;
        for item in self.todos().values() {
            item.checked().set(check);
        }
    }

    fn has_todos(&self) -> bool {
        !self.todos().is_empty()
    }
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

#[store]
impl<Lens> Store<TodoItem, Lens> {
    fn complete(&self) -> bool {
        self.checked().cloned()
    }

    fn incomplete(&self) -> bool {
        !self.complete()
    }

    fn active(&self, filter: FilterState) -> bool {
        match filter {
            FilterState::All => true,
            FilterState::Active => self.incomplete(),
            FilterState::Completed => self.complete(),
        }
    }
}

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // We store the state of our todo list in a store to use throughout the app.
    let mut todos = use_store(|| TodoState {
        todos: HashMap::new(),
        filter: FilterState::All,
    });

    // We use a simple memo to calculate the number of active todos.
    // Whenever the todos change, the active_todo_count will be recalculated.
    let active_todo_count = use_memo(move || todos.incomplete_count());

    // We use a memo to filter the todos based on the current filter state.
    // Whenever the todos or filter change, the filtered_todos will be recalculated.
    // Note that we're only storing the IDs of the todos, not the todos themselves.
    let filtered_todos = use_memo(move || todos.active_items());

    // Toggle all the todos to the opposite of the current state.
    // If all todos are checked, uncheck them all. If any are unchecked, check them all.
    let toggle_all = move |_| {
        todos.toggle_all();
    };

    rsx! {
        Stylesheet { href: STYLE }
        section { class: "todoapp",
            TodoHeader { todos }
            section { class: "main",
                if todos.has_todos() {
                    input {
                        id: "toggle-all",
                        class: "toggle-all",
                        r#type: "checkbox",
                        onchange: toggle_all,
                        checked: active_todo_count() == 0
                    }
                    label { r#for: "toggle-all" }
                }

                // Render the todos using the filtered_todos memo
                // We pass the ID along with the hashmap into the TodoEntry component so it can access the todo from the todos store.
                ul { class: "todo-list",
                    for id in filtered_todos() {
                        TodoEntry { key: "{id}", id, todos }
                    }
                }

                // We only show the footer if there are todos.
                if todos.has_todos() {
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
        if evt.key() == Key::Enter && !draft.is_empty() {
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
/// This takes the ID of the todo and the todos store as props
/// We can use these together to memoize the todo contents and checked state
#[component]
fn TodoEntry(mut todos: Store<TodoState>, id: u32) -> Element {
    let mut is_editing = use_signal(|| false);

    // When we get an item out of the store, it will only subscribe to that specific item.
    // Since we only get the single todo item, the component will only rerender when that item changes.
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
    // We use a memo to calculate whether we should show the "Clear completed" button.
    // This will recompute whenever the number of todos change or the checked state of an existing
    // todo changes
    let show_clear_completed = use_memo(move || todos.todos().values().any(|todo| todo.complete()));
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
