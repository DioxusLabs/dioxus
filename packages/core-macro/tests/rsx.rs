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
/// This test ensures that signals that contain an option (`Signal<Option<u16>>`) and the
/// read-only varients are correctly created as default when not provided.
///
/// These are compile-time tests.
/// See https://github.com/DioxusLabs/dioxus/issues/2648
#[cfg(test)]
mod test_optional_signals {
    use dioxus::prelude::*;

    // Test if test components fail to compile.
    #[component]
    fn UsesComponents() -> Element {
        rsx! {
            PropsStruct {}
            PropsParams {}
        }
    }

    // Test props as struct param.
    #[derive(Props, Clone, PartialEq)]
    struct MyTestProps {
        pub optional_signal: Signal<Option<u16>>,
        pub optional_read_signal: ReadOnlySignal<Option<16>>,
    }

    #[component]
    fn PropsStruct(_props: MyTestProps) -> Element {
        rsx! { "hi" }
    }

    // Test props as params.
    #[component]
    fn PropParams(
        opt_sig: Signal<Option<u16>>,
        opt_read_sig: ReadOnlySignal<Option<u16>>,
    ) -> Element {
        rsx! { "hi!" }
    }
}
