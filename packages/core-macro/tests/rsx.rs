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
