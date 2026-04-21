//! Inline styles.
//!
//! For quick one-off styling, every CSS property is available as an attribute on HTML
//! elements — use snake_case in Rust (`background_color`) for the kebab-case CSS property
//! (`background-color`). You can also set the raw `style:` attribute with a full string.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        // Each CSS property is just a named attribute
        div {
            background_color: "#1e3a8a",
            color: "white",
            padding: "20px",
            border_radius: "8px",
            font_family: "system-ui, sans-serif",

            h1 { "Inline styling" }
            p { "Property names are snake_case here, kebab-case in real CSS." }
        }

        // The raw `style` attribute accepts any CSS string
        div {
            style: "margin-top: 12px; border: 2px dashed tomato; padding: 10px;",
            "Use `style:` when you want a plain CSS string."
        }

        // Interpolation works too — great for values driven by Rust expressions
        for i in 1..=5 {
            span {
                display: "inline-block",
                width: "32px",
                height: "32px",
                margin: "4px",
                background_color: "hsl({i * 60}, 70%, 55%)",
            }
        }
    }
}
