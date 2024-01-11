use dioxus::prelude::*;

fn main() {}

fn app() -> Element {
    let count: &RefCell<Vec<Element>> = cx.use_hook(|| RefCell::new(Vec::new()));

    render! {
        unsafe_child_component {
            borrowed: count
        }
    }
}

#[derive(Props)]
struct Testing<'a> {
    borrowed: &'a RefCell<Vec<Element>>,
}

fn unsafe_child_component<'a>(cx: Testing) -> Element {
    let Testing { borrowed } = cx;
    let borrowed_temporary_data =
        cx.use_hook(|| String::from("This data is only valid for the lifetime of the child"));

    borrowed
        .borrow_mut()
        .push(render! {"{borrowed_temporary_data}"});

    cx.render(rsx! {
        div { "Hello, world!" }
    })
}
