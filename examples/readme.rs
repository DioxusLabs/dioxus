use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        Child { count: "High-Five counter: {count}" }
        Child { count: "count" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ChildProps {
    count: Option<String>,
}

fn Child(props: ChildProps) -> Element {
    rsx! {
        h1 { "{props.count.unwrap_or_default()}" }
    }
}
