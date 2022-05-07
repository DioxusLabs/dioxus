use std::collections::HashMap;

use crate::Query;
use dioxus_core as dioxus;
use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_elements::KeyCode;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use dioxus_html::on::FormData;

#[derive(Props)]
pub(crate) struct SliderProps<'a> {
    #[props(!optional)]
    raw_oninput: Option<&'a EventHandler<'a, FormData>>,
    #[props(!optional)]
    value: Option<&'a str>,
    #[props(!optional)]
    width: Option<&'a str>,
    #[props(!optional)]
    height: Option<&'a str>,
    #[props(!optional)]
    min: Option<&'a str>,
    #[props(!optional)]
    max: Option<&'a str>,
    #[props(!optional)]
    step: Option<&'a str>,
}

#[allow(non_snake_case)]
pub(crate) fn Slider<'a>(cx: Scope<'a, SliderProps>) -> Element<'a> {
    let tui_query: Query = cx.consume_context().unwrap();

    let value_state = use_state(&cx, || 0.0);
    let value: Option<f32> = cx.props.value.and_then(|v| v.parse().ok());
    let width = cx.props.width.unwrap_or("20px");
    let height = cx.props.height.unwrap_or("1px");
    let min = cx.props.min.and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let max = cx.props.max.and_then(|v| v.parse().ok()).unwrap_or(100.0);
    let size = max - min;
    let step = cx
        .props
        .step
        .and_then(|v| v.parse().ok())
        .unwrap_or(size / 10.0);

    let current_value = if let Some(value) = value {
        value
    } else {
        *value_state.get()
    }
    .max(min)
    .min(max);
    let fst_width = 100.0 * (current_value - min) / size;
    let snd_width = 100.0 * (max - current_value) / size;
    assert!(fst_width + snd_width > 99.0 && fst_width + snd_width < 101.0);

    let update = |value: String| {
        if let Some(oninput) = cx.props.raw_oninput {
            oninput.call(FormData {
                value: value,
                values: HashMap::new(),
            });
        }
    };

    cx.render(rsx! {
        div{
            width: "{width}",
            height: "{height}",
            display: "flex",
            flex_direction: "row",
            onkeydown: move |event| {
                match event.key_code {
                    KeyCode::LeftArrow => {
                        value_state.set((current_value - step).max(min).min(max));
                        update(value_state.current().to_string());
                    }
                    KeyCode::RightArrow => {
                        value_state.set((current_value + step).max(min).min(max));
                        update(value_state.current().to_string());
                    }
                    _ => ()
                }
            },
            onmousemove: move |evt| {
                let mouse = evt.data;
                if mouse.buttons != 0{
                    let node = tui_query.get(cx.root_node().mounted_id());
                    let width = node.size().unwrap().width;
                    value_state.set(min + size*(mouse.offset_x as f32) / width as f32);
                    update(value_state.current().to_string());
                }
            },
            div{
                width: "{fst_width}%",
                background_color: "rgba(10,10,10,0.5)",
            }
            div{
                "|"
            }
            div{
                width: "{snd_width}%",
                background_color: "rgba(10,10,10,0.5)",
            }
        }
    })
}
