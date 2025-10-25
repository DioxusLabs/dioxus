Creates a new Signal that is derived from other state. The derived signal will automatically update whenever any of the reactive values it reads on are written to. Note the signal is not memorized and the update is not immediate, the signal will be set to the value after the next async tick. Derived signals are useful for initializing values from props.

```rust
# use dioxus::prelude::*;
#
# fn App() -> Element {
#     rsx! {
#         Router::<Route> {}
#     }
# }
#[derive(Routable, Clone)]
enum Route {
    // When you first navigate to this route, initial_count will be used to set the value of
    // the count signal
    #[route("/:initial_count")]
    Counter { initial_count: u32 },
}

#[component]
fn Counter(initial_count: ReadSignal<u32>) -> Element {
    // The count will reset to the value of the prop whenever the prop changes
    let mut count = use_derived_signal(move || initial_count());

    rsx! {
        button {
            onclick: move |_| count += 1,
            "{count}"
        }
        Link {
            // Navigating to this link will change the initial_count prop to 10. Note, this
            // only updates the props, the component is not remounted
            to: Route::Counter { initial_count: 10 },
            "Go to initial count 10"
        }
    }
}
```
