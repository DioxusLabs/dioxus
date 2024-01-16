use dioxus::prelude::*;

fn main() {
    let mut dom = VirtualDom::prebuilt(app);
    let html = dioxus_ssr::render(&dom);

    println!("{}", html);
}

fn app() -> Element {
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
fn Component(props: Props) -> Element {
    render! {
        audio { ..props.attributes, "1: {props.extra_data}\n2: {props.extra_data2}" }
    }
}

#[derive(Props, PartialEq, Clone)]
struct Props {
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,

    extra_data: String,
    extra_data2: String,
}
