use std::cell::Cell;

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_hooks::{use_ref, use_state};
use dioxus_html as dioxus_elements;
use dioxus_web;
use rand::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    if cfg!(debug_assertions) {
        wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
        log::debug!("hello world");
    }

    for a in ADJECTIVES {
        wasm_bindgen::intern(*a);
    }
    for a in COLOURS {
        wasm_bindgen::intern(*a);
    }
    for a in NOUNS {
        wasm_bindgen::intern(*a);
    }
    for a in [
        "container",
        "jumbotron",
        "row",
        "Dioxus",
        "col-md-6",
        "col-md-1",
        "Create 1,000 rows",
        "run",
        "Create 10,000 rows",
        "runlots",
        "Append 1,000 rows",
        "add",
        "Update every 10th row",
        "update",
        "Clear",
        "clear",
        "Swap rows",
        "swaprows",
        "preloadicon glyphicon glyphicon-remove", //
        "aria-hidden",
        "onclick",
        "true",
        "false",
        "danger",
        "type",
        "id",
        "class",
        "glyphicon glyphicon-remove remove",
        "dioxus-id",
        "dioxus-event-click",
        "dioxus",
        "click",
        "1.10",
        "lbl",
        "remove",
        "dioxus-event",
        "col-sm-6 smallpad",
        "btn btn-primary btn-block",
        "",
        " ",
    ] {
        wasm_bindgen::intern(a);
    }
    for x in 0..100_000 {
        wasm_bindgen::intern(&x.to_string());
    }

    dioxus_web::launch(App);
}

#[derive(Clone, PartialEq, Copy)]
struct Label {
    key: usize,
    labels: [&'static str; 3],
}

static mut Counter: Cell<usize> = Cell::new(1);

impl Label {
    fn new_list(num: usize) -> Vec<Self> {
        let mut rng = SmallRng::from_entropy();
        let mut labels = Vec::with_capacity(num);

        let offset = unsafe { Counter.get() };
        unsafe { Counter.set(offset + num) };

        for k in offset..(offset + num) {
            labels.push(Label {
                key: k,
                labels: [
                    ADJECTIVES.choose(&mut rng).unwrap(),
                    COLOURS.choose(&mut rng).unwrap(),
                    NOUNS.choose(&mut rng).unwrap(),
                ],
            });
        }

        labels
    }
}

static App: Component<()> = |cx| {
    let mut items = use_ref(&cx, || vec![]);
    let mut selected = use_state(&cx, || None);

    cx.render(rsx! {
        div { class: "container"
            div { class: "jumbotron"
                div { class: "row"
                    div { class: "col-md-6", h1 { "Dioxus" } }
                    div { class: "col-md-6"
                        div { class: "row"
                            ActionButton { name: "Create 1,000 rows", id: "run",
                                onclick: move || items.set(Label::new_list(1_000)),
                            }
                            ActionButton { name: "Create 10,000 rows", id: "runlots",
                                onclick: move || items.set(Label::new_list(10_000)),
                            }
                            ActionButton { name: "Append 1,000 rows", id: "add",
                                onclick: move || items.write().extend(Label::new_list(1_000)),
                            }
                            ActionButton { name: "Update every 10th row", id: "update",
                                onclick: move || items.write().iter_mut().step_by(10).for_each(|item| item.labels[2] = "!!!"),
                            }
                            ActionButton { name: "Clear", id: "clear",
                                onclick: move || items.write().clear(),
                            }
                            ActionButton { name: "Swap Rows", id: "swaprows",
                                onclick: move || items.write().swap(0, 998),
                            }
                        }
                    }
                }
            }
            table { class: "table table-hover table-striped test-data"
                tbody { id: "tbody"
                    {items.read().iter().enumerate().map(|(id, item)| {
                        let [adj, col, noun] = item.labels;
                        let is_in_danger = if (*selected).map(|s| s == id).unwrap_or(false) {"danger"} else {""};
                        rsx!(tr { 
                            class: "{is_in_danger}",
                            key: "{id}",
                            td { class:"col-md-1" }
                            td { class:"col-md-1", "{item.key}" }
                            td { class:"col-md-1", onclick: move |_| selected.set(Some(id)),
                                a { class: "lbl", "{adj} {col} {noun}" }
                            }
                            td { class: "col-md-1"
                                a { class: "remove", onclick: move |_| { items.write().remove(id); },
                                    span { class: "glyphicon glyphicon-remove remove" aria_hidden: "true" }
                                }
                            }
                            td { class: "col-md-6" }
                        })
                    })}
                }
             }
            span { class: "preloadicon glyphicon glyphicon-remove" aria_hidden: "true" }
        }
    })
};

#[derive(Props)]
struct ActionButtonProps<'a> {
    name: &'static str,
    id: &'static str,
    onclick: &'a dyn Fn(),
}

fn ActionButton<'a>(cx: Scope<'a, ActionButtonProps<'a>>) -> Element {
    rsx!(cx, div { class: "col-sm-6 smallpad"
        button { class:"btn btn-primary btn-block", r#type: "button", id: "{cx.props.id}",  onclick: move |_| (cx.props.onclick)(),
            "{cx.props.name}"
        }
    })
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

// #[derive(PartialEq, Props)]
// struct RowProps<'a> {
//     row_id: usize,
//     label: &'a Label,
// }

// fn Row(cx: Context, props: &RowProps) -> Element {
//     rsx!(cx, tr {
//         td { class:"col-md-1", "{cx.props.row_id}" }
//         td { class:"col-md-1", onclick: move |_| { /* run onselect */ }
//             a { class: "lbl", {cx.props.label.labels} }
//         }
//         td { class: "col-md-1"
//             a { class: "remove", onclick: move |_| {/* remove */}
//                 span { class: "glyphicon glyphicon-remove remove" aria_hidden: "true" }
//             }
//         }
//         td { class: "col-md-6" }
//     })
// }
