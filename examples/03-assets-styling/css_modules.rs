//! This example shows how to use css modules with the `styles!` macro. Css modules convert css
//! class names to unique names to avoid class name collisions.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // Each `style!` macro will generate a `Styles` struct in the current scope
    styles!("/examples/assets/css_module1.css");

    mod other {
        use dioxus::prelude::*;
        // Multiple `styles!` macros can be used in the same scope by placing them in modules
        styles!(
            "/examples/assets/css_module2.css",
            // `styles!` can take `AssetOptions` as well
            AssetOptions::css_module()
                .with_minify(true)
                .with_preload(false)
        );
    }

    rsx! {
        div { class: Styles::container,
            div { class: other::Styles::test, "Hello, world!" }
            div { class: other::Styles::highlight, "This is highlighted" }
            div { class: Styles::global_class, "This uses a global class (no hash)" }
        }
    }
}
