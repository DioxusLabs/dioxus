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

    // You can manually specify the dependencies with `use_reactive` for values that are not reactive like props
    let computed = use_memo(use_reactive!(
        |(non_reactive_prop,)| non_reactive_prop + signal()
    ));
    use_effect(use_reactive!(|(non_reactive_prop,)| println!(
        "{}",
        non_reactive_prop + signal()
    )));
    let fut = use_resource(use_reactive!(|(non_reactive_prop,)| async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        non_reactive_prop + signal()
    }));

    rsx! {
        button {
            onclick: move |_| signal += 1,
            "Child count: {signal}"
        }

        "Sum: {computed}"

        "{fut():?}"
    }
}
