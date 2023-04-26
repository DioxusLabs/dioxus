use std::collections::HashMap;

use crate::widgets::get_root_id;
use dioxus::prelude::*;
use dioxus_elements::input_data::keyboard_types::Key;
use dioxus_html as dioxus_elements;
use dioxus_html::FormData;
use rink::Query;

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

    let value_state = use_state(cx, || 0.0);
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

    let current_value = match value {
        Some(value) => value,
        None => *value_state.get(),
    }
    .clamp(min, max);

    let fst_width = 100.0 * (current_value - min) / size;
    let snd_width = 100.0 * (max - current_value) / size;
    assert!(fst_width + snd_width > 99.0 && fst_width + snd_width < 101.0);

    let update = |value: String| {
        if let Some(oninput) = cx.props.raw_oninput {
            oninput.call(FormData {
                value,
                values: HashMap::new(),
                files: None,
            });
        }
    };

    render! {
        div{
            width: "{width}",
            height: "{height}",
            display: "flex",
            flex_direction: "row",
            onkeydown: move |event| {
                match event.key() {
                    Key::ArrowLeft => {
                        value_state.set((current_value - step).clamp(min, max));
                        update(value_state.current().to_string());
                    }
                    Key::ArrowRight => {
                        value_state.set((current_value + step).clamp(min, max));
                        update(value_state.current().to_string());
                    }
                    _ => ()
                }
            },
            onmousemove: move |evt| {
                let mouse = evt.data;
                if !mouse.held_buttons().is_empty(){
                    let node = tui_query.get(get_root_id(cx).unwrap());
                    let width = node.size().unwrap().width;
                    let offset = mouse.element_coordinates();
                    value_state.set(min + size*(offset.x as f32) / width as f32);
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
    }
}
