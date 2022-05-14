use std::{convert::TryInto, sync::Arc};

use dioxus::{events::MouseData, prelude::*};
use dioxus_core::UiEvent;

fn main() {
    dioxus::tui::launch(app);
}

fn app(cx: Scope) -> Element {
    fn to_str(c: &[i32; 3]) -> String {
        "#".to_string() + &c.iter().map(|c| format!("{c:02X?}")).collect::<String>()
    }

    fn get_brightness(m: Arc<MouseData>) -> i32 {
        let b: i32 = m.held_buttons().len().try_into().unwrap();
        127 * b
    }

    let q1_color = use_state(&cx, || [200; 3]);
    let q2_color = use_state(&cx, || [200; 3]);
    let q3_color = use_state(&cx, || [200; 3]);
    let q4_color = use_state(&cx, || [200; 3]);

    let q1_color_str = to_str(q1_color);
    let q2_color_str = to_str(q2_color);
    let q3_color_str = to_str(q3_color);
    let q4_color_str = to_str(q4_color);

    let page_coordinates = use_state(&cx, || "".to_string());
    let element_coordinates = use_state(&cx, || "".to_string());
    let buttons = use_state(&cx, || "".to_string());
    let modifiers = use_state(&cx, || "".to_string());

    let update_data = move |event: UiEvent<MouseData>| {
        let mouse_data = event.data;

        page_coordinates.set(format!("{:?}", mouse_data.page_coordinates()));
        element_coordinates.set(format!("{:?}", mouse_data.element_coordinates()));

        // Note: client coordinates are also available, but they would be the same as the page coordinates in this example, because there is no scrolling.
        // There are also screen coordinates, but they are currently the same as client coordinates due to technical limitations

        buttons.set(format!("{:?}", mouse_data.held_buttons()));
        modifiers.set(format!("{:?}", mouse_data.modifiers()));
    };

    cx.render(rsx! {
        div {
            width: "100%",
            height: "100%",
            flex_direction: "column",

            div {
                width: "100%",
                height: "50%",
                flex_direction: "row",
                div {
                    border_width: "1px",
                    width: "50%",
                    height: "100%",
                    justify_content: "center",
                    align_items: "center",
                    background_color: "{q1_color_str}",
                    onmouseenter: move |m| q1_color.set([get_brightness(m.data), 0, 0]),
                    onmousedown: move |m| q1_color.set([get_brightness(m.data), 0, 0]),
                    onmouseup: move |m| q1_color.set([get_brightness(m.data), 0, 0]),
                    onwheel: move |w| q1_color.set([q1_color[0] + (10.0*w.delta_y) as i32, 0, 0]),
                    onmouseleave: move |_| q1_color.set([200; 3]),
                    onmousemove: update_data,
                    "click me"
                }
                div {
                    width: "50%",
                    height: "100%",
                    justify_content: "center",
                    align_items: "center",
                    background_color: "{q2_color_str}",
                    onmouseenter: move |m| q2_color.set([get_brightness(m.data); 3]),
                    onmousedown: move |m| q2_color.set([get_brightness(m.data); 3]),
                    onmouseup: move |m| q2_color.set([get_brightness(m.data); 3]),
                    onwheel: move |w| q2_color.set([q2_color[0] + (10.0*w.delta_y) as i32;3]),
                    onmouseleave: move |_| q2_color.set([200; 3]),
                    onmousemove: update_data,
                    "click me"
                }
            }

            div {
                width: "100%",
                height: "50%",
                flex_direction: "row",
                div {
                    width: "50%",
                    height: "100%",
                    justify_content: "center",
                    align_items: "center",
                    background_color: "{q3_color_str}",
                    onmouseenter: move |m| q3_color.set([0, get_brightness(m.data), 0]),
                    onmousedown: move |m| q3_color.set([0, get_brightness(m.data), 0]),
                    onmouseup: move |m| q3_color.set([0, get_brightness(m.data), 0]),
                    onwheel: move |w| q3_color.set([0, q3_color[1] + (10.0*w.delta_y) as i32, 0]),
                    onmouseleave: move |_| q3_color.set([200; 3]),
                    onmousemove: update_data,
                    "click me"
                }
                div {
                    width: "50%",
                    height: "100%",
                    justify_content: "center",
                    align_items: "center",
                    background_color: "{q4_color_str}",
                    onmouseenter: move |m| q4_color.set([0, 0, get_brightness(m.data)]),
                    onmousedown: move |m| q4_color.set([0, 0, get_brightness(m.data)]),
                    onmouseup: move |m| q4_color.set([0, 0, get_brightness(m.data)]),
                    onwheel: move |w| q4_color.set([0, 0, q4_color[2] + (10.0*w.delta_y) as i32]),
                    onmouseleave: move |_| q4_color.set([200; 3]),
                    onmousemove: update_data,
                    "click me"
                }
            },
            div {"Page coordinates: {page_coordinates}"},
            div {"Element coordinates: {element_coordinates}"},
            div {"Buttons: {buttons}"},
            div {"Modifiers: {modifiers}"},
        }
    })
}
