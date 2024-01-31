use dioxus::prelude::*;

fn main() {
    dioxus_tui::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    use_future(move || async move {
        loop {
            count += 1;
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            schedule_update();
        }
    });

    rsx! {
        div { width: "100%",
            div { width: "50%", height: "5px", background_color: "blue", justify_content: "center", align_items: "center",
                "Hello {count}!"
            }
            div { width: "50%", height: "10px", background_color: "red", justify_content: "center", align_items: "center",
                "Hello {count}!"
            }
        }
    }
}
