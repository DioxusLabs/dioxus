//! Styling driven by state.
//!
//! Style attributes are interpolated the same way text is — read from signals inside `"{...}"`
//! or pass a conditional expression directly. The element re-renders only when the read
//! signals change, so you can freely animate colors, sizes, and transforms this way.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut hue = use_signal(|| 200.0_f32);
    let mut size = use_signal(|| 100.0_f32);
    let mut rotated = use_signal(|| false);

    rsx! {
        h1 { "Dynamic styling" }

        div {
            width: "{size}px",
            height: "{size}px",
            background_color: "hsl({hue}, 80%, 60%)",
            transition: "transform 300ms ease",
            transform: if rotated() { "rotate(45deg)" } else { "rotate(0deg)" },
            margin: "20px 0",
        }

        div {
            label { "Hue: {hue:.0}" }
            input {
                r#type: "range",
                min: "0",
                max: "360",
                value: "{hue}",
                oninput: move |evt| {
                    if let Ok(v) = evt.value().parse::<f32>() {
                        hue.set(v);
                    }
                },
            }
        }

        div {
            label { "Size: {size:.0}px" }
            input {
                r#type: "range",
                min: "40",
                max: "300",
                value: "{size}",
                oninput: move |evt| {
                    if let Ok(v) = evt.value().parse::<f32>() {
                        size.set(v);
                    }
                },
            }
        }

        button { onclick: move |_| rotated.toggle(), "Toggle rotation" }
    }
}
