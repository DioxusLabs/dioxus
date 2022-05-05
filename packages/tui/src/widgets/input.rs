use dioxus_core as dioxus;
use dioxus_core::prelude::fc_to_builder;
use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_html::on::FormData;

use crate::widgets::checkbox::CheckBoxInput;
use crate::widgets::textbox::TextInput;

#[derive(Props)]
pub struct InputProps<'a> {
    r#type: Option<&'static str>,
    oninput: Option<EventHandler<'a, FormData>>,
    value: Option<&'a str>,
    size: Option<&'a str>,
    width: Option<&'a str>,
    height: Option<&'a str>,
}

#[allow(non_snake_case)]
pub fn Input<'a>(cx: Scope<'a, InputProps<'a>>) -> Element<'a> {
    cx.render(match cx.props.r#type {
        Some("checkbox") => {
            rsx! {
                CheckBoxInput{
                    raw_oninput: cx.props.oninput.as_ref(),
                    value: cx.props.value,
                    size: cx.props.size,
                    width: cx.props.width,
                    height: cx.props.height,
                }
            }
        }
        _ => {
            rsx! {
                TextInput{
                    raw_oninput: cx.props.oninput.as_ref(),
                    value: cx.props.value,
                    size: cx.props.size,
                    width: cx.props.width,
                    height: cx.props.height,
                }
            }
        }
    })
}
