use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        // Use Map directly to lazily pull elements
        (0..10).map(|f| rsx! { "{f}" }),
        // Collect into an intermediate collection if necessary
        ["a", "b", "c"]
            .into_iter()
            .map(|f| rsx! { "{f}" })
            .collect::<Vec<_>>(),
        // Use optionals
        Some(rsx! { "Some" }),
    ))
}
