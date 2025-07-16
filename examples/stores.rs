use dioxus::prelude::*;
use dioxus_stores::use_store;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let value = use_store(|| Value {
        count: 0,
        values: vec![Value {
            count: 0,
            values: Vec::new(),
        }],
    });

    let mut count = value().count();
    let values = value().values();

    use_effect(move || {
        // This effect will run whenever the value changes
        println!("App value changed: {}", count.read());
    });

    rsx! {
        h1 { "Counter App {count.cloned()}" }
        button { onclick: move |_| *count.write() += 1, "Up high!" }
        button { onclick: move |_| *count.write() -= 1, "Down low!" }

        button { onclick: move |_| values.push(Value{ count: 0, values: Vec::new() }), "Push child" }

        for child in values.iter() {
            Child {
                value: WriteSignal::new(child.count()),
            }
        }
    }
}

#[component]
fn Child(value: WriteSignal<u32>) -> Element {
    use_effect(move || {
        // This effect will run whenever the value changes
        println!("Child component value changed: {}", value.read());
    });
    rsx! {
        h2 { "Child component with count {value}" }
        button { onclick: move |_| value += 1, "Increment" }
        button { onclick: move |_| value -= 1, "Decrement" }
    }
}

#[derive(Store)]
struct Value {
    #[store(foreign)]
    count: u32,
    values: Vec<Value>,
}
