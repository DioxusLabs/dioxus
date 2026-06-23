use dioxus::core::view::ViewExt;
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

#[test]
#[allow(unused)]
fn builder_extensions_compile_from_prelude() {
    fn app() -> Element {
        rsx! {
            button {
                class: "primary",
                r#type: "button",
                onclick: |_| {},
                "Click!"
            }
        }
    }
}

#[test]
#[allow(unused)]
fn component_builders_compile_as_nodes() {
    fn app() -> Element {
        Ok(Dashboard
            .builder()
            .title("HTML builder API")
            .count(3)
            .build()
            .into_vnode())
    }

    #[component]
    fn Dashboard(#[props(into)] title: String, count: usize) -> Element {
        Ok(html::main
            .child((
                Header.builder().title(title).build(),
                (0..count).map(|index| Card.builder().index(index).build()),
            ))
            .into_vnode())
    }

    #[component]
    fn Header(#[props(into)] title: String) -> Element {
        rsx! {
            h1 { "{title}" }
        }
    }

    #[component]
    fn Card(index: usize) -> Element {
        rsx! {
            article { "{index}" }
        }
    }
}

// Regression test: a `move` event handler that captures a non-`Copy` value must coexist with a
// sibling node that borrows the same value. Dynamic node values are bound to locals before the
// builder chain, so the borrow completes before the closure moves the value.
#[test]
#[allow(unused)]
fn move_closure_with_sibling_borrow_compiles() {
    fn app() -> Element {
        let mut selected = use_signal(String::new);
        let items = vec![String::from("a"), String::from("b")];
        rsx! {
            for wl in items {
                // The attribute closure moves `wl`; the sibling child borrows `wl` via interpolation.
                li {
                    onclick: move |_| selected.set(wl.clone()),
                    b { "{wl}" }
                }
            }
        }
    }
}

// Regression test: the same hazard across nesting — a parent element's attribute closure moves a
// value that a nested child node borrows.
#[test]
#[allow(unused)]
fn move_closure_with_nested_child_borrow_compiles() {
    fn app() -> Element {
        let mut selected = use_signal(String::new);
        let items = vec![String::from("a")];
        rsx! {
            for wl in items {
                div {
                    onclick: move |_| selected.set(wl.clone()),
                    span { "{wl}" }
                }
            }
        }
    }
}

#[test]
#[allow(unused)]
fn child_owned_component_builders_compile_as_nodes() {
    fn app() -> Element {
        let count = use_signal(|| 0);

        Ok(TakesSignal.builder().count(count).build().into_vnode())
    }

    #[component]
    fn TakesSignal(count: ReadSignal<i32>) -> Element {
        rsx! {
            "{count}"
        }
    }
}
