# Local State

The first step to dealing with complexity in your app is to refactor your state to be purely local. This encourages good code reuse and prevents leakage of abstractions across component boundaries.

## What it looks like

Let's say we're managing the state for a list of Todos. Whenever we edit the todo, we want the list to update. We might've started building our app but putting everything into a single `use_ref`.

```rust
struct Todo {
    contents: String,
    is_hovered: bool,
    is_editing: bool,
}

let todos = use_ref(&cx, || vec![Todo::new()]);

cx.render(rsx!{
    ul {
        todos.read().iter().enumerate().map(|(id, todo)| rsx!{
            li {
                h1 { "{todo.contents}" }
                onhover: move |_| *todos.write()[id].is_hovered = true;
            }
        })
    }
})
```

As shown above, whenever the todo is hovered, we want to set its state:

```rust
todos.write()[0].is_hovered = true;
```

As the amount of interactions goes up, so does the complexity of our state. Should the "hover" state really be baked into our data model?

Instead, let's refactor our Todo component to handle its own state:

```rust
#[inline_props]
fn Todo<'a>(cx: Scope, todo: &'a Todo) -> Element {
    let is_hovered = use_state(&cx, || false);

    cx.render(rsx!{
        li {
            h1 { "{todo.contents}" }
            onhover: move |_| is_hovered.set(true),
        }
    })
}
```

Now, we can simplify our Todo data model to get rid of local UI state:

```rust
struct Todo {
    contents: String,
}
```

This is not only better for encapsulation and abstraction, but it's only more performant! Whenever a Todo is hovered, we won't need to re-render *every* Todo again - only the Todo that's currently being hovered.
