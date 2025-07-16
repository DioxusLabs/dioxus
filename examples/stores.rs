use dioxus::prelude::{dioxus_stores::Store, *};
use dioxus_stores::use_store;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let value = use_store(|| Value {
        count: Default::default(),
        values: vec![Value {
            count: Default::default(),
            values: Vec::new(),
        }],
    });

    rsx! {
        Tree {
            value
        }
    }
}

#[component]
fn Tree(#[props(into)] value: Store<Value>) -> Element {
    let count = value.count();
    use_effect(move || {
        // This effect will run whenever the value changes
        println!("Child component value changed: {}", count.count().read());
    });
    rsx! {
        h2 { "Child component with count {count.count().read()}" }
        button { onclick: move |_| *count.count().write() += 1, "Increment" }
        button { onclick: move |_| *count.count().write() -= 1, "Decrement" }
        button { onclick: move |_| value.values().push(Value{ count: Default::default(), values: Vec::new() }), "Push child" }
        ul {
            for child in value.values().iter() {
                li {
                    Tree { value: child }
                }
            }
        }
    }
}

#[derive(Store)]
struct Value {
    count: DoubleCount,
    values: Vec<Value>,
}

#[derive(Store, Default)]
struct DoubleCount {
    #[store(foreign)]
    count: i32,
}
