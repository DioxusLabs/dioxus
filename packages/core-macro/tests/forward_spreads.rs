use dioxus::prelude::*;
use dioxus_core::ElementId;
use std::{any::Any, rc::Rc};

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
                attributes: props.attributes,
                height: "100%",
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
        rsx! {}
    }

    rsx! {
        Comp1 {
            width: "100%"
        }
    };
}
