use dioxus::prelude::{dioxus_stores::Selector, *};
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

    rsx! {
        Tree {
            value
        }
    }
}

#[component]
fn Tree(#[props(into)] value: Selector<Value<i32>>) -> Element {
    let mut count = value.count();
    use_effect(move || {
        // This effect will run whenever the value changes
        println!("Child component value changed: {}", count.read());
    });
    rsx! {
        h2 { "Child component with count {count.read()}" }
        button { onclick: move |_| *count.write() += 1, "Increment" }
        button { onclick: move |_| *count.write() -= 1, "Decrement" }
        button { onclick: move |_| value.values().push(Value{ count: 0, values: Vec::new() }), "Push child" }
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
struct Value<D> {
    #[store(foreign)]
    count: D,
    values: Vec<Value<D>>,
}
