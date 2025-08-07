//! This file is a fuzz-test for the wasm-split engine to ensure that it works as expected.
//! The docsite is a better target for this, but we try to boil down the complexity into this small
//! test file.

#![allow(non_snake_case)]

use dioxus::prelude::*;
use futures::AsyncReadExt;
use js_sys::Date;
use std::pin::Pin;
use wasm_bindgen::prelude::*;
use wasm_split::lazy_loader;

fn main() {
    dioxus::launch(|| {
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Routable, PartialEq, Eq, Debug, Clone)]
enum Route {
    #[layout(Nav)]
    #[route("/")]
    Home,
    #[route("/child")]
    ChildSplit,
}

fn Nav() -> Element {
    rsx! {
        div {
            Link { id: "link-home", to: Route::Home, "Home" }
            Link { id: "link-child", to: Route::ChildSplit, "Child" }
            Outlet::<Route> {}
        }
    }
}

pub(crate) static GLOBAL_COUNTER: GlobalSignal<usize> = Signal::global(|| 0);

fn Home() -> Element {
    let mut count = use_signal(|| 1);
    let mut res = use_signal(|| "hello".to_string());

    rsx! {
        h1 { "Hello bundle split 456" }
        h3 { id: "counter-display", "Count: {count}" }
        h3 { id: "global-counter", "Global Counter: {GLOBAL_COUNTER}" }
        button {
            id: "increment-counter",
            onclick: move |_| count += 1,
            "Click me"
        }
        button {
            id: "increment-counter-global",
            onclick: move |_| *GLOBAL_COUNTER.write() += 1,
            "Click me"
        }
        button {
            id: "add-body-text",
            onclick: move |_| add_body_text(),
            "Add body text"
        }
        button {
            id: "add-body-element",
            onclick: move |_| async move {
                add_body_element().await;
                count += 1;
            },
            "Add body element"
        }
        button {
            id: "gzip-it",
            onclick: move |_| async move {
                gzip_it().await;
            },
            "GZIP it"
        }
        button {
            id: "brotli-it",
            onclick: move |_| async move {
                brotli_it(&[0u8; 10]).await;
            },
            "Brotli It"
        }
        button {
            id: "make-request",
            onclick: move |_| async move {
                let res_ = make_request().await.unwrap();
                res.set(res_);
            },
            "Make Request!"
        }
        button {
            id: "make-local-request",
            onclick: move |_| async move {
                let client = reqwest::Client::new();
                let response = client
                    .get("https://dog.ceo/api/breeds/image/random")
                    .send()
                    .await?;
                let body = response.text().await?;
                *res.write() = body;
                Ok(())
            },
            "local request"
        }
        LazyComponent {
            abc: 0
        }
        div { "Response: {res}" }
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
    *GLOBAL_COUNTER.write() += 1;
}

#[wasm_split::wasm_split(two)]
async fn add_body_element() {
    let window = web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    let output = document.create_element("div").unwrap_throw();
    output.set_text_content(Some("Some inner div"));
    let output_box = document.get_element_by_id("output-box").unwrap_throw();
    output_box.append_child(&output).unwrap_throw();

    dioxus::core::queue_effect(move || {
        web_sys::console::log_1(&"add body async internal!".into());
        *GLOBAL_COUNTER.write() += 2;
    });
}

#[wasm_split::wasm_split(four)]
async fn gzip_it() {
    static DATA: &[u8] = &[0u8; 10];
    let reader = Box::pin(futures::io::BufReader::new(DATA));
    let reader: Pin<Box<dyn futures::io::AsyncBufRead>> = reader;

    dioxus::core::spawn(async move {
        let mut fut = Box::pin(async_compression::futures::bufread::GzipDecoder::new(
            reader,
        ));
        if fut.read_to_end(&mut Vec::new()).await.is_err() {
            web_sys::console::log_1(&"error reading gzip".into());
        }
        *GLOBAL_COUNTER.write() += 3;

        let res: Result<String, anyhow::Error> = Box::pin(async move {
            let client = reqwest::Client::new();
            let response = client
                .get("https://dog.ceo/api/breeds/image/random")
                .send()
                .await?;
            let body = response.text().await?;
            Ok(body)
        })
        .await;

        if res.is_err() {
            web_sys::console::log_1(&"error making request".into());
        }
    });
}

#[wasm_split::wasm_split(three)]
async fn brotli_it(data: &'static [u8]) {
    let reader = Box::pin(futures::io::BufReader::new(data));
    let reader: Pin<Box<dyn futures::io::AsyncBufRead>> = reader;

    dioxus::core::spawn(async move {
        let mut fut = Box::pin(async_compression::futures::bufread::BrotliDecoder::new(
            reader,
        ));
        if fut.read_to_end(&mut Vec::new()).await.is_err() {
            web_sys::console::log_1(&"error reading brotli".into());
        }
        *GLOBAL_COUNTER.write() += 4;
    });
}

#[wasm_split::wasm_split(eleven)]
async fn make_request() -> Result<String, anyhow::Error> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://dog.ceo/api/breeds/image/random")
        .send()
        .await?;
    let body = response.text().await?;
    Ok(body)
}

#[component(lazy)]
fn LazyComponent(abc: i32) -> Element {
    rsx! {
        div {
            "This is a lazy component! {abc}"
        }
    }
}

fn ChildSplit() -> Element {
    pub(crate) static DATE: GlobalSignal<Date> = Signal::global(Date::new_0);

    static LOADER: wasm_split::LazyLoader<(), Element> =
        lazy_loader!(extern "five" fn InnerChild(props: ()) -> Element);

    fn InnerChild(_props: ()) -> Element {
        static LOADER2: wasm_split::LazyLoader<Signal<String>, Element> =
            lazy_loader!(extern "fortytwo" fn InnerChild3(props: Signal<String>) -> Element);

        fn InnerChild3(count: Signal<String>) -> Element {
            pub(crate) static ICONCHECK: Component<()> = |_| {
                rsx! {
                    svg {
                        class: "octicon octicon-check js-clipboard-check-icon d-inline-block d-none",
                        fill: "rgb(26, 127, 55)",
                        height: "24",
                        version: "1.1",
                        "aria_hidden": "true",
                        width: "24",
                        view_box: "0 0 16 16",
                        "data_view_component": "true",
                        path {
                            d: "M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z",
                            fill_rule: "evenodd",
                        }
                    }
                    button {
                        onclick: move |_| {
                            *DATE.write() = Date::new_0();
                        },
                        "Update Date"
                    }
                }
            };

            let now = DATE.read().clone();

            rsx! {
                h1 { "Some other child" }
                h3 { "Global Counter: {GLOBAL_COUNTER}" }
                h3 { "Date: {now.to_date_string()}" }
                h3 { "count: {count}" }
                ICONCHECK {}
            }
        }

        #[wasm_bindgen(module = "/src/stars.js")]
        extern "C" {
            pub(crate) fn get_stars(name: String) -> Option<usize>;
            pub(crate) fn set_stars(name: String, stars: usize);
        }

        let num = get_stars("stars".to_string()).unwrap_or(0);

        use_resource(|| async move { LOADER2.load().await }).suspend()?;
        let mut count = use_signal(|| "hello".to_string());

        let fp = LOADER2.call(count).unwrap();

        rsx! {
            h1 { "Some huge child?" }
            p { "Stars: {num}" }
            button {
                onclick: move |_| {
                    set_stars("stars".to_string(), num + 1);
                    dioxus::core::needs_update();
                },
                "Add Star"
            }
            {fp}
            h3 { id: "nested-child-count", "Count: {count}" }
            button {
                id: "nested-child-add-world",
                onclick: move |_| {
                    *count.write() += " world";
                },
                "Add World"
            }
        }
    }

    use_resource(|| async move { LOADER.load().await }).suspend()?;

    LOADER.call(()).unwrap()
}
