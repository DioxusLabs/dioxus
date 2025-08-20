# Dioxus Stores

Stores are an extension to the Dioxus signals system for reactive nested data structures. Each store will lazily create signals for each field/member of the data structure as needed.

By default stores act a lot like [`dioxus_signals::Signal`]s, but they provide more granular
subscriptions without requiring nested signals. You should derive [`Store`](dioxus_stores_macro::Store) on your data
structures to generate selectors that let you scope the store to a specific part of your data structure.

```rust, no_run
use dioxus::prelude::*;
use dioxus_stores::*;

fn main() {
    dioxus::launch(app);
}

// Deriving the store trait provides methods to scope the store to specific parts of your data structure.
// The `Store` macro generates a `count` and `children` method for `Store<CounterTree>`
#[derive(Store, Default)]
struct CounterTree {
    count: i32,
    children: Vec<CounterTree>,
}

fn app() -> Element {
    let value = use_store(Default::default);

    rsx! {
        Tree {
            value
        }
    }
}

#[component]
fn Tree(value: Store<CounterTree>) -> Element {
    // Calling the generated `count` method returns a new store that can only
    // read and write the count field
    let mut count = value.count();
    let mut children = value.children();

    rsx! {
        button {
            // Incrementing the count will only rerun parts of the app that have read the count field
            onclick: move |_| count += 1,
            "Increment"
        }
        button {
            // Stores are aware of data structures like `Vec` and `Hashmap`. When we push an item to the vec
            // it will only rerun the parts of the app that depend on the length of the vec
            onclick: move |_| children.push(Default::default()),
            "Push child"
        }
        ul {
            // Iterating over the children gives us stores scoped to each child.
            for value in children.iter() {
                li {
                    Tree { value }
                }
            }
        }
    }
}
```
