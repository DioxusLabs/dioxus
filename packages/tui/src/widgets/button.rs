use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_elements::input_data::keyboard_types::Key;
use dioxus_html as dioxus_elements;
use dioxus_html::FormData;

#[derive(Props)]
pub(crate) struct ButtonProps<'a> {
    #[props(!optional)]
    raw_onclick: Option<&'a EventHandler<'a, FormData>>,
    #[props(!optional)]
    value: Option<&'a str>,
    #[props(!optional)]
    width: Option<&'a str>,
    #[props(!optional)]
    height: Option<&'a str>,
}

#[allow(non_snake_case)]
pub(crate) fn Button<'a>(cx: Scope<'a, ButtonProps>) -> Element<'a> {
    let state = use_state(cx, || false);
    let width = cx.props.width.unwrap_or("1px");
    let height = cx.props.height.unwrap_or("1px");

    let single_char = width == "1px" || height == "1px";
    let text = if let Some(v) = cx.props.value { v } else { "" };
    let border_style = if single_char { "none" } else { "solid" };
    let update = || {
        let new_state = !state.get();
        if let Some(callback) = cx.props.raw_onclick {
            callback.call(FormData {
                value: text.to_string(),
                values: HashMap::new(),
                files: None,
            });
        }
        state.set(new_state);
    };
    render! {
        div{
            width: "{width}",
            height: "{height}",
            border_style: "{border_style}",
            flex_direction: "row",
            align_items: "center",
            justify_content: "center",
            onclick: move |_| {
                update();
            },
            onkeydown: move |evt|{
                if !evt.is_auto_repeating() && match evt.key(){ Key::Character(c) if c == " " =>true, Key::Enter=>true, _=>false }  {
                    update();
                }
            },
            "{text}"
        }
    }
}
