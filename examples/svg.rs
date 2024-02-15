//! Thanks to @japsu and their project https://github.com/japsu/jatsi for the example!
//!
//! This example shows how to create a simple dice rolling app using SVG and Dioxus.
//! The `svg` element and its children have a custom namespace, and are attached using different methods than regular
//! HTML elements. Any element can specify a custom namespace by using the `namespace` meta attribute.
//!
//! If you `go-to-definition` on the `svg` element, you'll see its custom namespace.

use dioxus::prelude::*;
use rand::{thread_rng, Rng};

fn main() {
    launch(|| {
        rsx! {
            div { user_select: "none", webkit_user_select: "none", margin_left: "10%", margin_right: "10%",
                h1 { "Click die to generate a new value" }
                div { cursor: "pointer", height: "100%", width: "100%", Dice {} }
            }
        }
    });
}

#[component]
fn Dice() -> Element {
    const Y: bool = true;
    const N: bool = false;
    const DOTS: [(i64, i64); 7] = [(-1, -1), (-1, -0), (-1, 1), (1, -1), (1, 0), (1, 1), (0, 0)];
    const DOTS_FOR_VALUE: [[bool; 7]; 6] = [
        [N, N, N, N, N, N, Y],
        [N, N, Y, Y, N, N, N],
        [N, N, Y, Y, N, N, Y],
        [Y, N, Y, Y, N, Y, N],
        [Y, N, Y, Y, N, Y, Y],
        [Y, Y, Y, Y, Y, Y, N],
    ];

    let mut value = use_signal(|| 5);
    let active_dots = use_memo(move || &DOTS_FOR_VALUE[(value() - 1) as usize]);

    rsx! {
        svg {
            view_box: "-1000 -1000 2000 2000",
            prevent_default: "onclick",
            onclick: move |_| value.set(thread_rng().gen_range(1..=6)),
            rect { x: -1000, y: -1000, width: 2000, height: 2000, rx: 200, fill: "#aaa" }
            for ((x, y), _) in DOTS.iter().zip(active_dots.read().iter()).filter(|(_, &active)| active) {
                circle {
                    cx: *x * 600,
                    cy: *y * 600,
                    r: 200,
                    fill: "#333"
                }
            }
        }
    }
}
