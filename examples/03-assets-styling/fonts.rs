//! Loading fonts — from a CDN and bundled with the app.
//!
//! Two common ways to get a custom typeface onto the page:
//!
//! 1. Point `Stylesheet` at a hosted CSS file like Google Fonts — the browser fetches the
//!    font files on demand.
//! 2. Bundle a font file into the app with `asset!`, then declare an `@font-face` rule
//!    that points at it. This avoids the network round-trip and makes the font available
//!    offline.

use dioxus::prelude::*;

// A local font, bundled into the app. The asset macro fingerprints and copies the file
// into the final build; at runtime `INTER` formats to the public URL.
const INTER: Asset = asset!("/examples/assets/Inter-Regular.woff2");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        // Approach 1 — request a stylesheet from the Google Fonts CDN
        Stylesheet { href: "https://fonts.googleapis.com/css2?family=Lobster&family=Roboto+Mono&display=swap" }

        // Approach 2 — register a bundled font with an inline @font-face rule.
        // `{INTER}` interpolates to the asset's resolved URL at runtime.
        style { "@font-face {{ font-family: 'Inter'; src: url('{INTER}') format('woff2'); font-display: swap; }}" }

        h1 { font_family: "'Lobster', cursive", font_size: "48px",
            "From Google Fonts"
        }
        p { font_family: "'Roboto Mono', monospace",
            "This paragraph also comes from the CDN."
        }

        hr {}

        h2 { font_family: "'Inter', sans-serif", "Bundled Inter font" }
        p { font_family: "'Inter', sans-serif",
            "This paragraph uses Inter, shipped with the app — no network request needed."
        }
    }
}
