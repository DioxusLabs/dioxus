use dioxus_core as dioxus;
use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use dioxus_html::on::FormData;
use dioxus_native_core::utils::cursor::Cursor;
use std::collections::HashMap;
use std::fmt::Arguments;

#[derive(Props)]
pub(crate) struct TextInputProps<'a> {
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
        .and_then(|s| s.to_string().parse().ok())
        .unwrap_or(usize::MAX);

    cx.render({
        rsx! {
            div{
                border_style: "solid",
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

                cx.render(rsx! {span{
                    margin: "0px",
                    padding: "0px",
                    background_color: "rgba(100, 100, 100, 50%)",

                    "{text_highlighted}"
                }})

                "{text_after_second_cursor}"
            }
        }
    })
}
