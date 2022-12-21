use dioxus::prelude::*;
use fermi::{use_atom_state, use_init_atom_root, use_read, use_selector, Atom, Readable, Select};

fn main() {
    dioxus_desktop::launch(app);
}

static AGE: Atom<i32> = |_| 42;

fn select_computed(root: Select) -> i32 {
    root.get(AGE) + 10
}

fn app(cx: Scope) -> Element {
    use_init_atom_root(cx);

    dbg!(AGE.unique_id());

    let val = use_read(cx, AGE);

    let computed = use_selector(cx, select_computed);

    cx.render(rsx! {
        div {
             "Val: {val}"
            //  "Computed: {computed}"
        }
    })
}

// static NAME: Atom<String> = |_| "hello world".to_string();
// static TITLE: Atom<String> = |_| "hello world".to_string();

// fn names(root: Select) -> Vec<&str> {
//     root.get(TITLE).split_ascii_whitespace().collect()
// }

// fn combo(root: Select) -> String {
//     let title = root.get(TITLE);
//     let age = root.get(AGE);

//     format!("{} is {} years old", title, age)
// }

// fn multi_layer(root: Select) -> Vec<String> {
//     let title = root.get(TITLE);
//     let age = root.get(AGE);
//     let names = root.select(names);

//     names
//         .iter()
//         .map(|name| format!("{title}: {} is {} years old", name, age))
//         .collect()
// }
