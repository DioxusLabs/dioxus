//! Applying classes and styles based on state.
//!
//! Any attribute can interpolate signals and expressions using `"{value}"` formatting.
//! For `class:`, you can also build the class string inline with if/else, and individual
//! CSS properties can be set as named attributes like `color:` or `background_color:`.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut is_active = use_signal(|| false);
    let mut size = use_signal(|| 16);

    // Build a class string from state
    let classes = if is_active() { "btn active" } else { "btn" };

    rsx! {
        // CSS properties are plain named attributes — snake_case in Rust, kebab-case in CSS
        style { {STYLE} }

        button {
            class: classes,
            onclick: move |_| is_active.toggle(),
            "Toggle active (currently {is_active})"
        }

        // Or compute the class string ahead of time for anything more complex
        div {
            class: if is_active() { "box highlight" } else { "box" },
            "This box changes class with the button."
        }

        hr {}

        // Style values can be driven directly by signals
        p {
            font_size: "{size}px",
            color: if size() > 24 { "crimson" } else { "black" },
            "Font size: {size}px"
        }
        button { onclick: move |_| size += 2, "Bigger" }
        button { onclick: move |_| size -= 2, "Smaller" }
    }
}

const STYLE: &str = r#"
.btn { padding: 6px 12px; border: 1px solid #888; border-radius: 4px; }
.btn.active { background: #4CAF50; color: white; border-color: #2E7D32; }
.box { padding: 12px; margin-top: 12px; border: 1px dashed #999; }
.box.highlight { background: #fff3cd; border-color: #d39e00; }
"#;
