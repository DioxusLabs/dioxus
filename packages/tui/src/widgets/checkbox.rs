use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_elements::KeyCode;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use dioxus_html::on::FormData;

#[derive(Props)]
pub(crate) struct CheckBoxProps<'a> {
    #[props(!optional)]
    raw_oninput: Option<&'a EventHandler<'a, FormData>>,
    #[props(!optional)]
    value: Option<&'a str>,
    #[props(!optional)]
    width: Option<&'a str>,
    #[props(!optional)]
    height: Option<&'a str>,
    #[props(!optional)]
    checked: Option<&'a str>,
}

#[allow(non_snake_case)]
pub(crate) fn CheckBox<'a>(cx: Scope<'a, CheckBoxProps>) -> Element<'a> {
    let state = use_state(&cx, || cx.props.checked.filter(|&c| c == "true").is_some());
    let width = cx.props.width.unwrap_or("1px");
    let height = cx.props.height.unwrap_or("1px");

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
    let border_style = if width == "1px" || height == "1px" {
        "none"
    } else {
        "solid"
    };
    let update = move || {
        let new_state = !state.get();
        if let Some(callback) = cx.props.raw_oninput {
            callback.call(FormData {
                value: if let Some(value) = &cx.props.value {
                    if new_state {
                        value.to_string()
                    } else {
                        String::new()
                    }
                } else {
                    "on".to_string()
                },
                values: HashMap::new(),
            });
        }
        state.set(new_state);
    };
    cx.render(rsx! {
        div{
            width: "{width}",
            height: "{height}",
            border_style: "{border_style}",
            onclick: move |_| {
                update();
            },
            onkeydown: move |evt| {
                if !evt.repeat && matches!(evt.key_code, KeyCode::Space | KeyCode::Enter) {
                    update();
                }
            },
            "{text}"
        }
    })
}
