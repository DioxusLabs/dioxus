use dioxus::prelude::*;
use dioxus_core::UiEvent;
use dioxus_html::on::MouseData;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let page_coordinates = use_state(&cx, || "".to_string());
    let screen_coordinates = use_state(&cx, || "".to_string());
    let element_coordinates = use_state(&cx, || "".to_string());

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

    let update_mouse_position = move |event: UiEvent<MouseData>| {
        let mouse_data = event.data;

        page_coordinates.set(format!("{:?}", mouse_data.page_coordinates()));
        screen_coordinates.set(format!("{:?}", mouse_data.screen_coordinates()));
        element_coordinates.set(format!("{:?}", mouse_data.element_coordinates()));

        // Note: client coordinates are also available, but they would be the same as the page coordinates in this example, because there is no scrolling.
    };

    cx.render(rsx! (
        div {
            style: "{container_style}",
            "Hover over to display coordinates:",
            div {
                style: "{rect_style}",
                onmousemove: update_mouse_position,
            }
            div {"Page coordinates: {page_coordinates}"},
            div {"Screen coordinates: {screen_coordinates}"},
            div {"Element coordinates: {element_coordinates}"},
        }
    ))
}
