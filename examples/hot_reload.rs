use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    dioxus::desktop::launch_with_props(with_hot_reload, app, |b| b);
}

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 170);

    use_future(&cx, (), move |_| {
        let mut count = count.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                count += 1;
            }
        }
    });

    cx.render(rsx! {
        div {
            width: format!("{}px", count),
            background_color: "#999999",
            onclick: move |_| {
                count.modify(|count| *count + 1);
            },
            "High-Five counter: {count}",
            Comp{
                color: "#083289"
            }
            Comp{
                color: "green"
            }
        }
    })
}

#[derive(PartialEq, Props)]
struct CompProps {
    color: &'static str,
}

fn Comp(cx: Scope<CompProps>) -> Element {
    cx.render(rsx! {
        h1 {
            color: cx.props.color,
            "Hello, from a component!"
        }
    })
}
