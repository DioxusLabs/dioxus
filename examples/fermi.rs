#![allow(non_snake_case)]

use dioxus::prelude::*;
use fermi::*;

fn main() {
    dioxus_desktop::launch(app)
}

static NAME: Atom<String> = Atom(|_| "world".to_string());

fn app() -> Element {
    use_init_atom_root(cx);
    let name = use_read(&NAME);

    cx.render(rsx! {
        div { "hello {name}!" }
        Child {}
        ChildWithRef {}
    })
}

fn Child() -> Element {
    let set_name = use_set(&NAME);

    cx.render(rsx! {
        button {
            onclick: move |_| set_name("dioxus".to_string()),
            "reset name"
        }
    })
}

static NAMES: AtomRef<Vec<String>> = AtomRef(|_| vec!["world".to_string()]);

fn ChildWithRef() -> Element {
    let names = use_atom_ref(&NAMES);

    cx.render(rsx! {
        div {
            ul {
                for name in names.read().iter() {
                    li { "hello: {name}" }
                }
            }
            button {
                onclick: move |_| {
                    let names = names.clone();
                    cx.spawn(async move {
                        names.write().push("asd".to_string());
                    })
                },
                "Add name"
            }
        }
    })
}
