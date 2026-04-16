# Dioxus Optics

`dioxus-optics` is an experimental crate extracted from the `repo_readme` demo.

It explores a carrier-generic optics model where one field projection can
compose across:

- sync reads
- sync writes
- optional child access
- result branch access
- collection child access
- keyed map child access
- await-time projection

This crate is intentionally small and experimental. It is not wired into
`derive(Store)` or the Dioxus hook APIs directly.

`Optic::new` allocates through a Dioxus runtime owner, so standalone examples
create a tiny `VirtualDom` and run optics code inside its root scope.

## Example

```rust
use dioxus_core::{ScopeId, VNode, VirtualDom};
use dioxus_optics::prelude::*;

#[derive(Clone)]
struct App {
    todos: Vec<Todo>,
}

#[derive(Clone)]
struct Todo {
    done: bool,
}

fn app_todos(app: &App) -> &Vec<Todo> {
    &app.todos
}

fn app_todos_mut(app: &mut App) -> &mut Vec<Todo> {
    &mut app.todos
}

fn todo_done(todo: &Todo) -> &bool {
    &todo.done
}

fn todo_done_mut(todo: &mut Todo) -> &mut bool {
    &mut todo.done
}

fn with_runtime<R>(f: impl FnOnce() -> R) -> R {
    let mut dom = VirtualDom::new(VNode::empty);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, f)
}

with_runtime(|| {
    let app = Optic::new(App {
        todos: vec![Todo { done: false }],
    });

    let todos = app.clone().map_ref_mut(app_todos, app_todos_mut);
    assert_eq!(todos.read().len(), 1);

    for todo in todos.each::<Todo>().iter() {
        *todo.map_ref_mut(todo_done, todo_done_mut).write() = true;
    }
});
```
