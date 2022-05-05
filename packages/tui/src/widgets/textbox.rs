use dioxus_core as dioxus;
use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use dioxus_html::on::FormData;
use dioxus_native_core::utils::cursor::Cursor;
use std::collections::HashMap;

#[derive(Props)]
pub(crate) struct TextInputProps<'a> {
    #[props(!optional)]
    raw_oninput: Option<&'a EventHandler<'a, FormData>>,
    #[props(!optional)]
    value: Option<&'a str>,
    #[props(!optional)]
    size: Option<&'a str>,
    #[props(!optional)]
    width: Option<&'a str>,
    #[props(!optional)]
    height: Option<&'a str>,
}
#[allow(non_snake_case)]
pub(crate) fn TextInput<'a>(cx: Scope<'a, TextInputProps>) -> Element<'a> {
    let text_ref = use_ref(&cx, || {
        if let Some(intial_text) = cx.props.value {
            intial_text.to_string()
        } else {
            String::new()
        }
    });
    let cursor = use_ref(&cx, || Cursor::default());

    let text = text_ref.read().clone();
    let start_highlight = cursor.read().first().idx(&text);
    let end_highlight = cursor.read().last().idx(&text);
    let (text_before_first_cursor, text_after_first_cursor) = text.split_at(start_highlight);
    let (text_highlighted, text_after_second_cursor) =
        text_after_first_cursor.split_at(end_highlight - start_highlight);

    let text_highlighted = if text_highlighted.is_empty() {
        String::new()
    } else {
        text_highlighted.to_string() + "|"
    };

    let max_len = cx
        .props
        .size
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);

    let width = cx.props.width.unwrap_or("10px");
    let height = cx.props.height.unwrap_or("3px");

    // don't draw a border unless there is enough space
    let border = if width
        .strip_suffix("px")
        .and_then(|w| w.parse::<i32>().ok())
        .filter(|w| *w < 3)
        .is_some()
        || height
            .strip_suffix("px")
            .and_then(|h| h.parse::<i32>().ok())
            .filter(|h| *h < 3)
            .is_some()
    {
        "none"
    } else {
        "solid"
    };

    cx.render({
        rsx! {
            div{
                width: "{width}",
                height: "{height}",
                border_style: "{border}",
                align_items: "left",

                // prevent tabing out of the textbox
                prevent_default: "onkeydown",
                onkeydown: move |k| {
                    let mut text = text_ref.write();
                    cursor.write().handle_input(&*k, &mut text, max_len);
                    if let Some(input_handler) = &cx.props.raw_oninput{
                        input_handler.call(FormData{
                            value: text.clone(),
                            values: HashMap::new(),
                        });
                    }
                },

                "{text_before_first_cursor}|"

                span{
                    background_color: "rgba(100, 100, 100, 50%)",

                    "{text_highlighted}"
                }

                "{text_after_second_cursor}"
            }
        }
    })
}
