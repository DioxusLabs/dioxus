use dioxus::events::WheelEvent;
use dioxus::prelude::*;
use dioxus_html::on::{KeyboardEvent, MouseEvent};
use dioxus_html::KeyCode;

fn main() {
    rink::launch(app);
}

fn app(cx: Scope) -> Element {
    let (key, set_key) = use_state(&cx, || KeyCode::Space);
    let (mouse, set_mouse) = use_state(&cx, || (0, 0));
    let (count, set_count) = use_state(&cx, || 0);
    let (buttons, set_buttons) = use_state(&cx, || 0);
    let (mouse_clicked, set_mouse_clicked) = use_state(&cx, || false);

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            flex_direction: "column",
            onkeydown: move |evt: KeyboardEvent| {
                match evt.data.key_code {
                    KeyCode::LeftArrow => set_count(count + 1),
                    KeyCode::RightArrow => set_count(count - 1),
                    KeyCode::UpArrow => set_count(count + 10),
                    KeyCode::DownArrow => set_count(count - 10),
                    _ => {},
                }
                set_key(evt.key_code);
            },
            onwheel: move |evt: WheelEvent| {
                set_count(count + evt.data.delta_y as i64);
            },
            ondrag: move |evt: MouseEvent| {
                set_mouse((evt.data.screen_x, evt.data.screen_y));
            },
            onmousedown: move |evt: MouseEvent| {
                set_mouse((evt.data.screen_x, evt.data.screen_y));
                set_buttons(evt.data.buttons);
                set_mouse_clicked(true);
            },
            onmouseup: move |evt: MouseEvent| {
                set_buttons(evt.data.buttons);
                set_mouse_clicked(false);
            },

            "count: {count:?}",
            "key: {key:?}",
            "mouse buttons: {buttons:b}",
            "mouse pos: {mouse:?}",
            "mouse button pressed: {mouse_clicked}"
        }
    })
}

fn app2<'a>(cx: Scope<'a>) -> Element<'a> {
    let (count, set_count) = use_state(&cx, || 0);

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            oninput: move |_| set_count(count + 1),
            "Hello world!",
            h1 {},
        }
    })
}
