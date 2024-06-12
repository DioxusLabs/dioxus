<details>
<summary>How should I move around state? (Click here to learn more about moving around and sharing state in Dioxus)</summary>

You will often need to move state around between your components. Dioxus provides three different ways to pass around state:

1. Just pass your values as [props](https://dioxuslabs.com/learn/0.5/reference/component_props):

```rust
# use dioxus::prelude::*;
fn MyComponent() -> Element {
    let count = use_signal(|| 0);

    rsx! {
        IncrementButton {
            count
        }
    }
}

#[component]
fn IncrementButton(mut count: Signal<i32>) -> Element {
    rsx! {
        button {
            onclick: move |_| count += 1,
            "Increment"
        }
    }
}
```

This is the most common way to pass state around. It is the most explicit and local to your component. Use this when it isn't overly annoying to pass around a value.

2. Use [use_context](https://dioxuslabs.com/learn/0.5/reference/context) to pass state from a parent component to all children:

```rust
# use dioxus::prelude::*;
#[derive(Clone, Copy)]
struct MyState {
    count: Signal<i32>
}

fn ParentComponent() -> Element {
    // Use context provider provides an unique type to all children of this component
    use_context_provider(|| MyState { count: Signal::new(0) });

    rsx! {
        // IncrementButton will have access to the count without explicitly passing it through props
        IncrementButton {}
    }
}

#[component]
fn IncrementButton() -> Element {
    // Use context gets the value from a parent component
    let mut count = use_context::<MyState>().count;

    rsx! {
        button {
            onclick: move |_| count += 1,
            "Increment"
        }
    }
}
```

This is slightly less explicit than passing it as a prop, but it is still local to the component. This is really great if you want state that is global to part of your app. It lets you create multiple global-ish states while still making state different when you reuse components. If I create a new `ParentComponent`, it will have a new `MyState`.

3. Globals let you share state with your whole app with rust statics:

```rust
# use dioxus::prelude::*;
// Count will be created the first time you access it with the closure you pass to Signal::global
static COUNT: GlobalSignal<i32> = Signal::global(|| 0);

fn ParentComponent() -> Element {
    rsx! {
        IncrementButton {}
    }
}

fn IncrementButton() -> Element {
    rsx! {
        button {
            // You don't need to pass anything around or get anything out of the context because COUNT is global
            onclick: move |_| *COUNT.write() += 1,
            "Increment"
        }
    }
}
```

Global state can be very ergonomic if your state is truly global, but you shouldn't use it if you need state to be different for different instances of your component. If I create another `IncrementButton` it will use the same `COUNT`. Libraries should generally avoid this to make components more reusable.

> Note: Even though it is in a static, `COUNT` will be different for each app instance (this is generally only reliant on the server).

</details>
