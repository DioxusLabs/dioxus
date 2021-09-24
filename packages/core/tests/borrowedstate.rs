use dioxus::{nodes::VSuspended, prelude::*, DomEdit, TestDom};
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;

static Parent: FC<()> = |cx, props| {
    let value = cx.use_hook(|_| String::new(), |f| &*f, |_| {});

    cx.render(rsx! {
        div {
            Child { name: value }
            Child { name: value }
            Child { name: value }
            Child { name: value }
        }
    })
};

#[derive(Props)]
struct ChildProps<'a> {
    name: &'a String,
}

fn Child<'a>(cx: Context<'a>, props: &'a ChildProps) -> DomTree<'a> {
    cx.render(rsx! {
        div {
            h1 { "it's nested" }
            Child2 { name: props.name }
        }
    })
}

#[derive(Props)]
struct Grandchild<'a> {
    name: &'a String,
}

fn Child2<'a>(cx: Context<'a>, props: &Grandchild) -> DomTree<'a> {
    cx.render(rsx! {
        div { "Hello {props.name}!" }
    })
}

#[test]
fn test_borrowed_state() {}
