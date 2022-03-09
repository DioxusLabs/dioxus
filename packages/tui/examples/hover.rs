use std::{convert::TryInto, sync::Arc};

use dioxus::{events::MouseData, prelude::*};

fn main() {
    rink::launch(app);
}

fn app(cx: Scope) -> Element {
    fn to_str(c: &[i32; 3]) -> String {
        "#".to_string() + &c.iter().map(|c| format!("{c:02X?}")).collect::<String>()
    }

    fn get_brightness(m: Arc<MouseData>) -> i32 {
        let mb = m.buttons;
        let b: i32 = m.buttons.count_ones().try_into().unwrap();
        127 * b
    }

    let (q1_color, set_q1_color) = use_state(&cx, || [200; 3]);
    let (q2_color, set_q2_color) = use_state(&cx, || [200; 3]);
    let (q3_color, set_q3_color) = use_state(&cx, || [200; 3]);
    let (q4_color, set_q4_color) = use_state(&cx, || [200; 3]);

    let q1_color_str = to_str(q1_color);
    let q2_color_str = to_str(q2_color);
    let q3_color_str = to_str(q3_color);
    let q4_color_str = to_str(q4_color);

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
                    onmouseenter: move |m| set_q1_color([get_brightness(m.data), 0, 0]),
                    onmousedown: move |m| set_q1_color([get_brightness(m.data), 0, 0]),
                    onmouseup: move |m| set_q1_color([get_brightness(m.data), 0, 0]),
                    onwheel: move |w| set_q1_color([q1_color[0] + (10.0*w.delta_y) as i32, 0, 0]),
                    onmouseleave: move |_| set_q1_color([200; 3]),
                    "click me"
                }
                div {
                    width: "50%",
                    height: "100%",
                    justify_content: "center",
                    align_items: "center",
                    background_color: "{q2_color_str}",
                    onmouseenter: move |m| set_q2_color([get_brightness(m.data); 3]),
                    onmousedown: move |m| set_q2_color([get_brightness(m.data); 3]),
                    onmouseup: move |m| set_q2_color([get_brightness(m.data); 3]),
                    onwheel: move |w| set_q2_color([q2_color[0] + (10.0*w.delta_y) as i32;3]),
                    onmouseleave: move |_| set_q2_color([200; 3]),
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
                    onmouseenter: move |m| set_q3_color([0, get_brightness(m.data), 0]),
                    onmousedown: move |m| set_q3_color([0, get_brightness(m.data), 0]),
                    onmouseup: move |m| set_q3_color([0, get_brightness(m.data), 0]),
                    onwheel: move |w| set_q3_color([0, q3_color[1] + (10.0*w.delta_y) as i32, 0]),
                    onmouseleave: move |_| set_q3_color([200; 3]),
                    "click me"
                }
                div {
                    width: "50%",
                    height: "100%",
                    justify_content: "center",
                    align_items: "center",
                    background_color: "{q4_color_str}",
                    onmouseenter: move |m| set_q4_color([0, 0, get_brightness(m.data)]),
                    onmousedown: move |m| set_q4_color([0, 0, get_brightness(m.data)]),
                    onmouseup: move |m| set_q4_color([0, 0, get_brightness(m.data)]),
                    onwheel: move |w| set_q4_color([0, 0, q4_color[2] + (10.0*w.delta_y) as i32]),
                    onmouseleave: move |_| set_q4_color([200; 3]),
                    "click me"
                }
            }
        }
    })
}
