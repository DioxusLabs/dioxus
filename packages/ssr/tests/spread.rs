use dioxus::prelude::*;

#[test]
fn spread() {
    let dom = VirtualDom::prebuilt(app);
    let html = dioxus_ssr::render(&dom);

    assert_eq!(
        html,
        r#"<audio data-custom-attribute="value" style="width:10px;height:10px;left:1;">1: hello1
2: hello2</audio>"#
    );
}

fn app() -> Element {
    rsx! {
        SpreadableComponent {
            width: "10px",
            extra_data: "hello{1}",
            extra_data2: "hello{2}",
            height: "10px",
            left: 1,
            "data-custom-attribute": "value",
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct Props {
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,

    extra_data: String,

    extra_data2: String,
}

#[component]
fn SpreadableComponent(props: Props) -> Element {
    rsx! {
        audio { ..props.attributes, "1: {props.extra_data}\n2: {props.extra_data2}" }
    }
}
