use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let count: &RefCell<Vec<&Element>> = cx.use_hook(|| RefCell::new(Vec::new()));

    let element = render! {
        div {
            "hello world!"
        }
    };

    let nested_borrows = cx.use_hook(Vec::new);
    nested_borrows.push("hello world!");

    cx.render(rsx! {
        // unsafe_child_component {
        //     borrowed: count
        // }
        safe_child_component {
            borrowed: element
        }
        safe_nested_borrows {
            borrowed: &**nested_borrows
        }
    })
}

#[derive(Props)]
struct NestedBorrows<'a> {
    borrowed: &'a [&'a str],
}

fn safe_nested_borrows<'a>(cx: Scope<'a, NestedBorrows<'a>>) -> Element<'a> {
    render! {
        div { "{cx.props.borrowed:?}" }
    }
}

#[derive(Props)]
struct Testing<'a> {
    borrowed: &'a RefCell<Vec<&'a Element<'a>>>,
}

fn unsafe_child_component<'a>(cx: Scope<'a, Testing<'a>>) -> Element<'a> {
    let Testing { borrowed } = cx.props;
    let borrowed = borrowed.borrow();
    cx.render(rsx! {
        div { "Hello, world!" }
    })
}

#[derive(Props)]
struct SafeTesting<'a> {
    borrowed: Element<'a>,
}

fn safe_child_component<'a>(cx: Scope<'a, SafeTesting<'a>>) -> Element<'a> {
    let SafeTesting { borrowed } = cx.props;
    cx.render(rsx! {
        div {
            borrowed
        }
    })
}
