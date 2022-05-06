use std::collections::HashMap;

use dioxus_core as dioxus;
use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use dioxus_html::on::FormData;

#[derive(Props)]
pub(crate) struct CheckBoxInputProps<'a> {
    #[props(!optional)]
    raw_oninput: Option<&'a EventHandler<'a, FormData>>,
    #[props(!optional)]
    value: Option<&'a str>,
    #[props(!optional)]
    width: Option<&'a str>,
    #[props(!optional)]
    height: Option<&'a str>,
}

#[allow(non_snake_case)]
pub(crate) fn CheckBoxInput<'a>(cx: Scope<'a, CheckBoxInputProps>) -> Element<'a> {
    let state = use_state(&cx, || false);
    let width = cx.props.width.unwrap_or("1px");
    let height = cx.props.width.unwrap_or("1px");

    let single_char = width == "1px" && height == "1px";
    let text = if single_char {
        if *state.get() {
            "☑"
        } else {
            "☐"
        }
    } else {
        if *state.get() {
            "✓"
        } else {
            " "
        }
    };
    let border_style = if single_char { "none" } else { "solid" };
    cx.render(rsx! {
        div{
            width: "{width}",
            height: "{height}",
            border_style: "{border_style}",
            onmouseup: |_| {
                let new_state = !state.get();
                if let Some(callback) = cx.props.raw_oninput{
                    callback.call(FormData{
                        value: if let Some(value) = &cx.props.value {
                            if new_state {
                                value.to_string()
                            }
                            else {
                                String::new()
                            }
                        } else{
                            "on".to_string()
                        },
                        values: HashMap::new(),
                    });
                }
                state.set(new_state);
            },
            "{text}"
        }
    })
}
