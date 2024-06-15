#![allow(non_snake_case)]

use dioxus::{
    desktop::{
        tao::{dpi::PhysicalSize, window::ResizeDirection},
        use_window, use_wry_event_handler, window, WindowBuilder, WindowEvent,
    },
    html::input_data::MouseButton,
    prelude::*,
};

use std::process::exit;
use std::time::Duration;

use serde::{Deserialize, Serialize};

fn main() {
    let cfg = dioxus::desktop::Config::new()
        .with_custom_index(
            r#"
<!DOCTYPE html>
<html>
  <head>
    <title>Custom decorations</title>
  </head>
  <body>
    <div id="main" class="w-screen h-screen"></div>
  </body>
</html>
        "#
            .into(),
        )
        .with_window(
            WindowBuilder::new()
                .with_always_on_top(cfg!(debug_assertions))
                .with_decorations(false)
                .with_min_inner_size(PhysicalSize::new(800, 600)),
        )
        .with_menu(None);
    LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // the styling is precompiled tailwind, you cant really change the styles by adding more tailwind
        //  classes in here sadly, but you can add your own .css file or use inline styles to change the styling.
        style { {include_str!("./assets/custom_decorations.css")} }
        // adds a resize area, all components from here need to be given as children to this main component.
        WithDecorations {}
    }
}

#[component]
fn ResizingArea(resizable: bool) -> Element {
    #[cfg(debug_assertions)]
    let mut is_debug = use_signal(|| window().webview.is_devtools_open());
    #[cfg(not(debug_assertions))]
    let mut is_debug = use_signal(|| false);
    #[cfg(debug_assertions)]
    let _ = use_coroutine(move |_: UnboundedReceiver<()>| async move {
        loop {
            is_debug.set(window().webview.is_devtools_open());
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    if resizable {
        rsx! {
            div {
                class: "resizing_area z-50",
                style: "pointer-events: none;",
                onmousedown: move |e| e.stop_propagation(),
                div {
                    onmousedown: move |e| {
                        window().drag_resize_window(ResizeDirection::NorthWest).unwrap();
                        e.stop_propagation();
                    },
                    class: "cursor-nw-resize nw z-50",
                    style: "pointer-events: auto;"
                }
                div {
                    onmousedown: move |e| {
                        window().drag_resize_window(ResizeDirection::North).unwrap();
                        e.stop_propagation();
                    },
                    style: "pointer-events: auto;",
                    class: "cursor-n-resize n z-50"
                }
                div {
                    onmousedown: move |e| {
                        window().drag_resize_window(ResizeDirection::NorthEast).unwrap();
                        e.stop_propagation();
                    },
                    style: "pointer-events: auto;",
                    class: "cursor-ne-resize ne z-50"
                }
                div {
                    onmousedown: move |e| {
                        window().drag_resize_window(ResizeDirection::West).unwrap();
                        e.stop_propagation();
                    },
                    style: "pointer-events: auto;",
                    class: "cursor-w-resize w z-50"
                }
                div {
                    onmousedown: move |e| {
                        window().drag_resize_window(ResizeDirection::East).unwrap();
                        e.stop_propagation();
                    },
                    style: "pointer-events: auto;",
                    class: "cursor-e-resize e z-50"
                }
                if !is_debug() {
                    div {
                        onmousedown: move |e| {
                            window().drag_resize_window(ResizeDirection::SouthWest).unwrap();
                            e.stop_propagation();
                        },
                        style: "pointer-events: auto;",
                        class: "cursor-sw-resize sw z-50"
                    }
                } else {
                    div { class: "sw z-50" }
                }
                if !is_debug() {
                    div {
                        onmousedown: move |e| {
                            window().drag_resize_window(ResizeDirection::South).unwrap();
                            e.stop_propagation();
                        },
                        style: "pointer-events: auto;",
                        class: "cursor-s-resize s z-50"
                    }
                } else {
                    div { class: "s z-50" }
                }
                if !is_debug() {
                    div {
                        onmousedown: move |e| {
                            window().drag_resize_window(ResizeDirection::SouthEast).unwrap();
                            e.stop_propagation();
                        },
                        style: "pointer-events: auto;",
                        class: "cursor-se-resize se z-50"
                    }
                } else {
                    div { class: "se z-50" }
                }
            }
        }
    } else {
        None
    }
}

#[component]
pub fn WithDecorations(children: Element) -> Element {
    let mut fullscreen = use_signal(|| window().fullscreen().is_some());
    let mut moving = use_signal(|| false);
    let mut maximized = use_signal(|| window().is_maximized());
    let mut events = use_hook(|| {
        eval(
            r#"
document.onkeydown = (keyDownEvent) => {
    if (keyDownEvent.key === "F11") {
        dioxus.send("F11");
        console.log("f11");
    }
}"#,
        )
    });

    // used for making the maximized value be up to date
    {
        let desktop_window = use_window();
        use_wry_event_handler(move |e, _| {
            use dioxus::desktop::tao::event::Event;
            if let Event::WindowEvent {
                event: WindowEvent::Resized(_),
                window_id,
                ..
            } = e
            {
                if window_id != &desktop_window.window.id() {
                    return;
                }

                maximized.set(desktop_window.is_maximized());
            }
        });
    }

    // fullscreen logic
    use_future(move || async move {
        #[derive(Debug, Copy, Clone, Deserialize, Serialize)]
        enum Evt {
            #[serde(rename = "F11")]
            F11,
        }

        while let Ok(evt) = events.recv().await {
            match serde_json::from_value::<Evt>(evt) {
                Ok(Evt::F11) => {
                    if fullscreen() {
                        window().set_fullscreen(false);
                        fullscreen.set(false)
                    } else {
                        window().set_fullscreen(true);
                        fullscreen.set(true)
                    }
                }
                Err(e) => {
                    eprintln!("failed deserializing event: {}", e);
                }
            }
        }
    });

    rsx! {
        ResizingArea { resizable: !fullscreen() && !maximized() }
        div {
            onmousemove: move |e| e.stop_propagation(),
            class: "w-full h-full grid grid-rows-[2rem_calc(100vh_-_2rem)] gap-0",
            if !fullscreen() {
                div {
                    class: "w-full bg-neutral-900 flex flex-row-reverse cursor-pointer",
                    ondoubleclick: move |_| {
                        if maximized() {
                            window().set_maximized(false);
                            maximized.set(false);
                        } else {
                            window().set_maximized(true);
                            maximized.set(true);
                        }
                    },
                    // when dragging, the mouseup event isn't sent, so we have to do this manually
                    onmousemove: move |e| {
                        if e.held_buttons().contains(MouseButton::Primary) && !moving() {
                            moving.set(true);
                            window().drag_window().unwrap();
                        } else if !e.held_buttons().contains(MouseButton::Primary) && moving() {
                            moving.set(false);
                        }
                    },
                    button {
                        onclick: move |_| { exit(0) },
                        class: "p-2 hover:bg-red-600 transition duration-100",
                        Icon { icon: FiX, fill: "#e8e7fe", class: "w-8 h-full" }
                    }
                    button {
                        onclick: move |_| {
                            if maximized() {
                                window().set_maximized(false);
                                maximized.set(false);
                            } else {
                                window().set_maximized(true);
                                maximized.set(true);
                            }
                        },
                        class: "p-2 hover:bg-neutral-600 transition duration-100",
                        if !maximized() {
                            Icon { icon: FiMaximize, fill: "#e8e7fe", class: "w-8 h-full" }
                        } else {
                            Icon { icon: FiMinimize, fill: "#e8e7fe", class: "w-8 h-full" }
                        }
                    }
                    button {
                        onclick: move |_| {
                            window().set_minimized(true);
                        },
                        class: "p-2 hover:bg-neutral-600 transition duration-100",
                        Icon { icon: FiMinus, fill: "#e8e7fe", class: "w-8 h-full" }
                    }
                }
            }
            div { class: "w-full", {children} }
        }
    }
}

// The code below is taken from the dioxus-free-icons crate, It has bee ncopied over to make it easier to maintain this example
// in case of future breaking changes in dioxus.

/// Icon shape trait
pub trait IconShape {
    fn view_box(&self) -> &str;
    fn xmlns(&self) -> &str;
    fn child_elements(&self) -> Element;
    fn fill_and_stroke<'a>(&self, user_color: &'a str) -> (&'a str, &'a str, &'a str) {
        ("none", user_color, "0")
    }
    fn stroke_linecap(&self) -> &str {
        "butt"
    }
    fn stroke_linejoin(&self) -> &str {
        "miter"
    }
}

/// Icon component Props
#[derive(PartialEq, Props, Clone)]
pub struct IconProps<T: IconShape + Clone + PartialEq + 'static> {
    /// The icon shape to use.
    pub icon: T,
    /// The height of the `<svg>` element. Defaults to 20.
    #[props(default = 20)]
    pub height: u32,
    /// The width of the `<svg>` element. Defaults to 20.
    #[props(default = 20)]
    pub width: u32,
    /// The color to use for filling the icon. Defaults to "currentColor".
    #[props(default = "currentColor".to_string())]
    pub fill: String,
    /// An class for the `<svg>` element.
    #[props(default = "".to_string())]
    pub class: String,
    /// An accessible, short-text description for the icon.
    pub title: Option<String>,
}

/// Icon component which generates SVG elements
#[allow(non_snake_case)]
pub fn Icon<T: IconShape + Clone + PartialEq + 'static>(props: IconProps<T>) -> Element {
    let (fill, stroke, stroke_width) = props.icon.fill_and_stroke(&props.fill);
    rsx!(
        svg {
            class: "{props.class}",
            height: "{props.height}",
            width: "{props.width}",
            view_box: "{props.icon.view_box()}",
            xmlns: "{props.icon.xmlns()}",
            fill,
            stroke,
            stroke_width,
            stroke_linecap: "{props.icon.stroke_linecap()}",
            stroke_linejoin: "{props.icon.stroke_linejoin()}",
            if let Some(title_text) = props.title {
                title { "{title_text}" }
            }
            {props.icon.child_elements()}
        }
    )
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FiMaximize;
impl IconShape for FiMaximize {
    fn view_box(&self) -> &str {
        "0 0 24 24"
    }
    fn xmlns(&self) -> &str {
        "http://www.w3.org/2000/svg"
    }
    fn fill_and_stroke<'a>(&self, user_color: &'a str) -> (&'a str, &'a str, &'a str) {
        ("none", user_color, "2")
    }
    fn stroke_linecap(&self) -> &str {
        "round"
    }
    fn stroke_linejoin(&self) -> &str {
        "round"
    }
    fn child_elements(&self) -> Element {
        rsx! {
            path { d: "M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FiMinimize;
impl IconShape for FiMinimize {
    fn view_box(&self) -> &str {
        "0 0 24 24"
    }
    fn xmlns(&self) -> &str {
        "http://www.w3.org/2000/svg"
    }
    fn fill_and_stroke<'a>(&self, user_color: &'a str) -> (&'a str, &'a str, &'a str) {
        ("none", user_color, "2")
    }
    fn stroke_linecap(&self) -> &str {
        "round"
    }
    fn stroke_linejoin(&self) -> &str {
        "round"
    }
    fn child_elements(&self) -> Element {
        rsx! {
            path { d: "M8 3v3a2 2 0 0 1-2 2H3m18 0h-3a2 2 0 0 1-2-2V3m0 18v-3a2 2 0 0 1 2-2h3M3 16h3a2 2 0 0 1 2 2v3" }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FiMinus;
impl IconShape for FiMinus {
    fn view_box(&self) -> &str {
        "0 0 24 24"
    }
    fn xmlns(&self) -> &str {
        "http://www.w3.org/2000/svg"
    }
    fn fill_and_stroke<'a>(&self, user_color: &'a str) -> (&'a str, &'a str, &'a str) {
        ("none", user_color, "2")
    }
    fn stroke_linecap(&self) -> &str {
        "round"
    }
    fn stroke_linejoin(&self) -> &str {
        "round"
    }
    fn child_elements(&self) -> Element {
        rsx! {
            line {
                x1: "5",
                x2: "19",
                y1: "12",
                y2: "12"
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FiX;
impl IconShape for FiX {
    fn view_box(&self) -> &str {
        "0 0 24 24"
    }
    fn xmlns(&self) -> &str {
        "http://www.w3.org/2000/svg"
    }
    fn fill_and_stroke<'a>(&self, user_color: &'a str) -> (&'a str, &'a str, &'a str) {
        ("none", user_color, "2")
    }
    fn stroke_linecap(&self) -> &str {
        "round"
    }
    fn stroke_linejoin(&self) -> &str {
        "round"
    }
    fn child_elements(&self) -> Element {
        rsx! {
            line {
                x1: "18",
                x2: "6",
                y1: "6",
                y2: "18"
            }
            line {
                x1: "6",
                x2: "18",
                y1: "6",
                y2: "18"
            }
        }
    }
}
