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

The same mapping helpers (`map_ref`, `map_ref_mut`, `map_some`, `map_ok`,
`map_err`, `map_variant`, `map_variant_with`, `iter`, `get`, `flatten_some`,
`to_option`) work uniformly over any carrier that implements
`dioxus_signals::Readable` — including `CopyValue`, `Signal`, `SyncSignal`,
`Memo`, `ReadSignal`, `WriteSignal`, `GlobalSignal`, `GlobalMemo`, and
`dioxus_stores::Store`. The helpers are exposed as the [`OpticExt`] /
[`OpticIter`] / [`OpticMutExt`] extension traits so call sites read as
`signal.map_ref(...)` / `signal.iter()` without first wrapping the carrier.
`iter` dispatches on the target shape (`Vec<T>`, `HashMap<K, V, S>`,
`BTreeMap<K, V>`) and returns a reusable carrier that implements
`IntoIterator` for `&Self` so it can be iterated any number of times.
Write helpers apply whenever the underlying carrier is also `Writable`;
`map_ref` gives a read-only path appropriate for `Memo`.

Enum variants project through the `Prism` primitive. `map_some` / `map_ok` /
`map_err` are sugar over prisms for `Option` and `Result`; `map_variant::<P>()`
and `map_variant_with(try_ref, try_mut, try_into)` cover any user-defined sum
type. See `examples/page_router.rs` for a multi-variant walkthrough.

In application code, just call the helpers directly on a `Signal` / `Store` /
`Memo` / etc. — `signal.map_ref_mut(...)`, `store.iter()`, `memo.map_some()`.
The blanket `OpticExt` / `OpticIter` / `OpticMutExt` impls cover every
[`Access`] / [`AccessMut`] carrier; you only need `Optic::from_access` when
wrapping a custom carrier (an `Access` impl that lives outside the standard
reactive types). `Optic::new(value)` allocates a `CopyValue` through a Dioxus
runtime owner and is mostly useful for standalone examples that build a tiny
`VirtualDom` and run optics code inside its root scope.

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

    for todo in &todos.iter() {
        *todo.map_ref_mut(todo_done, todo_done_mut).write() = true;
    }
});
```
