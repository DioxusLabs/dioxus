use dioxus::prelude::*;

fn main() {
    dioxus_tui::launch(app);
}

fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 0);

    use_future(cx, (), move |_| {
        let count = count.to_owned();
        let update = cx.schedule_update();
        async move {
            loop {
                count.with_mut(|f| *f += 1);
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                update();
            }
        }
    });

    cx.render(rsx! {
        div { width: "100%",
            div { width: "50%", height: "5px", background_color: "blue", justify_content: "center", align_items: "center",
                "Hello {count}!"
            }
            div { width: "50%", height: "10px", background_color: "red", justify_content: "center", align_items: "center",
                "Hello {count}!"
            }
        }
    })
}
