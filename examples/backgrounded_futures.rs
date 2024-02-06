use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut show_child = use_signal(|| true);
    let mut count = use_signal(|| 0);

    let child = use_memo(move || {
        rsx! {
            Child {
                count
            }
        }
    });

    rsx! {
        button { onclick: move |_| show_child.toggle(), "Toggle child" }
        button { onclick: move |_| count += 1, "Increment count" }
        if show_child() {
            {child.cloned()}
        }
    }
}

#[component]
fn Child(count: Signal<i32>) -> Element {
    let mut early_return = use_signal(|| false);

    let early = rsx! {
        button { onclick: move |_| early_return.toggle(), "Toggle {early_return} early return" }
    };

    if early_return() {
        return early;
    }

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            println!("Child")
        }
    });

    use_effect(move || {
        println!("Child count: {}", count());
    });

    rsx! {
        "hellO!"
        {early}
    }
}
