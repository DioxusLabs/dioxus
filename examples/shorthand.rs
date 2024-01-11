use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let a = 123;
    let b = 456;
    let c = 789;
    let class = "class";
    let id = "id";

    // todo: i'd like it for children on elements to be inferred as the children of the element
    // also should shorthands understand references/dereferences?
    // ie **a, *a, &a, &mut a, etc
    let children = render! { "Child" };
    let onclick = move |_| println!("Clicked!");

    render! {
        div { class, id, {&children} }
        Component { a, b, c, children, onclick }
        Component { a, ..ComponentProps { a: 1, b: 2, c: 3, children: None, onclick: Default::default() } }
    }
}

#[component]
fn Component<'a>(
    cx: Scope<'a>,
    a: i32,
    b: i32,
    c: i32,
    children: Element<'a>,
    onclick: EventHandler<'a, ()>,
) -> Element {
    render! {
        div { "{a}" }
        div { "{b}" }
        div { "{c}" }
        div { {children} }
        div {
            onclick: move |_| onclick.call(()),
        }
    }
}
