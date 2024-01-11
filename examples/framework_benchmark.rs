#![allow(non_snake_case)]

use dioxus::prelude::*;
use rand::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

#[derive(Clone, PartialEq)]
struct Label {
    key: usize,
    labels: [&'static str; 3],
}

impl Label {
    fn new_list(num: usize) -> Vec<Self> {
        let mut rng = SmallRng::from_entropy();
        let mut labels = Vec::with_capacity(num);
        for x in 0..num {
            labels.push(Label {
                key: x,
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

fn app(cx: Scope) -> Element {
    let items = use_ref(cx, Vec::new);
    let selected = use_state(cx, || None);

    cx.render(rsx! {
        div { class: "container",
            div { class: "jumbotron",
                div { class: "row",
                    div { class: "col-md-6", h1 { "Dioxus" } }
                    div { class: "col-md-6",
                        div { class: "row",
                            ActionButton { name: "Create 1,000 rows", id: "run",
                                onclick: move |_| items.set(Label::new_list(1_000)),
                            }
                            ActionButton { name: "Create 10,000 rows", id: "runlots",
                                onclick: move |_| items.set(Label::new_list(10_000)),
                            }
                            ActionButton { name: "Append 1,000 rows", id: "add",
                                onclick: move |_| items.write().extend(Label::new_list(1_000)),
                            }
                            ActionButton { name: "Update every 10th row", id: "update",
                                onclick: move |_| items.write().iter_mut().step_by(10).for_each(|item| item.labels[2] = "!!!"),
                            }
                            ActionButton { name: "Clear", id: "clear",
                                onclick: move |_| items.write().clear(),
                            }
                            ActionButton { name: "Swap rows", id: "swaprows",
                                onclick: move |_| items.write().swap(0, 998),
                            }
                        }
                    }
                }
            }
            table {
                tbody {
                    for (id, item) in items.read().iter().enumerate() {
                        tr {
                            class: if (*selected).map(|s| s == id).unwrap_or(false) { "danger" },
                            td { class:"col-md-1" }
                            td { class:"col-md-1", "{item.key}" }
                            td { class:"col-md-1", onclick: move |_| selected.set(Some(id)),
                                a { class: "lbl", "{item.labels[0]}{item.labels[1]}{item.labels[2]}" }
                            }
                            td { class: "col-md-1",
                                a { class: "remove", onclick: move |_| { items.write().remove(id); },
                                    span { class: "glyphicon glyphicon-remove remove", aria_hidden: "true" }
                                }
                            }
                            td { class: "col-md-6" }
                        }
                    }

                }
             }
            span { class: "preloadicon glyphicon glyphicon-remove", aria_hidden: "true" }
        }
    })
}

#[derive(Props)]
struct ActionButtonProps<'a> {
    name: &'a str,
    id: &'a str,
    onclick: EventHandler<'a>,
}

fn ActionButton<'a>(cx: Scope<'a, ActionButtonProps<'a>>) -> Element {
    cx.render(rsx! {
        div {
            class: "col-sm-6 smallpad",
            button {
                class:"btn btn-primary btn-block",
                r#type: "button",
                id: "{cx.props.id}",
                onclick: move |_| cx.props.onclick.call(()),

                "{cx.props.name}"
            }
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
