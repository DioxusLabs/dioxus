//! Signals can degrade into a Read variant automatically
//! This is done thanks to a conversion by the #[component] macro

use dioxus_core::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);

    loop {
        dom.render_immediate(&mut dioxus_core::dioxus_vdom::NoOpMutations);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        Child {
            count,
            "hiiii"
        }
    }
}

#[component]
fn Child(count: ReadSignal<i32>, children: Element) -> Element {
    rsx! {
        div { "Count: {count}" }
        {children}
    }
}
