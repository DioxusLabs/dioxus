//! Multiwindow example
//!
//! This example shows how to implement a simple multiwindow application using dioxus.
//! This works by spawning a new window when the user clicks a button. We have to build a new virtualdom which has its
//! own context, root elements, etc.

use dioxus::{desktop::DesktopServiceProxy, prelude::*};
use std::f64;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let proxy = use_context::<DesktopServiceProxy>();
    let onclick = move |_| {
        println!("clicked");
        proxy.new_window(|| VirtualDom::new(popup), Default::default);
    };

    rsx! {
        button { onclick, "New Window" }
        Canvas {}
    }
}

fn popup() -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        div {
            h1 { "Popup Window" }
            p { "Count: {count}" }
            button { onclick: move |_| count += 1, "Increment" }
            Canvas {}
        }
    }
}

#[component]
fn Canvas() -> Element {
    use_effect(draw_canvas);
    rsx! {
        canvas {
            id: "canvas",
            height: "500",
            width: "500",
            border: "solid"
        }
    }
}

fn draw_canvas() {
    let proxy: DesktopServiceProxy = consume_context();
    proxy.devtool();
    println!("Running effect");
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    let context = Rc::new(context);
    let pressed = Rc::new(Cell::new(false));
    {
        let context = context.clone();
        let pressed = pressed.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            context.begin_path();
            context.move_to(event.offset_x() as f64, event.offset_y() as f64);
            pressed.set(true);
        });
        canvas
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    {
        let context = context.clone();
        let pressed = pressed.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            if pressed.get() {
                context.line_to(event.offset_x() as f64, event.offset_y() as f64);
                context.stroke();
                context.begin_path();
                context.move_to(event.offset_x() as f64, event.offset_y() as f64);
            }
        });
        canvas
            .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    {
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            pressed.set(false);
            context.line_to(event.offset_x() as f64, event.offset_y() as f64);
            context.stroke();
        });
        canvas
            .add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
