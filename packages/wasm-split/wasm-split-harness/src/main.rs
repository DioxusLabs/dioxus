#![allow(non_snake_case)]

use dioxus::wasm_split;
use std::{pin::Pin, thread::LocalKey};

use dioxus::prelude::*;
use futures::AsyncReadExt;
use js_sys::Date;
use wasm_bindgen::prelude::*;
use wasm_split::{lazy_loader, LazyLoader, LazySplitLoader};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // static LOADER: wasm_split::LazyLoader<(), Element> =
    //     lazy_loader!(extern "eleven" fn Home(args: ()) -> Element);

    // let res = use_resource(|| async move { LOADER.load().await }).suspend()?;

    // LOADER.call(()).unwrap()

    rsx! {
        Router::<Route> {}
    }
}

// #[derive(Clone)]
// enum Route {
//     Home,
// }

// impl std::fmt::Display for Route {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Route::Home => f.write_str("/"),
//         }
//     }
// }

// impl std::str::FromStr for Route {
//     type Err = dioxus_router::routable::RouteParseError<String>;
//     fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
//         Ok(Self::Home)
//     }
// }

// fn Blog(args: ()) -> Element {
//     rsx! {
//         h1 { "Blog" }
//     }
// }

// #[inline(never)]
// #[no_mangle]
// pub fn HomeInner() -> Element {
//     static HOME_LOADER: wasm_split::LazyLoader<(), Element> =
//         lazy_loader!(extern "eleven" fn Home(args: ()) -> Element);

//     static BLOG_LOADER: wasm_split::LazyLoader<(), Element> =
//         lazy_loader!(extern "twelve" fn Blog(args: ()) -> Element);

//     use_resource(|| async move { HOME_LOADER.load().await }).suspend()?;
//     use_resource(|| async move { BLOG_LOADER.load().await }).suspend()?;
//     rsx! {
//         {HOME_LOADER.call(()).unwrap()}
//         {BLOG_LOADER.call(()).unwrap()}
//         ChildSplit {}
//     }
// }

// impl Routable for Route {
//     #[doc = " The error that can occur when parsing a route."]
//     const SITE_MAP: &'static [SiteMapSegment] = &[];

//     #[doc = " Render the route at the given level"]
//     fn render(&self, level: usize) -> Element {
//         match self {
//             Route::Home => rsx! {
//                 HomeInner {}
//             },
//         }
//     }
// }

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
            Link { to: Route::Home, "Home" }
            Link { to: Route::ChildSplit, "Child" }
            Outlet::<Route> {}
        }
    }
}

pub(crate) static GLOBAL_COUNTER: GlobalSignal<usize> = Signal::global(|| 0);

fn Home(args: ()) -> Element {
    let mut count = use_signal(|| 1);

    // rsx! {
    //     div { "Hello" }
    //     p { "Count: {count}" }
    // }

    rsx! {
        h1 { "Hello bundle split 456" }
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
        button {
            onclick: move |_| async move {
                gzip_it().await;
            },
            "GZIP it"
        }
        // button {
        //     onclick: move |_| async move {
        //         brotli_it(&[0u8; 10]).await;
        //     },
        //     "Brotli It"
        // }
        h3 { "Global Counter: {GLOBAL_COUNTER}" }
        div { id: "output-box" }
        // ChildSplit {}
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

    dioxus::prelude::queue_effect(move || {
        web_sys::console::log_1(&"add body async internal!".into());
        *GLOBAL_COUNTER.write() += 2;
    });
}

#[wasm_split::wasm_split(three)]
async fn brotli_it(data: &'static [u8]) {
    let reader = Box::pin(futures::io::BufReader::new(data));
    let reader: Pin<Box<dyn futures::io::AsyncBufRead>> = reader;

    dioxus::prelude::spawn(async move {
        let mut fut = Box::pin(async_compression::futures::bufread::BrotliDecoder::new(
            reader,
        ));
        if fut.read_to_end(&mut Vec::new()).await.is_err() {
            web_sys::console::log_1(&"error reading brotli".into());
        }
        *GLOBAL_COUNTER.write() += 3;
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
        *GLOBAL_COUNTER.write() += 4;
    });
}

fn ChildSplit() -> Element {
    pub(crate) static DATE: GlobalSignal<Date> = Signal::global(|| Date::new_0());

    static LOADER: wasm_split::LazyLoader<(), Element> =
        lazy_loader!(extern "five" fn InnerChild(props: ()) -> Element);

    fn InnerChild(props: ()) -> Element {
        static LOADER2: wasm_split::LazyLoader<Signal<String>, Element> =
            lazy_loader!(extern "fortytwo" fn InnerChild3(props: Signal<String>) -> Element);

        fn InnerChild3(count: Signal<String>) -> Element {
            // fn InnerChild3(count: Signal<String>) -> Element {
            pub(crate) static IconCheckGh: Component<()> = |cx| {
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
                IconCheckGh {}
            }
        }

        #[wasm_bindgen(module = "/src/stars.js")]
        extern "C" {
            pub(crate) fn get_stars(name: String) -> Option<usize>;
            pub(crate) fn set_stars(name: String, stars: usize);
        }

        let num = get_stars("stars".to_string()).unwrap_or(0);

        let inner_child = use_resource(|| async move { LOADER2.load().await }).suspend()?;
        let mut count = use_signal(|| "hello".to_string());

        let fp = LOADER2.call(count).unwrap();

        rsx! {
            h1 { "Some huge child?" }
            p { "Stars: {num}" }
            button {
                onclick: move |_| {
                    set_stars("stars".to_string(), num + 1);
                    dioxus::prelude::needs_update();
                },
                "Add Star"
            }
            {fp}
            h3 { "count: {count}" }
            button {
                onclick: move |_| {
                    *count.write() += " world";
                },
                "Add World"
            }
        }

        // rsx! {
        //     "hi"
        // }
    }

    use_resource(|| async move { LOADER.load().await }).suspend()?;

    LOADER.call(()).unwrap()
}
