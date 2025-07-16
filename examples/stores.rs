use dioxus::prelude::{dioxus_stores::Store, *};
use dioxus_stores::use_store;

fn main() {
    dioxus::launch(app);
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
fn Tree(value: Store<Value>) -> Element {
    rsx! {
        Counter { count: value.count().count().boxed_mut() }
        button { onclick: move |_| value.values().push(Value{ count: Default::default(), values: Vec::new() }), "Push child" }
        ul {
            for child in value.values().into_iter() {
                li {
                    Tree { value: child }
                }
            }
        }
    }
}

#[component]
fn Counter(count: WriteSignal<i32>) -> Element {
    println!("Child counter run: {}", count);

    rsx! {
        h2 { "Child component with count {count}" }
        button { onclick: move |_| count += 1, "Increment" }
        button { onclick: move |_| count -= 1, "Decrement" }
    }
}
