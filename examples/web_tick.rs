#![allow(non_upper_case_globals, non_snake_case)]
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
    // Setup logging
    // wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    // console_error_panic_hook::set_once();
    for adj in ADJECTIVES {
        wasm_bindgen::intern(adj);
    }
    for col in COLOURS {
        wasm_bindgen::intern(col);
    }
    for no in NOUNS {
        wasm_bindgen::intern(no);
    }
    wasm_bindgen::intern("col-md-1");
    wasm_bindgen::intern("col-md-6");
    wasm_bindgen::intern("glyphicon glyphicon-remove remove");
    wasm_bindgen::intern("remove");
    wasm_bindgen::intern("dioxus");
    wasm_bindgen::intern("lbl");
    wasm_bindgen::intern("true");

    dioxus::web::launch(App, |c| c);
}

static App: FC<()> = |cx| {
    // let mut count = use_state(cx, || 0);

    let mut rng = SmallRng::from_entropy();
    let rows = (0..1_000).map(|f| {
        let label = Label::new(&mut rng);
        rsx! {
            Row {
                row_id: f,
                label: label
            }
        }
    });
    cx.render(rsx! {
        table {
            tbody {
                {rows}
            }
        }
    })
    // cx.render(rsx! {
    //     div {
    //         // h1 { "Hifive counter: {count}" }
    //         // {cx.children()}
    //         // button { onclick: move |_| count += 1, "Up high!" }
    //         // button { onclick: move |_| count -= 1, "Down low!" }
    //         {(0..1000).map(|i| rsx!{ div { "Count: {count}" } })}
    //     }
    // })
};

#[derive(PartialEq, Props)]
struct RowProps {
    row_id: usize,
    label: Label,
}
fn Row<'a>(cx: Context<'a, RowProps>) -> DomTree {
    let [adj, col, noun] = cx.label.0;
    cx.render(rsx! {
        tr {
            td { class:"col-md-1", "{cx.row_id}" }
            td { class:"col-md-1", onclick: move |_| { /* run onselect */ }
                a { class: "lbl", "{adj}" "{col}" "{noun}" }
            }
            td { class: "col-md-1"
                a { class: "remove", onclick: move |_| {/* remove */}
                    span { class: "glyphicon glyphicon-remove remove" aria_hidden: "true" }
                }
            }
            td { class: "col-md-6" }
        }
    })
}
use rand::prelude::*;

#[derive(PartialEq)]
struct Label([&'static str; 3]);

impl Label {
    fn new(rng: &mut SmallRng) -> Self {
        Label([
            ADJECTIVES.choose(rng).unwrap(),
            COLOURS.choose(rng).unwrap(),
            NOUNS.choose(rng).unwrap(),
        ])
    }
}

static ADJECTIVES: &[&str] = &[
    "pretty",
    "large",
    "big",
    "small",
    "tall",
    "short",
    "long",
    "handsome",
    "plain",
    "quaint",
    "clean",
    "elegant",
    "easy",
    "angry",
    "crazy",
    "helpful",
    "mushy",
    "odd",
    "unsightly",
    "adorable",
    "important",
    "inexpensive",
    "cheap",
    "expensive",
    "fancy",
];

static COLOURS: &[&str] = &[
    "red", "yellow", "blue", "green", "pink", "brown", "purple", "brown", "white", "black",
    "orange",
];

static NOUNS: &[&str] = &[
    "table", "chair", "house", "bbq", "desk", "car", "pony", "cookie", "sandwich", "burger",
    "pizza", "mouse", "keyboard",
];
