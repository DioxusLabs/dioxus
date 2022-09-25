use dioxus::prelude::*;
use fermi::{atom, use_selector, Atom, Select};

static TITLE: Atom<String> = atom(|| "The Biggest Name in Hollywood".to_string());

fn names(root: Select) -> Vec<&str> {
    root.get(TITLE).split_ascii_whitespace().collect()
}

fn app(cx: Scope) -> Element {
    let names = use_selector(&cx, names);

    cx.render(rsx! {
        ul {
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
