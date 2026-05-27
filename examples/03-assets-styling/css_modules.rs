//! This example shows how to use css modules with the `css_module` macro. Css modules convert css
//! class names to unique names to avoid class name collisions.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // Each `css_module` macro will expand the annotated struct in the current scope
    #[css_module("/examples/assets/css_module1.css")]
    struct Styles;

    #[css_module(
        "/examples/assets/css_module2.css",
        // `css_module` can take `AssetOptions` as well
        AssetOptions::css_module()
            .with_minify(true)
            .with_preload(false)
    )]
    struct OtherStyles;

    rsx! {
        div { class: Styles::container,
            div { class: OtherStyles::test, "Hello, world!" }
            div { class: OtherStyles::highlight, "This is highlighted" }
            div { class: Styles::global_class, "This uses a global class (no hash)" }
        }
    }
}
