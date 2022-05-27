use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    dioxus::desktop::launch_with_props(with_hot_reload, app, |b| b);
}

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);

    use_future(&cx, (), move |_| {
        let mut count = count.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                count += 1;
            }
        }
    });

    cx.render(rsx! {
        h1 {
            width: format!("{}px", count),
            "High-Five counter: {count}",
            Comp{
                color: "#083289"
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
