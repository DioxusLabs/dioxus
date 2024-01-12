use dioxus::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);
    let _ = dom.rebuild();
    let html = dioxus_ssr::render(&dom);

    println!("{}", html);
}

fn app(cx: Scope) -> Element {
    render! {
        Component {
            width: "10px",
            extra_data: "hello{1}",
            extra_data2: "hello{2}",
            height: "10px",
            left: 1
        }
    }
}

#[component]
fn Component<'a>(cx: Scope<'a, Props<'a>>) -> Element<'a> {
    render! {
        audio { ..cx.props.attributes, "1: {cx.props.extra_data}\n2: {cx.props.extra_data2}" }
    }
}

#[derive(Props)]
struct Props<'a> {
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute<'a>>,
    extra_data: &'a str,
    extra_data2: &'a str,
}
