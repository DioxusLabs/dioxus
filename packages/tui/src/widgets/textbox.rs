use dioxus_core as dioxus;
use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_elements::KeyCode;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use dioxus_html::on::FormData;
use dioxus_native_core::utils::cursor::{Cursor, Pos};
use std::collections::HashMap;

#[derive(Props)]
pub(crate) struct TextBoxProps<'a> {
    #[props(!optional)]
    raw_oninput: Option<&'a EventHandler<'a, FormData>>,
    #[props(!optional)]
    value: Option<&'a str>,
    #[props(!optional)]
    size: Option<&'a str>,
    #[props(!optional)]
    max_length: Option<&'a str>,
    #[props(!optional)]
    width: Option<&'a str>,
    #[props(!optional)]
    height: Option<&'a str>,
}
#[allow(non_snake_case)]
pub(crate) fn TextBox<'a>(cx: Scope<'a, TextBoxProps>) -> Element<'a> {
    let text_ref = use_ref(&cx, || {
        if let Some(intial_text) = cx.props.value {
            intial_text.to_string()
        } else {
            String::new()
        }
    });
    let cursor = use_ref(&cx, || Cursor::default());
    let dragging = use_state(&cx, || false);

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
        .max_length
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);

    let width = cx
        .props
        .width
        .map(|s| s.to_string())
        // px is the same as em in tui
        .or(cx.props.size.map(|s| s.to_string() + "px"))
        .unwrap_or("10px".to_string());
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

                onkeydown: move |k| {
                    if k.key_code == KeyCode::Enter {
                        return;
                    }
                    let mut text = text_ref.write();
                    cursor.write().handle_input(&*k, &mut text, max_len);
                    if let Some(input_handler) = &cx.props.raw_oninput{
                        input_handler.call(FormData{
                            value: text.clone(),
                            values: HashMap::new(),
                        });
                    }
                },

                onmousemove: move |evt| {
                    if *dragging.get() {
                        let mut new = Pos::new(evt.data.offset_x as usize, evt.data.offset_y as usize);
                        if border != "none" {
                            new.col -= 1;
                            new.row -= 1;
                        }
                        let mut cursor = cursor.write();
                        if new != cursor.start {
                            cursor.end = Some(new);
                        }
                    }
                },
                onmousedown: move |evt| {
                    let mut new = Pos::new(evt.data.offset_x as usize, evt.data.offset_y as usize);
                    if border != "none" {
                        new.col -= 1;
                        new.row -= 1;
                    }
                    cursor.set(Cursor::from_start(new));
                    dragging.set(true);
                },
                onmouseup: move |_| {
                    dragging.set(false);
                },
                onmouseleave: move |_| {
                    dragging.set(false);
                },
                onmouseenter: move |_| {
                    dragging.set(false);
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
