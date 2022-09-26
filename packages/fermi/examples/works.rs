use dioxus::prelude::*;
use fermi::{use_selector, Atom, Select};

static AGE: Atom<i32> = || 42;
static NAME: Atom<String> = || "hello world".to_string();
static TITLE: Atom<String> = || "hello world".to_string();

fn names(root: Select) -> Vec<&str> {
    root.get(TITLE).split_ascii_whitespace().collect()
}

fn combo(root: Select) -> String {
    let title = root.get(TITLE);
    let age = root.get(AGE);

    format!("{} is {} years old", title, age)
}

fn multi_layer(root: Select) -> Vec<String> {
    let title = root.get(TITLE);
    let age = root.get(AGE);
    let names = root.select(names);

    names
        .iter()
        .map(|name| format!("{title}: {} is {} years old", name, age))
        .collect()
}

fn app(cx: Scope) -> Element {
    let age = use_atom(cx, AGE);
    let names = use_selector(&cx, names);
    let header = use_selector(&cx, combo);

    cx.render(rsx! {
        ul {
            "{header}"
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
