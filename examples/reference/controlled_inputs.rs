use dioxus::prelude::*;
fn main() {}

pub static Example: FC<()> = |cx, props| {
    cx.render(rsx! {
        div {

        }
    })
};

// A controlled component:
static ControlledSelect: FC<()> = |cx, props| {
    let value = use_state(cx, || String::from("Grapefruit"));
    cx.render(rsx! {
        select { value: "{value}", onchange: move |evt| value.set(evt.value()),
            option { value: "Grapefruit", "Grapefruit"}
            option { value: "Lime", "Lime"}
            option { value: "Coconut", "Coconut"}
            option { value: "Mango", "Mango"}
        }
    })
};

// TODO - how do uncontrolled things work?
static UncontrolledSelect: FC<()> = |cx, props| {
    let value = use_state(cx, || String::new());

    cx.render(rsx! {
        select {
            option { value: "Grapefruit", "Grapefruit"}
            option { value: "Lime", "Lime"}
            option { value: "Coconut", "Coconut"}
            option { value: "Mango", "Mango"}
        }
    })
};
