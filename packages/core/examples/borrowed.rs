use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {}

fn App(cx: Scope<()>) -> Element {
    cx.render(rsx!(div {
        App2 {
            p: "asd"
        }
    }))
}

#[derive(Props)]
struct Borrowed<'a> {
    p: &'a str,
}

fn App2<'a>(cx: Scope<'a, Borrowed<'a>>) -> Element {
    let g = eat2(&cx);
    todo!()
}

fn eat2(s: &ScopeState) {}

fn eat(f: &str) {}

fn bleat() {
    let blah = String::from("asd");
    eat(&blah);
}

// struct Lower {}

// #[derive(Clone, Copy)]
// struct Upper {}
// impl std::ops::Deref for Upper {
//     type Target = Lower;

//     fn deref(&self) -> &Self::Target {
//         todo!()
//     }
// }

// fn mark(f: &Lower) {}
// fn bark() {
//     let up = Upper {};
//     mark(&up);
// }
