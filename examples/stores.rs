use std::fmt::Display;

use dioxus::prelude::{dioxus_stores::*, *};
use dioxus_stores::use_store;

fn main() {
    dioxus::launch(app);
}

#[derive(Store)]
struct Value {
    count: i32,
    values: Vec<Value>,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Value(count: {}, values: {})",
            self.count,
            self.values.len()
        )
    }
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
        Counter { count: value.count() }
        button { onclick: move |_| value.values().push(Value { count: Default::default(), values: Vec::new() }), "Push child" }
        ul {
            for child in value.values().iter() {
                li {
                    Tree { value: child }
                }
            }
        }
    }
}

#[component]
fn Counter(count: Store<i32>) -> Element {
    println!("Child counter run: {}", count);

    rsx! {
        h2 { "Child component with count {count}" }
        button { onclick: move |_| count += 1, "Increment" }
        button { onclick: move |_| count -= 1, "Decrement" }
    }
}
