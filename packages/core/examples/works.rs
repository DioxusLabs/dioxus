#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {
    let _ = VirtualDom::new(parent);
}

fn parent(cx: Scope<()>) -> Element {
    let value = cx.use_hook(|_| String::new(), |f| f);

    cx.render(rsx! {
        div {
            child( name: value )
        }
    })
}

#[derive(Props)]
struct ChildProps<'a> {
    name: &'a str,
}

fn child<'a>(cx: Scope<'a, ChildProps<'a>>) -> Element {
    cx.render(rsx! {
        div {
            h1 { "it's nested" }
            grandchild( name: cx.props.name )
        }
    })
}

#[derive(Props)]
struct Grandchild<'a> {
    name: &'a str,
}

fn grandchild<'a>(cx: Scope<'a, Grandchild>) -> Element<'a> {
    cx.render(rsx! {
        div { "Hello {cx.props.name}!" }
        great_grandchild( name: cx.props.name )
    })
}

fn great_grandchild<'a>(cx: Scope<'a, Grandchild>) -> Element<'a> {
    cx.render(rsx! {
        div {
            h1 { "it's nested" }
        }
    })
}

/*
can we implement memoization as a wrapper or something? Like we just intercept the
render function?




*/
