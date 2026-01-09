use bon::Builder;
use dioxus::prelude::*;
use dioxus_builder::FunctionComponent;
use dioxus_core::Properties;

// Minimal, fully type-safe component + props builder example.
// This mirrors the "Props + FunctionComponent" pattern using Dioxus types.

#[derive(Builder, Clone, PartialEq)]
struct MyComponentProps {
    title: String,
    count: u32,
}

impl Properties for MyComponentProps {
    type Builder = MyComponentPropsBuilder;

    fn builder() -> Self::Builder {
        MyComponentProps::builder()
    }

    fn memoize(&mut self, other: &Self) -> bool {
        self == other
    }
}

#[allow(non_snake_case)]
fn MyCoolComponent(props: MyComponentProps) -> Element {
    println!("Title: {}, Count: {}", props.title, props.count);
    VNode::empty()
}

fn it_works() {
    let _props = MyCoolComponent
        .new()
        .title("Hello World".to_string())
        .count(42)
        .build();
}

fn main() {
    it_works();
}
