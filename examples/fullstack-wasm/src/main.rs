#[cfg(feature = "web")]
use dioxus::prelude::*;

fn main() {
    dioxus::launch(fullstack_wasm::app);
}
