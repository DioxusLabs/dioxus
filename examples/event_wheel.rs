use dioxus::prelude::*;
use dioxus_core::UiEvent;
use dioxus_html::on::WheelData;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let delta = use_state(&cx, || "".to_string());

    let container_style = r#"
        display: flex;
        flex-direction: column;
        align-items: center;
    "#;
    let rect_style = r#"
        background: deepskyblue;
        height: 50vh;
        width: 50vw;
    "#;

    let handle_event = move |event: UiEvent<WheelData>| {
        let wheel_data = event.data;
        delta.set(format!("{:?}", wheel_data.delta()));
    };

    cx.render(rsx! (
        div {
            style: "{container_style}",
            "Scroll mouse wheel over rectangle:",
            div {
                style: "{rect_style}",
                onwheel: handle_event,
            }
            div {"Delta: {delta}"},
        }
    ))
}
