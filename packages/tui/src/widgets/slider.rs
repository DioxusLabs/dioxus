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
}

#[allow(non_snake_case)]
pub(crate) fn Slider<'a>(cx: Scope<'a, SliderProps>) -> Element<'a> {
    let value_state = use_state(&cx, || 0.0);
    let value: Option<f32> = cx.props.value.and_then(|v| v.parse().ok());
    let state = use_state(&cx, || false);
    let width = cx.props.width.unwrap_or("1px");
    let height = cx.props.width.unwrap_or("1px");
    let min = cx.props.min.and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let max = cx.props.max.and_then(|v| v.parse().ok()).unwrap_or(100.0);
    let current_value = if let Some(value) = value {
        value
    } else {
        *value_state.get()
    }
    .max(min)
    .min(max);
    let size = max - min;
    let text = "-".repeat((current_value - min) as usize)
        + "|"
        + &"-".repeat((max - current_value) as usize);

    cx.render(rsx! {
        div{
            width: "{width}",
            height: "{height}",
            onkeydown: move |event| {
                match event.key_code {
                    KeyCode::LeftArrow => {
                        value_state.set((current_value - size / 10.0).max(min).min(max));
                    }
                    KeyCode::RightArrow => {
                        value_state.set((current_value + size / 10.0).max(min).min(max));
                    }
                    _ => ()
                }
            },
            "{text}"
        }
    })
}
