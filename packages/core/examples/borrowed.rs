use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(div {
        app2(
            p: "asd"
        )
    }))
}

#[derive(Props)]
struct Borrowed<'a> {
    p: &'a str,
}

fn app2<'a>(cx: Scope<'a, Borrowed<'a>>) -> Element {
    let g = eat2(&cx);
    rsx!(cx, "")
}

fn eat2(s: &ScopeState) {}

fn eat(f: &str) {}

fn bleat() {
    let blah = String::from("asd");
    eat(&blah);
}
