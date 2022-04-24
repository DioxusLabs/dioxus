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
                onhover: move |_| *todos.write()[id].is_hovered = true,
                h1 { "{todo.contents}" }
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


Wherever possible, you should try to refactor the "view" layer *out* of your data model.

## Immutability

Storing all of your state inside a `use_ref` might be tempting. However, you'll quickly run into an issue where the `Ref` provided by `read()` "does not live long enough." An indeed - you can't return locally-borrowed value into the Dioxus tree.

In these scenarios consider breaking your `use_ref` into individual `use_state`s.

You might've started modeling your component with a struct and use_ref

```rust
struct State {
    count: i32,
    color: &'static str,
    names: HashMap<String, String>
}

// in the component
let state = use_ref(&cx, State::new)
```

The "better" approach for this particular component would be to break the state apart into different values:

```rust
let count = use_state(&cx, || 0);
let color = use_state(&cx, || "red");
let names = use_state(&cx, HashMap::new);
```

You might recognize that our "names" value is a HashMap - which is not terribly cheap to clone every time we update its value. To solve this issue, we *highly* suggest using a library like [`im`](https://crates.io/crates/im) which will take advantage of structural sharing to make clones and mutations much cheaper.

When combined with the `make_mut` method on `use_state`, you can get really succinct updates to collections:

```rust
let todos = use_state(&cx, im_rc::HashMap::default);

todos.make_mut().insert("new todo", Todo {
    contents: "go get groceries",
});
```

## Moving on

This particular local patterns are powerful but is not a cure-all for state management problems. Sometimes your state problems are much larger than just staying local to a component.


