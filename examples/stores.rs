use std::fmt::Display;

use dioxus::prelude::{dioxus_stores::Selector, *};
use dioxus_stores::use_store;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let value: dioxus_stores::Store<Value<i32>> = use_store(|| Value {
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
                value: child,
            }
        }
    }
}

#[component]
fn Child(#[props(into)] value: Selector<Value<i32>>) -> Element {
    let mut count = value.count();
    use_effect(move || {
        // This effect will run whenever the value changes
        println!("Child component value changed: {}", count.read());
    });
    rsx! {
        h2 { "Child component with count {count.read()}" }
        button { onclick: move |_| *count.write() += 1, "Increment" }
        button { onclick: move |_| *count.write() -= 1, "Decrement" }
    }
}

#[derive(Store)]
struct Value<D> {
    #[store(foreign)]
    count: D,
    values: Vec<Value<D>>,
}

impl<D, V, F, FMut> From<ValueSelector<D, MappedMutSignal<Value<D>, V, F, FMut>>>
    for ValueSelector<D, WriteSignal<Value<D>>>
where
    V: Writable<Storage = UnsyncStorage> + 'static,
    F: Fn(&V::Target) -> &Value<D> + 'static,
    FMut: Fn(&mut V::Target) -> &mut Value<D> + 'static,
{
    fn from(value: ValueSelector<D, MappedMutSignal<Value<D>, V, F, FMut>>) -> Self {
        ValueSelector {
            selector: value.selector.map(|s| s.into()),
            _phantom: std::marker::PhantomData,
        }
    }
}
