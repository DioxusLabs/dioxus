use dioxus::{events::MouseEvent, prelude::*};
use fxhash::FxBuildHasher;
use std::rc::Rc;

fn main() {
    dioxus::desktop::launch(App, |c| c);
}

// We use a special immutable hashmap to make hashmap operations efficient
type RowList = im_rc::HashMap<usize, Rc<str>, FxBuildHasher>;

static App: FC<()> = |cx, _props| {
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
                            ActionButton { name: "Create 1,000 rows", id: "run", onclick: {create_rendered_rows(0, 1_000)} }
                            ActionButton { name: "Create 10,000 rows", id: "runlots", onclick: {create_rendered_rows(0, 10_000)} }
                            ActionButton { name: "Append 1,000 rows", id: "add", onclick: {append_1_000_rows} }
                            ActionButton { name: "Update every 10th row", id: "update", onclick: {update_every_10th_row} }
                            ActionButton { name: "Clear", id: "clear", onclick: {clear_rows} }
                            ActionButton { name: "Swap rows", id: "swaprows", onclick: {swap_rows} }
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

#[derive(Props)]
struct ActionButtonProps<'a> {
    name: &'static str,
    id: &'static str,
    onclick: &'a dyn Fn(MouseEvent),
}

fn ActionButton<'a>(cx: Context<'a>, props: &'a ActionButtonProps) -> DomTree<'a> {
    rsx!(cx, div { class: "col-sm-6 smallpad"
        button { class:"btn btn-primary btn-block", r#type: "button", id: "{props.id}",  onclick: {props.onclick},
            "{props.name}"
        }
    })
}

#[derive(PartialEq, Props)]
struct RowProps {
    row_id: usize,
    label: Rc<str>,
}
fn Row<'a>(cx: Context<'a>, props: &'a RowProps) -> DomTree<'a> {
    rsx!(cx, tr {
        td { class:"col-md-1", "{props.row_id}" }
        td { class:"col-md-1", onclick: move |_| { /* run onselect */ }
            a { class: "lbl", "{props.label}" }
        }
        td { class: "col-md-1"
            a { class: "remove", onclick: move |_| {/* remove */}
                span { class: "glyphicon glyphicon-remove remove" aria_hidden: "true" }
            }
        }
        td { class: "col-md-6" }
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
