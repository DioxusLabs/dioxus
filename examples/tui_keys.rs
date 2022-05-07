use dioxus::events::WheelEvent;
use dioxus::prelude::*;
use dioxus_html::geometry::ScreenPoint;
use dioxus_html::input::MouseButtonSet;
use dioxus_html::on::{KeyboardEvent, MouseEvent};
use dioxus_html::KeyCode;

fn main() {
    dioxus::tui::launch(app);
}

fn app(cx: Scope) -> Element {
    let key = use_state(&cx, || "".to_string());
    let mouse = use_state(&cx, || ScreenPoint::zero());
    let count = use_state(&cx, || 0);
    let buttons = use_state(&cx, || MouseButtonSet::empty());
    let mouse_clicked = use_state(&cx, || false);

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
                    KeyCode::LeftArrow => count.set(count + 1),
                    KeyCode::RightArrow => count.set(count - 1),
                    KeyCode::UpArrow => count.set(count + 10),
                    KeyCode::DownArrow => count.set(count - 10),
                    _ => {},
                }
                key.set(format!("{:?} repeating: {:?}", evt.key, evt.repeat));
            },
            onwheel: move |evt: WheelEvent| {
                count.set(count + evt.data.delta_y as i64);
            },
            ondrag: move |evt: MouseEvent| {
                mouse.set(evt.data.screen_coordinates());
            },
            onmousedown: move |evt: MouseEvent| {
                mouse.set(evt.data.screen_coordinates());
                buttons.set(evt.data.held_buttons());
                mouse_clicked.set(true);
            },
            onmouseup: move |evt: MouseEvent| {
                buttons.set(evt.data.held_buttons());
                mouse_clicked.set(false);
            },

            "count: {count:?}",
            "key: {key}",
            "mouse buttons: {buttons:?}",
            "mouse pos: {mouse:?}",
            "mouse button pressed: {mouse_clicked}"
        }
    })
}
