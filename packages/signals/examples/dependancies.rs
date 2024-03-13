use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut signal = use_signal(|| 0);

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            signal += 1;
        }
    });

    rsx! {
        "Parent count: {signal}"
        Child {
            non_reactive_prop: signal()
        }
    }
}

#[component]
fn Child(non_reactive_prop: i32) -> Element {
    let mut signal = use_signal(|| 0);

    // You can manually specify the dependencies with `use_dependencies` for values that are not reactive like props
    let computed =
        use_memo(move || non_reactive_prop + signal()).use_dependencies((&non_reactive_prop,));

    rsx! {
        button {
            onclick: move |_| signal += 1,
            "Child count: {signal}"
        }

        "Sum: {computed}"
    }
}
