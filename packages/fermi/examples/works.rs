use dioxus::prelude::*;
use fermi::{atom, use_selector, Atom, Select, Selector};

static TITLE: Atom<String> = atom(|| "The Biggest Name in Hollywood".to_string());

fn names(root: Select) -> Vec<&str> {
    root.get(TITLE).split_ascii_whitespace().collect()
}

fn first_name(root: Select) -> Option<&str> {
    root.get(TITLE).split_ascii_whitespace().next()
}

fn app(cx: Scope) -> Element {
    let first_name = use_selector(&cx, first_name).unwrap();
    let names = use_selector(&cx, names);

    cx.render(rsx! {
        ul {
            "{first_name}"
            names.iter().map(|f| rsx! {
                li {"{f}"}
            })
        }
    })
}

fn main() {
    let mut dom = VirtualDom::new(app);

    let edits = dom.rebuild();

    dbg!(edits);
}
