use dioxus::prelude::*;

// Regression test for https://github.com/DioxusLabs/dioxus/issues/3844
#[test]
fn forward_spreads() {
    #[derive(Props, Clone, PartialEq)]
    struct Comp1Props {
        #[props(extends = GlobalAttributes)]
        attributes: Vec<Attribute>,
    }

    #[component]
    fn Comp1(props: Comp1Props) -> Element {
        rsx! {
            Comp2 {
                attributes: props.attributes.clone(),
                height: "100%",
            }
            Comp2 {
                height: "100%",
                attributes: props.attributes.clone(),
            }
        }
    }

    #[derive(Props, Clone, PartialEq)]
    struct CompProps2 {
        #[props(extends = GlobalAttributes)]
        attributes: Vec<Attribute>,
    }

    #[component]
    fn Comp2(props: CompProps2) -> Element {
        let attributes = props.attributes;
        rsx! {
            div {
                ..attributes
            }
        }
    }

    let merged = || {
        rsx! {
            Comp1 {
                width: "100%"
            }
        }
    };
    let dom = VirtualDom::prebuilt(merged);
    let html = dioxus_ssr::render(&dom);
    assert_eq!(
        html,
        r#"<div style="width:100%;height:100%;"></div><div style="width:100%;height:100%;"></div>"#
    );
}
