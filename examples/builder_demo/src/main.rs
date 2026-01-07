use dioxus::prelude::*;
use dioxus_builder::*;
use dioxus_core::Attribute;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    use_hook(|| {
        let doc = document::document();
        doc.set_title("Dioxus Builder Demo".to_string());
        doc.create_link(
            document::LinkProps::builder()
                .rel(Some("stylesheet".to_string()))
                .href(Some(TAILWIND_CSS.to_string()))
                .build(),
        );
    });

    // head().title("Dioxus Builder Demo")

    rsx! {
        body_section { count }
    }
}

#[component]
fn body_section(count: Signal<i32>) -> Element {
    div()
        .class_list([
            "flex",
            "flex-col",
            "items-center",
            "justify-center",
            "min-h-screen",
            "bg-gray-100",
            "p-8",
            "space-y-6",
        ])
        .child(header_section())
        .child(counter_section(count))
        .child(list_section(count))
        .child(attribute_helpers_section(count))
        .child(footer_section())
        .build()
}

fn header_section() -> Element {
    div()
        .class_list(["container", "mx-auto", "p-4", "text-center", "space-y-2"])
        .child(
            h1().class("text-4xl font-bold text-blue-600")
                .child("Dioxus Builder Demo"),
        )
        .child(
            p().class("text-lg text-gray-700")
                .child("This UI is built using the typed builder API and Tailwind CSS."),
        )
        .build()
}

fn counter_section(mut count: Signal<i32>) -> Element {
    div()
        .class_list(["flex", "space-x-4", "items-center"])
        .child(
            button()
                .class("px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 transition")
                .onclick(move |_| count -= 1)
                .child("-"),
        )
        .child(
            span()
                .class("text-2xl font-mono w-12 text-center")
                .child(count.to_string()),
        )
        .child(
            button()
                .class("px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 transition")
                .onclick(move |_| count += 1)
                .child("+"),
        )
        .build()
}

fn list_section(count: Signal<i32>) -> Element {
    div()
        .class("mt-4 w-full max-w-md bg-white shadow-xl rounded-lg overflow-hidden")
        .child(
            div()
                .class("p-4 border-b bg-gray-50")
                .child(h2().class("font-semibold").child("Item List")),
        )
        .child(
            ul().class("divide-y divide-gray-200")
                .children((0..count()).map(|i| {
                    li().class("p-4 hover:bg-gray-50 flex justify-between")
                        .child(span().child(format!("Item record #{}", i + 1)))
                        .child(
                            span()
                                .class("text-xs text-gray-400 capitalize")
                                .child(if i % 2 == 0 { "Even" } else { "Odd" }),
                        )
                })),
        )
        .build()
}

fn attribute_helpers_section(count: Signal<i32>) -> Element {
    let is_even = count() % 2 == 0;
    let extra_attrs = [
        Attribute::new("data-role", "builder-demo", None, false),
        Attribute::new("data-count", count().to_string(), None, false),
    ];

    div()
        .class_list([
            "mt-4",
            "w-full",
            "max-w-md",
            "bg-white",
            "shadow",
            "rounded-lg",
            "p-4",
            "space-y-3",
            "border",
        ])
        .class_if(is_even, "border-green-300")
        .class_if(!is_even, "border-amber-300")
        .attr_if(is_even, "data-state", "even")
        .attrs(extra_attrs)
        .child(h2().class("font-semibold").child("Attribute Helpers"))
        .child(
            p().class("text-sm text-gray-600")
                .child("Uses class_list, class_if, attr_if, and attrs()."),
        )
        .child_if(
            is_even,
            p().class("text-sm text-green-600")
                .child("child_if: count is even"),
        )
        .child_if_else(
            is_even,
            p().class("text-xs text-gray-400")
                .child("child_if_else: even branch"),
            p().class("text-xs text-gray-400")
                .child("child_if_else: odd branch"),
        )
        .build()
}

fn footer_section() -> Element {
    footer()
        .class("mt-8 text-gray-400 text-sm")
        .child("Built with dioxus-builder")
        .build()
}
