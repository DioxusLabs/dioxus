use dioxus::prelude::*;
use rand::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);

    _ = dom.rebuild();

    for _ in 0..1000 {
        _ = dom.rebuild();
    }
}

fn app(cx: Scope) -> Element {
    let mut rng = SmallRng::from_entropy();

    cx.render(rsx! (
        table {
            tbody {
                for f in 0..10_000_usize {
                    table_row {
                        row_id: f,
                        label: Label::new(&mut rng)
                    }
                }
            }
        }
    ))
}

#[derive(PartialEq, Props)]
struct RowProps {
    row_id: usize,
    label: Label,
}
fn table_row(cx: Scope<RowProps>) -> Element {
    let [adj, col, noun] = cx.props.label.0;
    cx.render(rsx! {
        tr {
            td { class:"col-md-1", "{cx.props.row_id}" }
            td { class:"col-md-1", onclick: move |_| { /* run onselect */ },
                a { class: "lbl", "{adj}" "{col}" "{noun}" }
            }
            td { class: "col-md-1",
                a { class: "remove", onclick: move |_| {/* remove */},
                    span { class: "glyphicon glyphicon-remove remove", aria_hidden: "true" }
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
