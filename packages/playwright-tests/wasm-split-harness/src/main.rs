use dioxus::prelude::*;
use futures::AsyncReadExt;
use std::pin::Pin;
use wasm_bindgen::prelude::*;

fn main() {
    dioxus::launch(app);
    dioxus::launch(|| {
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Routable, Clone, PartialEq, Debug)]
enum Route {
    #[layout(Nav)]
    #[route("/")]
    Home,

    #[route("/about")]
    About,
}

#[component]
fn Nav() -> Element {
    rsx! {
        nav {
            Link { to: Route::Home {}, "Home" }
            Link { to: Route::About {}, "About" }
        }
        div { Outlet::<Route> {} }
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        h1 { "Home" }
        p { "This is the home page" }
    }
}

#[component]
fn About() -> Element {
    rsx! {
        h1 { "About" }
        p { "This is the about page" }
    }
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "Hello bundle split" }
        h3 { "Count: {count}" }
        button { onclick: move |_| count += 1, "Click me" }
        button { onclick: move |_| add_body_text(), "Add body text" }
        button {
            onclick: move |_| async move {
                add_body_element().await;
                count += 1;
            },
            "Add body element"
        }
        button { onclick: move |_| gzip_it(), "GZIP it" }
        button { onclick: move |_| brotli_it(), "Brotli It" }
        div { id: "output-box" }
    }
}

#[wasm_split::wasm_split(one)]
async fn add_body_text() {
    let window = web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    let output = document.create_text_node("Rendered!");
    let output_box = document.get_element_by_id("output-box").unwrap_throw();
    output_box.append_child(&output).unwrap_throw();
}

#[wasm_split::wasm_split(two)]
async fn add_body_element() {
    let window = web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    let output = document.create_element("div").unwrap_throw();
    output.set_text_content(Some("Some inner div"));
    let output_box = document.get_element_by_id("output-box").unwrap_throw();
    output_box.append_child(&output).unwrap_throw();

    dioxus::prelude::queue_effect(move || {
        web_sys::console::log_1(&"add body async internal!".into());
    });
}

#[wasm_split::wasm_split(three)]
async fn brotli_it() {
    static DATA: &[u8] = &[0u8; 10];
    let reader = Box::pin(futures::io::BufReader::new(DATA));
    let reader: Pin<Box<dyn futures::io::AsyncBufRead>> = reader;

    dioxus::prelude::spawn(async move {
        let mut fut = Box::pin(async_compression::futures::bufread::BrotliDecoder::new(
            reader,
        ));
        if fut.read_to_end(&mut Vec::new()).await.is_err() {
            web_sys::console::log_1(&"error reading brotli".into());
        }
    });
}

#[wasm_split::wasm_split(four)]
async fn gzip_it() {
    static DATA: &[u8] = &[0u8; 10];
    let reader = Box::pin(futures::io::BufReader::new(DATA));
    let reader: Pin<Box<dyn futures::io::AsyncBufRead>> = reader;

    dioxus::prelude::spawn(async move {
        let mut fut = Box::pin(async_compression::futures::bufread::GzipDecoder::new(
            reader,
        ));
        if fut.read_to_end(&mut Vec::new()).await.is_err() {
            web_sys::console::log_1(&"error reading gzip".into());
        }
    });
}
