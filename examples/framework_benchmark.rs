//! JS Framework Benchmark
//! ----------------------
//!
//! This example is used in the JS framework benchmarking tool to compare Dioxus' performance with other frontend frameworks.
//!
//!
//!

use dioxus::{events::on::MouseEvent, prelude::*};
use dioxus_html as dioxus_elements;
use fxhash::{FxBuildHasher, FxHasher32};
use std::rc::Rc;

fn main() {
    log::debug!("starting!");
    dioxus::desktop::launch(App, |c| c);
}

// We use a special immutable hashmap to make hashmap operations efficient
type RowList = im_rc::HashMap<usize, Rc<str>, FxBuildHasher>;

static App: FC<()> = |cx| {
    let items = use_state(cx, || RowList::default());

    let create_rendered_rows = move |from, num| move |_| items.set(create_row_list(from, num));

    let append_1_000_rows =
        move |_| items.set(create_row_list(items.len(), 1000).union((*items).clone()));

    let update_every_10th_row = move |_| {
        let mut new_items = (*items).clone();
        let mut small_rng = SmallRng::from_entropy();
        new_items.iter_mut().step_by(10).for_each(|(_, val)| {
            *val = create_new_row_label(&mut String::with_capacity(30), &mut small_rng)
        });
        items.set(new_items);
    };
    let clear_rows = move |_| items.set(RowList::default());

    let swap_rows = move |_| {
        // this looks a bit ugly because we're using a hashmap instead of a vec
        if items.len() > 998 {
            let mut new_items = (*items).clone();
            let a = new_items.get(&0).unwrap().clone();
            *new_items.get_mut(&0).unwrap() = new_items.get(&998).unwrap().clone();
            *new_items.get_mut(&998).unwrap() = a;
            items.set(new_items);
        }
    };

    let rows = items.iter().map(|(key, value)| {
        rsx!(Row {
            key: "{key}",
            row_id: *key as usize,
            label: value.clone(),
        })
    });

    cx.render(rsx! {
        div { class: "container"
            div { class: "jumbotron"
                div { class: "row"
                    div { class: "col-md-6", h1 { "Dioxus" } }
                    div { class: "col-md-6"
                        div { class: "row"
                            ActionButton { name: "Create 1,000 rows", id: "run", action: create_rendered_rows(0, 1_000) }
                            ActionButton { name: "Create 10,000 rows", id: "runlots", action: create_rendered_rows(0, 10_000) }
                            ActionButton { name: "Append 1,000 rows", id: "add", action: append_1_000_rows }
                            ActionButton { name: "Update every 10th row", id: "update", action: update_every_10th_row, }
                            ActionButton { name: "Clear", id: "clear", action: clear_rows }
                            ActionButton { name: "Swap rows", id: "swaprows", action: swap_rows }
                        }
                    }
                }
            }
            table {
                tbody {
                    {rows}
                }
             }
            span {}
        }
    })
};

#[derive(Clone)]
struct RowController {}

#[derive(Props)]
struct ActionButtonProps<F: Fn(MouseEvent)> {
    name: &'static str,
    id: &'static str,
    action: F,
}
fn ActionButton<F>(cx: Context<ActionButtonProps<F>>) -> DomTree
where
    F: Fn(MouseEvent),
{
    cx.render(rsx! {
        div { class: "col-sm-6 smallpad"
            button { class:"btn btn-primary btn-block", r#type: "button", id: "{cx.id}",  onclick: {&cx.action},
                "{cx.name}"
            }
        }
    })
}

#[derive(PartialEq, Props)]
struct RowProps {
    row_id: usize,
    label: Rc<str>,
}
fn Row<'a>(cx: Context<'a, RowProps>) -> DomTree {
    cx.render(rsx! {
        tr {
            td { class:"col-md-1", "{cx.row_id}" }
            td { class:"col-md-1", onclick: move |_| { /* run onselect */ }
                a { class: "lbl", "{cx.label}" }
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
fn create_new_row_label(label: &mut String, rng: &mut SmallRng) -> Rc<str> {
    label.push_str(ADJECTIVES.choose(rng).unwrap());
    label.push(' ');
    label.push_str(COLOURS.choose(rng).unwrap());
    label.push(' ');
    label.push_str(NOUNS.choose(rng).unwrap());
    Rc::from(label.as_ref())
}

fn create_row_list(from: usize, num: usize) -> RowList {
    let mut small_rng = SmallRng::from_entropy();
    let mut buf = String::with_capacity(35);
    (from..num + from)
        .map(|f| {
            let o = (f, create_new_row_label(&mut buf, &mut small_rng));
            buf.clear();
            o
        })
        .collect::<RowList>()
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
