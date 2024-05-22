use dioxus::prelude::*;

// This test just checks that event handlers compile without explicit type annotations
// It will not actually run any code
#[test]
#[allow(unused)]
fn event_handlers_compile() {
    fn app() -> Element {
        let mut todos = use_signal(String::new);
        rsx! {
            input {
                // Normal event handlers work without explicit type annotations
                oninput: move |evt| todos.set(evt.value()),
            }
            button {
                // async event handlers work without explicit type annotations
                onclick: |event| async move {
                    println!("{event:?}");
                },
            }

            // New! You can now use async closures for custom event handlers!
            // This shouldn't require an explicit type annotation
            TakesEventHandler { onclick: |event| async move {
                println!("{event:?}");
            } }
            // Or you can accept a callback that returns a value
            // This shouldn't require an explicit type annotation
            TakesEventHandlerWithArg { double: move |value| (value * 2) as i32 }
        }
    }

    #[component]
    fn TakesEventHandler(onclick: EventHandler<MouseEvent>) -> Element {
        rsx! {
            button {
                // You can pass in EventHandlers directly to events
                onclick: onclick,
                "Click!"
            }
            button {
                // Or use the shorthand syntax
                onclick,
                "Click!"
            }

            // You should also be able to forward event handlers to other components with the shorthand syntax
            TakesEventHandler {
                onclick
            }
        }
    }

    #[component]
    fn TakesEventHandlerWithArg(double: Callback<u32, i32>) -> Element {
        let mut count = use_signal(|| 2);
        rsx! {
            button {
                // Callbacks let you easily inject custom logic into your components
                onclick: move |_| count.set(double(count()) as u32),
                "{count}"
            }
        }
    }
}
