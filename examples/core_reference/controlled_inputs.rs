use dioxus::prelude::*;
fn main() {}

pub fn Example(cx: Scope) -> Element {
    cx.render(rsx! {
        div {

        }
    })
}

// A controlled component:
pub fn ControlledSelect(cx: Scope) -> Element {
    let value = use_state(&cx, || String::from("Grapefruit"));
    cx.render(rsx! {
        select { value: "{value}", onchange: move |evt| value.set(evt.value()),
            option { value: "Grapefruit", "Grapefruit"}
            option { value: "Lime", "Lime"}
            option { value: "Coconut", "Coconut"}
            option { value: "Mango", "Mango"}
        }
    })
}

// TODO - how do uncontrolled things work?
pub fn UncontrolledSelect(cx: Scope) -> Element {
    let value = use_state(&cx, || String::new());

    cx.render(rsx! {
        select {
            option { value: "Grapefruit", "Grapefruit"}
            option { value: "Lime", "Lime"}
            option { value: "Coconut", "Coconut"}
            option { value: "Mango", "Mango"}
        }
    })
}
