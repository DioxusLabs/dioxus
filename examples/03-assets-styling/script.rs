//! Adding a script to the document head.
//!
//! `document::Script` injects a `<script>` tag into the page. Use it to embed an inline
//! snippet via children, or load an external script by passing a `src`. Scripts are
//! deduplicated by their `src`, so mounting the same script from multiple components is safe.
//!
//! Prefer `Script` over `link {}` for better integration with SSR and pre-rendering.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        // Inline script — children must be a single text node
        document::Script { "console.log('Hello from an inline script!');" }

        // External script, loaded with `defer` so it runs after parsing
        document::Script { src: "https://cdn.jsdelivr.net/npm/canvas-confetti@1.9.3/dist/confetti.browser.min.js", defer: true }

        h1 { "Script example" }
        p { "Open your browser's console to see the inline script's output." }

        button {
            onclick: move |_| {
                // Call out to the confetti library loaded above
                document::eval("confetti && confetti();");
            },
            "🎉 Confetti"
        }
    }
}
