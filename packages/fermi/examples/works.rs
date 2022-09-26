use dioxus::prelude::*;
use fermi::{use_atom_state, use_selector, Atom, Select};

static AGE: Atom<i32> = |_| 42;
static NAME: Atom<String> = |_| "hello world".to_string();
static TITLE: Atom<String> = |_| "hello world".to_string();

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
    let names = use_selector(&cx, names);
    let header = use_selector(&cx, combo);
    let multi = use_selector(&cx, multi_layer);

    let mut age = use_atom_state(&cx, AGE);

    cx.render(rsx! {
        ul {
            "{header}"
            button {
                onclick: move |_| age += 1,
                "Increment age"
            }
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
