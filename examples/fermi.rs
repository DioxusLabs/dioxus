#![allow(non_snake_case)]

use dioxus::prelude::*;
use fermi::*;

fn main() {
    dioxus_desktop::launch(app)
}

static NAME: Atom<String> = Atom(|_| "world".to_string());

fn app(cx: Scope) -> Element {
    use_init_atom_root(cx);
    let name = use_read(cx, &NAME);

    cx.render(rsx! {
        div { "hello {name}!" }
        Child {}
        ChildWithRef {}
    })
}

fn Child(cx: Scope) -> Element {
    let set_name = use_set(cx, &NAME);

    cx.render(rsx! {
        button {
            onclick: move |_| set_name("dioxus".to_string()),
            "reset name"
        }
    })
}

static NAMES: AtomRef<Vec<String>> = AtomRef(|_| vec!["world".to_string()]);

fn ChildWithRef(cx: Scope) -> Element {
    let names = use_atom_ref(cx, &NAMES);

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
