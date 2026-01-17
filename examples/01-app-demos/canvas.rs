//! Canvas
//!
//! This example demonstrates how to use web apis in dioxus desktop. Dioxus integrates with web-sys-x to
//! provide access to a web-sys compatible api through the webview. This lets you call into the webview
//! to access the dom while still running your code natively

use dioxus::prelude::*;
use std::f64;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_x::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        Canvas {}
    }
}

#[component]
fn Canvas() -> Element {
    use_effect(draw_canvas);
    rsx! {
        canvas {
            id: "canvas",
            height: "250",
            width: "250",
            border: "solid",
            touch_action: "none"
        }
    }
}

fn draw_canvas() {
    let document = web_sys_x::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys_x::HtmlCanvasElement = canvas
        .dyn_into::<web_sys_x::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys_x::CanvasRenderingContext2d>()
        .unwrap();
    let context = Rc::new(context);
    let pressed = Rc::new(Cell::new(false));
    {
        let context = context.clone();
        let pressed = pressed.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys_x::PointerEvent| {
            context.begin_path();
            context.move_to(event.offset_x() as f64, event.offset_y() as f64);
            pressed.set(true);
        });
        canvas
            .add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    {
        let context = context.clone();
        let pressed = pressed.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys_x::PointerEvent| {
            if pressed.get() {
                context.line_to(event.offset_x() as f64, event.offset_y() as f64);
                context.stroke();
                context.begin_path();
                context.move_to(event.offset_x() as f64, event.offset_y() as f64);
            }
        });
        canvas
            .add_event_listener_with_callback("pointermove", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    {
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys_x::PointerEvent| {
            pressed.set(false);
            context.line_to(event.offset_x() as f64, event.offset_y() as f64);
            context.stroke();
        });
        canvas
            .add_event_listener_with_callback("pointerup", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
