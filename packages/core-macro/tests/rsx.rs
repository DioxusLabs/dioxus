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
        #[props(into, default = Some("abc"))]
        pub opt_into_w_default_val: Option<String>,

        #[props(into, default)]
        pub opt_into_w_default: Option<String>,

        #[props(default = Some("abc"))]
        pub opt_default: Option<String>,

        // Test no default
        #[props(into)]
        pub some_data: bool,

        pub some_other_data: bool,
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
                regular_read_signal: ReadOnlySignal::new(Signal::new(1234)),
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
        pub optional_read_signal: ReadOnlySignal<Option<u16>>,
        pub regular_read_signal: ReadOnlySignal<u16>,
    }

    #[component]
    fn PropsStruct(props: MyTestProps) -> Element {
        rsx! { "hi" }
    }

    // Test props as params.
    #[component]
    fn PropParams(opt_read_sig: ReadOnlySignal<Option<u16>>) -> Element {
        rsx! { "hi!" }
    }

    #[component]
    fn DoubleOption(optional: Option<Option<u16>>) -> Element {
        rsx! { "hi!" }
    }
}
