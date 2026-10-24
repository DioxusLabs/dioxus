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
    // Both forwarded spreads carry width and height through the component chain
    // to both divs (the #3844 regression). The dynamic-attribute slot is
    // normalized by (name, namespace), so the style declarations are sorted.
    assert_eq!(
        html,
        r#"<div style="height:100%;width:100%;"></div><div style="height:100%;width:100%;"></div>"#
    );
}
