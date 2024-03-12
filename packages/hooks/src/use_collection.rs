/*
a form of use_state explicitly for map-style collections (BTreeMap, HashMap, etc).

Why?
---
Traditionally, it's possible to use the "use_state" hook for collections in the React world.
Adding a new entry would look something similar to:

```js
let (map, set_map) = useState({});
set_map({ ...map, [key]: value });
```
The new value then causes the appropriate update when passed into children.

This is moderately efficient because the fields of the map are moved, but the data itself is not cloned.
However, if you used similar approach with Dioxus:

```rust
let (map, set_map) = use_signal(|| HashMap::new());
set_map({
    let mut newmap = map.clone();
    newmap.set(key, value);
    newmap
})
```
Unfortunately, you'd be cloning the entire state every time a value is changed. The obvious solution is to
wrap every element in the HashMap with an Rc. That way, cloning the HashMap is on par with its JS equivalent.

Fortunately, we can make this operation even more efficient in Dioxus, leveraging the borrow rules of Rust.

This hook provides a memoized collection, memoized setters, and memoized getters. This particular hook is
extremely powerful for implementing lists and supporting core state management needs for small apps.

If you need something even more powerful, check out the dedicated atomic state management Dioxus Dataflow, which
uses the same memoization on top of the use_context API.

Here's a fully-functional todo app using the use_map API:
```rust
static TodoList: Component = |cx| {
    let todos = use_map(|| HashMap::new());
    let input = use_signal(|| None);

    rsx!{
        div {
            button {
                "Add todo"
                onclick: move |_| {
                    let new_todo = TodoItem::new(input.contents());
                    todos.insert(new_todo.id.clone(), new_todo);
                    input.clear();
                }
            }
            button {
                "Clear todos"
                onclick: move |_| todos.clear()
            }
            input {
                placeholder: "What needs to be done?"
                ref: input
            }
            ul {
                {todos.iter().map(|todo| rsx!(
                    li {
                        key: todo.id
                        span { "{todo.content}" }
                        button {"x", onclick: move |_| todos.remove(todo.key.clone())}
                    }
                ))}
            }
        }
    })
}

```

*/
