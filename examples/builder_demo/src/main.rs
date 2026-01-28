use dioxus::prelude::*;
// Minimal, type-safe builder pattern with a small counter app.

#[component(builder)]
fn CounterCard(#[props(into)] title: String, count: Signal<i32>) -> Element {
    div()
        .class("p-4 rounded-lg border border-gray-200 bg-white shadow-sm")
        .aria_label("Counter component")
        .pipe(|builder| data!(builder, "testid", "counter-card"))
        .child(
            div()
                .class("flex items-center text-gray-800")
                .style_prop("gap", "0.5rem")
                .child(
                    svg()
                        .class("h-5 w-5 text-gray-500")
                        .viewBox("0 0 24 24")
                        .fill("none")
                        .stroke("currentColor")
                        .stroke_width("2")
                        .aria_hidden(true)
                        .child(path().d("M12 6v6l4 2")),
                )
                .child(h2().class("text-lg font-semibold").text(title)),
        )
        .child(
            p().class("text-sm text-gray-600")
                .text(format!("Count is {}", count())),
        )
        .build()
}

fn button_base(builder: ElementBuilder) -> ElementBuilder {
    builder.class("px-3 py-2 rounded font-medium transition-colors")
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    div()
        .class_list([
            "min-h-screen",
            "bg-gray-100",
            "flex",
            "items-center",
            "justify-center",
            "p-8",
        ])
        .child(
            div()
                .class("space-y-4")
                .child(
                    CounterCard
                        .new()
                        .title("Builder + derive macro")
                        .count(count),
                )
                .child(
                    div()
                        .class("flex items-center gap-3")
                        .aria_label("Counter controls")
                        .pipe(|builder| data!(builder, "testid", "counter-controls"))
                        .child(
                            button()
                                .with(button_base)
                                .class("bg-red-500 text-white hover:bg-red-600")
                                .aria_label("Decrement")
                                .onclick(move |_| count -= 1)
                                .text("-"),
                        )
                        .child(
                            span()
                                .class("text-lg font-mono text-gray-800")
                                .aria_live("polite")
                                .text(count().to_string()),
                        )
                        .child(
                            button()
                                .with(button_base)
                                .class("bg-green-500 text-white hover:bg-green-600")
                                .aria_label("Increment")
                                .onclick(move |_| count += 1)
                                .text("+"),
                        ),
                ),
        )
        .build()
}

fn main() {
    dioxus::launch(app);
}
