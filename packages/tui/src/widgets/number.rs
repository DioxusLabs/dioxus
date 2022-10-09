use crossterm::{execute, cursor::MoveTo};
use dioxus::prelude::*;
use dioxus_core::prelude::fc_to_builder;
use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_elements::KeyCode;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use dioxus_html::on::FormData;
use dioxus_native_core::utils::cursor::{Cursor, Pos};
use taffy::geometry::Point;
use std::{collections::HashMap, io::stdout};
use crate::Query;

use crate::widgets::Input;

#[derive(Props)]
pub(crate) struct NumbericInputProps<'a> {
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
pub(crate) fn NumbericInput<'a>(cx: Scope<'a, NumbericInputProps>) -> Element<'a> {
    let tui_query: Query = cx.consume_context().unwrap();
    let tui_query_clone = tui_query.clone();

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

    let update = |text: String| {
        if let Some(input_handler) = &cx.props.raw_oninput {
            input_handler.call(FormData {
                value: text,
                values: HashMap::new(),
            });
        }
    };
    let increase = move || {
        let mut text = text_ref.write();
        *text = (text.parse::<f64>().unwrap_or(0.0) + 1.0).to_string();
        update(text.clone());
    };
    let decrease = move || {
        let mut text = text_ref.write();
        *text = (text.parse::<f64>().unwrap_or(0.0) - 1.0).to_string();
        update(text.clone());
    };

    cx.render({
        rsx! {
            div{
                width: "{width}",
                height: "{height}",
                border_style: "{border}",

                onkeydown: move |k| {
                    if matches!(k.key_code, KeyCode::LeftArrow | KeyCode::RightArrow | KeyCode::Backspace | KeyCode::Period | KeyCode::Dash) || k.key.chars().all(|c| c.is_numeric()) {
                        let mut text = text_ref.write();
                        cursor.write().handle_input(&*k, &mut text, max_len);
                        update(text.clone());

                        let node = tui_query.get(cx.root_node().mounted_id());
                        let Point{ x, y } = node.pos().unwrap();
    
                        let Pos { col, row } = cursor.read().start;
                        let (x, y) = (col as u16 + x as u16 + if border == "none" {0} else {1}, row as u16 + y as u16 + if border == "none" {0} else {1});
                        if let Ok(pos) = crossterm::cursor::position() {
                            if pos != (x, y){
                                execute!(stdout(), MoveTo(x, y)).unwrap();
                            }
                        }
                        else{
                            execute!(stdout(), MoveTo(x, y)).unwrap();
                        }
                    }
                    else{
                        match k.key_code {
                            KeyCode::UpArrow =>{
                                increase();
                            }
                            KeyCode::DownArrow =>{
                                decrease();
                            }
                            _ => ()
                        }
                    }
                },
                onmousemove: move |evt| {
                    if *dragging.get() {
                        let mut new = Pos::new(evt.data.offset_x as usize, evt.data.offset_y as usize);
                        if border != "none" {
                            new.col = new.col.saturating_sub(1);
                        }
                        // textboxs are only one line tall
                        new.row = 0;

                        if new != cursor.read().start {
                            cursor.write().end = Some(new);
                        }
                    }
                },
                onmousedown: move |evt| {
                    let mut new = Pos::new(evt.data.offset_x as usize, evt.data.offset_y as usize);
                    if border != "none" {
                        new.col = new.col.saturating_sub(1);
                    }
                    new.row = 0;

                    new.realize_col(&text_ref.read());
                    cursor.set(Cursor::from_start(new));
                    dragging.set(true);
                    let node = tui_query_clone.get(cx.root_node().mounted_id());
                    let Point{ x, y } = node.pos().unwrap();
                    
                    let Pos { col, row } = cursor.read().start;
                    let (x, y) = (col as u16 + x as u16 + if border == "none" {0} else {1}, row as u16 + y as u16 + if border == "none" {0} else {1});
                    if let Ok(pos) = crossterm::cursor::position() {
                        if pos != (x, y){
                            execute!(stdout(), MoveTo(x, y)).unwrap();
                        }
                    }
                    else{
                        execute!(stdout(), MoveTo(x, y)).unwrap();
                    }
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
                onfocusout: |_| {
                    execute!(stdout(), MoveTo(0, 1000)).unwrap();
                },

                "{text_before_first_cursor}"

                span{
                    background_color: "rgba(255, 255, 255, 50%)",

                    "{text_highlighted}"
                }

                "{text_after_second_cursor}"

                div{
                    background_color: "rgba(255, 255, 255, 50%)",
                    color: "black",
                    Input{
                        r#type: "button",
                        onclick: move |_| {
                            decrease();
                        }
                        value: "<",
                    }
                    " "
                    Input{
                        r#type: "button",
                        onclick: move |_| {
                            increase();
                        }
                        value: ">",
                    }
                }
            }
        }
    })
}
