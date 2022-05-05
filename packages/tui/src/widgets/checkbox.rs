use std::collections::HashMap;

use core::fmt::Arguments;
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
    value: Option<Arguments<'a>>,
    #[props(!optional)]
    size: Option<Arguments<'a>>,
    #[props(!optional)]
    width: Option<Arguments<'a>>,
    #[props(!optional)]
    height: Option<Arguments<'a>>,
}

#[allow(non_snake_case)]
pub(crate) fn CheckBoxInput<'a>(cx: Scope<'a, CheckBoxInputProps>) -> Element<'a> {
    let state = use_state(&cx, || false);

    let text = if *state.get() { "■" } else { "□" };
    cx.render(rsx! {
        div{
            onmouseup: |_| {
                let new_state = !state.get();
                if let Some(callback) = cx.props.raw_oninput{
                    callback.call(FormData{
                        value: if let Some(value) = cx.props.value {
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
