use dioxus::prelude::*;
use dioxus_builder::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    div()
        .class("flex flex-col items-center justify-center min-h-screen bg-gray-100 p-8 space-y-4")
        .child(
            h1()
                .class("text-4xl font-bold text-blue-600")
                .child("Dioxus Builder Demo with Hotpatching")
        )
        .child(
            p()
                .class("text-lg text-gray-700")
                .child("This UI is built using the typed builder API.")
        )
        .child(test())
        .child(
            div()
                .class("flex space-x-4 items-center")
                .child(
                    button()
                        .class("px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 transition")
                        .onclick(move |_| count -= 1)
                        .child("-")
                )
                .child(
                    span()
                        .class("text-2xl font-mono w-12 text-center")
                        .child(count.to_string())
                )
                .child(
                    button()
                        .class("px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 transition")
                        .onclick(move |_| count += 1)
                        .child("+")
                )
        )
        .child(
            div()
                .class("mt-8 w-full max-w-md bg-white shadow-xl rounded-lg overflow-hidden")
                .child(
                    div()
                        .class("p-4 border-b bg-gray-50")
                        .child(h2().class("font-semibold").child("Item List"))
                )
                .child(
                    ul()
                        .class("divide-y divide-gray-200")
                        .children((0..count()).map(|i| {
                            li()
                                .class("p-4 hover:bg-gray-50 flex justify-between")
                                .child(span().child(format!("Item record #{}", i + 1)))
                                .child(
                                    span()
                                        .class("text-xs text-gray-400 capitalize")
                                        .child(if i % 2 == 0 { "Even" } else { "Odd" })
                                )
                        }))
                )
        )
        .child(
            footer()
                .class("mt-12 text-gray-400 text-sm")
                .child("Built with dioxus-builder")
        )
        .build()
}

fn test() -> Element {
    div()
        .class("container mx-auto p-4")
        .child(
            h1().class("text-3xl font-bold mb-4")
                .child("Hello, Dioxus Builder!"),
        )
        .child(
            button()
                .class("bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded")
                .onclick(|_| println!("Button clicked!"))
                .child("Click Me"),
        )
        .build()
}
