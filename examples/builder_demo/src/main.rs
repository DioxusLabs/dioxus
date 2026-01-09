use bon::Builder;
use dioxus::prelude::*;
use dioxus_builder::FunctionComponent;
use dioxus_builder::*;
use dioxus_core::Properties;

// Minimal, type-safe builder pattern with a small counter app.

#[derive(Builder, Clone, PartialEq)]
struct MyComponentProps {
    #[builder(into)]
    title: String,
    count: Signal<i32>,
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

impl<S> dioxus_core::IntoDynNode for MyComponentPropsBuilder<S>
where
    S: my_component_props_builder::IsComplete,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        dioxus_core::IntoDynNode::into_dyn_node(MyCoolComponent(self.build()))
    }
}

#[allow(non_snake_case)]
fn MyCoolComponent(props: MyComponentProps) -> Element {
    div()
        .class("p-4 rounded-lg border border-gray-200 bg-white shadow-sm")
        .child(
            h2().class("text-lg font-semibold text-gray-800")
                .text(props.title),
        )
        .child(
            p().class("text-sm text-gray-600")
                .text(format!("Count is {}", props.count)),
        )
        .build()
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    div()
        .class_list([
            "min-h-screen",
            "bg-gray-100",
            "flex",
            "items-center",
            "justify-center",
            "p-8",
        ])
        .child(
            div()
                .class("space-y-4")
                .child(
                    MyCoolComponent
                        .new()
                        .title("Builder + bon props")
                        .count(count),
                )
                .child(
                    div()
                        .class("flex items-center gap-3")
                        .child(
                            button()
                                .class("px-3 py-2 rounded bg-red-500 text-white")
                                .onclick(move |_| count -= 1)
                                .text("-"),
                        )
                        .child(
                            span()
                                .class("text-lg font-mono text-gray-800")
                                .text(count.to_string()),
                        )
                        .child(
                            button()
                                .class("px-3 py-2 rounded bg-green-500 text-white")
                                .onclick(move |_| count += 1)
                                .text("+"),
                        ),
                ),
        )
        .build()
}

fn main() {
    dioxus::launch(app);
}
