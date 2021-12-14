#![allow(non_snake_case, non_upper_case_globals)]
//! This benchmark tests just the overhead of Dioxus itself.
//!
//! For the JS Framework Benchmark, both the framework and the browser is benchmarked together. Dioxus prepares changes
//! to be made, but the change application phase will be just as performant as the vanilla wasm_bindgen code. In essence,
//! we are measuring the overhead of Dioxus, not the performance of the "apply" phase.
//!
//! On my MBP 2019:
//! - Dioxus takes 3ms to create 1_000 rows
//! - Dioxus takes 30ms to create 10_000 rows
//!
//! As pure "overhead", these are amazing good numbers, mostly slowed down by hitting the global allocator.
//! These numbers don't represent Dioxus with the heuristic engine installed, so I assume it'll be even faster.

use criterion::{criterion_group, criterion_main, Criterion};
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use rand::prelude::*;

fn main() {
    static App: Component<()> = |cx, _| {
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
    };

    let mut dom = VirtualDom::new(App);
    let g = dom.rebuild();
    assert!(g.edits.len() > 1);
}

#[derive(PartialEq, Props)]
struct RowProps {
    row_id: usize,
    label: Label,
}
fn Row(cx: Scope, props: &RowProps) -> Element {
    let [adj, col, noun] = props.label.0;
    cx.render(rsx! {
        tr {
            td { class:"col-md-1", "{props.row_id}" }
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
