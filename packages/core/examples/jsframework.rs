#![allow(non_snake_case)]

use dioxus::component::Scope;
use dioxus::events::on::MouseEvent;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use rand::prelude::*;
use std::fmt::Display;

fn main() {
    let mut dom = VirtualDom::new(App);
    let g = dom.rebuild();
    assert!(g.edits.len() > 1);
}

fn App((cx, props): Scope<()>) -> Element {
    let mut rng = SmallRng::from_entropy();
    let rows = (0..10_000_usize).map(|f| {
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
}

#[derive(PartialEq, Props)]
struct RowProps {
    row_id: usize,
    label: Label,
}

fn Row((cx, props): Scope<RowProps>) -> Element {
    let handler = move |evt: MouseEvent| {
        let g = evt.button;
    };
    cx.render(rsx! {
        tr {
            td { class:"col-md-1", "{props.row_id}" }
            td { class:"col-md-1", onclick: move |_| { /* run onselect */ }
                a { class: "lbl", "{props.label}" }
            }
            td { class: "col-md-1"
                a { class: "remove", onclick: {handler}
                    span { class: "glyphicon glyphicon-remove remove" aria_hidden: "true" }
                }
            }
            td { class: "col-md-6" }
        }
    })
}

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
impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.0[0], self.0[1], self.0[2])
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
