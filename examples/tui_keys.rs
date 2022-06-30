use dioxus::events::WheelEvent;
use dioxus::prelude::*;
use dioxus_html::geometry::ScreenPoint;
use dioxus_html::input_data::keyboard_types::Code;
use dioxus_html::input_data::MouseButtonSet;
use dioxus_html::on::{KeyboardEvent, MouseEvent};

fn main() {
    dioxus::tui::launch(app);
}

fn app(cx: Scope) -> Element {
    let key = use_state(&cx, || "".to_string());
    let mouse = use_state(&cx, ScreenPoint::zero);
    let count = use_state(&cx, || 0);
    let buttons = use_state(&cx, MouseButtonSet::empty);
    let mouse_clicked = use_state(&cx, || false);

    let key_down_handler = move |evt: KeyboardEvent| {
        match evt.data.code() {
            Code::ArrowLeft => count.set(count + 1),
            Code::ArrowRight => count.set(count - 1),
            Code::ArrowUp => count.set(count + 10),
            Code::ArrowDown => count.set(count - 10),
            _ => {}
        }
        key.set(format!(
            "{:?} repeating: {:?}",
            evt.key(),
            evt.is_auto_repeating()
        ));
    };

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            flex_direction: "column",
            onkeydown: key_down_handler,
            onwheel: move |evt: WheelEvent| {
                count.set(count + evt.data.delta().strip_units().y as i64);
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
