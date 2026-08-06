#[test]
fn rsx() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/rsx/trailing-comma-0.rs");
}

/// This test ensures that automatic `into` conversion occurs for default values.
///
/// These are compile-time tests.
/// See https://github.com/DioxusLabs/dioxus/issues/2373
#[cfg(test)]
mod test_default_into {
    use dioxus::prelude::*;

    #[derive(Props, Clone, PartialEq)]
    struct MyCoolProps {
        // Test different into configurations
        #[props(into, default = true)]
        pub val_into_w_default_val: u16,

        #[props(into, default)]
        pub val_into_w_default: u16,

        #[props(default = true.into())]
        pub val_default: u16,

        // Test different into configurations with strings
        #[props(into, default = "abc")]
        pub str_into_w_default_val: String,

        #[props(into, default)]
        pub str_into_w_default: String,

        #[props(default = "abc".into())]
        pub str_default: String,

        // Test options
        #[props(into, default = Some("abc".into()))]
        pub opt_into_w_default_val: Option<String>,

        #[props(into, default)]
        pub opt_into_w_default: Option<String>,

        #[props(default = Some("abc".into()))]
        pub opt_default: Option<String>,

        pub opt_element: Option<Element>,

        // Test no default
        #[props(into)]
        pub some_data: bool,

        pub some_other_data: bool,

        // Test default values for signals
        #[props(default)]
        read_only_w_default: ReadSignal<bool>,

        #[props(default = true)]
        read_only_w_default_val: ReadSignal<bool>,

        #[props(default = ReadSignal::new(Signal::new(true)))]
        read_only_w_default_val_explicit: ReadSignal<bool>,

        // Test default values for callbacks/event handlers
        #[props(default)]
        callback_w_default: Callback,

        #[props(default = move |_| {})]
        callback_w_default_val_closure: Callback,

        #[props(default = {
            fn test(_: ()) {}
            test
        })]
        callback_w_default_val_expr_fn: Callback,

        #[props(default = Callback::new(move |_: ()| {}))]
        callback_w_default_val_explicit: Callback,

        #[props(default)]
        event_handler_w_default: EventHandler<KeyboardEvent>,

        #[props(default = move |_| {})]
        event_handler_w_default_val_closure: EventHandler<KeyboardEvent>,

        #[props(default = {
            fn test(_: KeyboardEvent) {}
            test
        })]
        event_handler_w_default_val_expr_fn: EventHandler<KeyboardEvent>,

        #[props(default = EventHandler::new(move |_: KeyboardEvent| {}))]
        event_handler_w_default_val_explicit: EventHandler<KeyboardEvent>,
    }
}

/// This test ensures that read-only signals that contain an option (`Signal<Option<u16>>`)
/// are correctly created as default when not provided.
///
/// These are compile-time tests.
/// See https://github.com/DioxusLabs/dioxus/issues/2648
#[cfg(test)]
#[allow(unused)]
mod test_optional_signals {
    use dioxus::prelude::*;

    // Test if test components fail to compile.
    #[component]
    fn UsesComponents() -> Element {
        rsx! {
            PropsStruct {
                regular_read_signal: ReadSignal::new(Signal::new(1234)),
            }
            PropsStruct {
                optional_read_signal: 1234,
                regular_read_signal: 123u16,
            }
            PropParams {}
            PropParams {
                opt_read_sig: 1234
            }
            DoubleOption {}
            DoubleOption { optional: Some(1234) }
        }
    }

    // Test props as struct param.
    #[derive(Props, Clone, PartialEq)]
    struct MyTestProps {
        pub optional_read_signal: ReadSignal<Option<u16>>,
        pub regular_read_signal: ReadSignal<u16>,
    }

    #[component]
    fn PropsStruct(props: MyTestProps) -> Element {
        rsx! { "hi" }
    }

    // Test props as params.
    #[component]
    fn PropParams(opt_read_sig: ReadSignal<Option<u16>>) -> Element {
        rsx! { "hi!" }
    }

    #[component]
    fn DoubleOption(optional: Option<Option<u16>>) -> Element {
        rsx! { "hi!" }
    }
}

/// rsx evaluates expressions in three tiers: formatted strings that only borrow, then everything
/// else in source order, then event handlers. These are compile-time tests for the patterns that
/// ordering is supposed to keep working.
///
/// See https://github.com/DioxusLabs/dioxus/issues/3737
#[cfg(test)]
#[allow(unused)]
mod test_evaluation_order {
    use dioxus::prelude::*;

    #[component]
    fn TakesString(text: String) -> Element {
        rsx! { "{text}" }
    }

    #[component]
    fn TakesCallback(label: String, onpick: EventHandler<()>) -> Element {
        rsx! { "{label}" }
    }

    fn consume(_: String) {}

    /// An attribute that borrows can be written before a child that moves
    fn attribute_then_component() -> Element {
        let text = String::from("hello");
        rsx! {
            div { width: "{text}",
                TakesString { text }
            }
        }
    }

    /// A component that borrows can be written before an attribute that moves
    fn component_then_attribute() -> Element {
        let text = String::from("hello");
        rsx! {
            div {
                TakesString { text: "{text}" }
                div { width: text }
            }
        }
    }

    /// Event handlers are evaluated last, so a handler can move a value that a formatted string
    /// somewhere else in the template borrows
    fn handler_then_formatted() -> Element {
        let text = String::from("hello");
        rsx! {
            div {
                button { onclick: move |_| consume(text.clone()), "pick" }
                span { "{text}" }
            }
        }
    }

    /// ...including when the formatted string is an attribute, or nested deeper in the tree
    fn handler_then_nested_formatted() -> Element {
        let text = String::from("hello");
        rsx! {
            div { onclick: move |_| consume(text.clone()),
                div {
                    span { class: "{text}", "{text}" }
                }
            }
        }
    }

    /// Callback props are applied to the props builder after the props that only borrow
    fn callback_prop_then_prop() -> Element {
        let text = String::from("hello");
        rsx! {
            TakesCallback {
                onpick: move |_| consume(text.clone()),
                label: "{text}",
            }
        }
    }

    /// The shape of every list row: a handler that owns the item and a label that displays it
    fn list_row() -> Element {
        let items = vec![String::from("a"), String::from("b")];
        rsx! {
            for item in items {
                div { key: "{item}",
                    button { onclick: move |_| consume(item.clone()), "pick" }
                    span { "{item}" }
                }
            }
        }
    }

    /// Formatted strings only borrow, so they can be written after the value is moved elsewhere
    fn move_then_formatted_sibling() -> Element {
        let text = String::from("hello");
        rsx! {
            div {
                TakesString { text: text.clone() }
                span { "{text}" }
            }
        }
    }
}
