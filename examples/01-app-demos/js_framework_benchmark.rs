//! The keyed js-framework-benchmark app, ported from Leptos's example.
//!
//! Run with `dx serve --example js_framework_benchmark --platform web`.
//! Labels have independent signals and selection uses per-row memos, so neither
//! operation needs to render the entire list again.

use dioxus::prelude::*;
use rand::prelude::*;
use std::rc::Rc;

const ADJECTIVES: &[&str] = &[
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
const COLOURS: &[&str] = &[
    "red", "yellow", "blue", "green", "pink", "brown", "purple", "brown", "white", "black",
    "orange",
];
const NOUNS: &[&str] = &[
    "table", "chair", "house", "bbq", "desk", "car", "pony", "cookie", "sandwich", "burger",
    "pizza", "mouse", "keyboard",
];

#[derive(PartialEq)]
struct RowData {
    id: usize,
    label: Signal<String>,
}

impl Drop for RowData {
    fn drop(&mut self) {
        self.label.manually_drop();
    }
}

fn build_data(count: usize, next_id: &mut usize) -> Vec<Rc<RowData>> {
    let mut rng = rand::rng();
    (0..count)
        .map(|_| {
            let label = format!(
                "{} {} {}",
                ADJECTIVES.choose(&mut rng).unwrap(),
                COLOURS.choose(&mut rng).unwrap(),
                NOUNS.choose(&mut rng).unwrap(),
            );
            let id = *next_id;
            *next_id += 1;
            Rc::new(RowData {
                id,
                label: Signal::leak_with_caller(label, std::panic::Location::caller()),
            })
        })
        .collect()
}

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut data = use_signal(Vec::<Rc<RowData>>::new);
    let mut selected = use_signal(|| None::<usize>);
    let mut next_id = use_signal(|| 1);

    rsx! {
        document::Link {
            rel: "stylesheet",
            href: "https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/css/bootstrap.min.css",
        }
        div { class: "container",
            div { class: "jumbotron",
                div { class: "row",
                    div { class: "col-md-6", h1 { "Dioxus" } }
                    div { class: "col-md-6",
                        div { class: "row",
                            Button { id: "run", text: "Create 1,000 rows", onclick: move |_| {
                                data.set(build_data(1_000, &mut next_id.write()));
                                selected.set(None);
                            } }
                            Button { id: "runlots", text: "Create 10,000 rows", onclick: move |_| {
                                data.set(build_data(10_000, &mut next_id.write()));
                                selected.set(None);
                            } }
                            Button { id: "add", text: "Append 1,000 rows", onclick: move |_| {
                                data.write().extend(build_data(1_000, &mut next_id.write()));
                            } }
                            Button { id: "update", text: "Update every 10th row", onclick: move |_| {
                                for row in data.peek().iter().step_by(10) {
                                    let mut label = row.label;
                                    label.write().push_str(" !!!");
                                }
                            } }
                            Button { id: "clear", text: "Clear", onclick: move |_| {
                                data.clear();
                                selected.set(None);
                            } }
                            Button { id: "swaprows", text: "Swap Rows", onclick: move |_| {
                                if data.peek().len() > 998 {
                                    data.write().swap(1, 998);
                                }
                            } }
                        }
                    }
                }
            }
            table { class: "table table-hover table-striped test-data",
                tbody {
                    for row in data.read().iter() {
                        Row { key: "{row.id}", row: row.clone(), selected,
                            onremove: move |id| data.write().retain(|row| row.id != id),
                        }
                    }
                }
            }
            span { class: "preloadicon glyphicon glyphicon-remove", "aria-hidden": "true" }
        }
    }
}

#[component]
fn Button(id: &'static str, text: &'static str, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { class: "col-sm-6 smallpad",
            button { id, class: "btn btn-primary btn-block", r#type: "button", onclick, "{text}" }
        }
    }
}

#[component]
fn Row(
    row: Rc<RowData>,
    mut selected: Signal<Option<usize>>,
    onremove: EventHandler<usize>,
) -> Element {
    let id = row.id;
    let label = row.label;
    let is_selected = use_memo(move || selected() == Some(id));
    rsx! {
        tr { class: if is_selected() { "danger" } else { "" },
            td { class: "col-md-1", "{id}" }
            td { class: "col-md-4", a { onclick: move |_| selected.set(Some(id)), "{label}" } }
            td { class: "col-md-1",
                a { onclick: move |_| onremove.call(id),
                    span { class: "glyphicon glyphicon-remove", "aria-hidden": "true" }
                }
            }
            td { class: "col-md-6" }
        }
    }
}
