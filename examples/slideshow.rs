//! Example: Webview Renderer
//! -------------------------
//!
//! This example shows how to use the dioxus_desktop crate to build a basic desktop application.
//!
//! Under the hood, the dioxus_desktop crate bridges a native Dioxus VirtualDom with a custom prebuit application running
//! in the webview runtime. Custom handlers are provided for the webview instance to consume patches and emit user events
//! into the native VDom instance.
//!
//! Currently, NodeRefs won't work properly, but all other event functionality will.

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App, |c| c);
}

static App: FC<()> = |cx| {
    let slides = use_state(cx, SlideController::new);

    let slide = match slides.slide_id {
        0 => cx.render(rsx!(Title {})),
        1 => cx.render(rsx!(Slide1 {})),
        2 => cx.render(rsx!(Slide2 {})),
        3 => cx.render(rsx!(Slide3 {})),
        _ => cx.render(rsx!(End {})),
    };

    cx.render(rsx! {
        div {
            style: {
                background_color: "red"
            }
            div {
                div { h1 {"my awesome slideshow"} }
                div {
                    button {"<-", onclick: move |_| slides.get_mut().go_forward()}
                    h3 { "{slides.slide_id}" }
                    button {"->" onclick: move |_| slides.get_mut().go_backward()}
                 }
            }
            {slide}
        }
    })
};

#[derive(Clone)]
struct SlideController {
    slide_id: isize,
}
impl SlideController {
    fn new() -> Self {
        Self { slide_id: 0 }
    }
    fn can_go_forward(&self) -> bool {
        false
    }
    fn can_go_backward(&self) -> bool {
        true
    }
    fn go_forward(&mut self) {
        if self.can_go_forward() {
            self.slide_id += 1;
        }
    }
    fn go_backward(&mut self) {
        if self.can_go_backward() {
            self.slide_id -= 1;
        }
    }
}

const Title: FC<()> = |cx| {
    cx.render(rsx! {
        div {
            h1 { "Title" }
            p {}
        }
    })
};
const Slide1: FC<()> = |cx| {
    cx.render(rsx! {
        div {
            h1 { "Slide1" }
            p {}
        }
    })
};
const Slide2: FC<()> = |cx| {
    cx.render(rsx! {
        div {
            h1 { "Slide2" }
            p {}
        }
    })
};
const Slide3: FC<()> = |cx| {
    cx.render(rsx! {
        div {
            h1 { "Slide3" }
            p {}
        }
    })
};
const End: FC<()> = |cx| {
    cx.render(rsx! {
        div {
            h1 { "End" }
            p {}
        }
    })
};
