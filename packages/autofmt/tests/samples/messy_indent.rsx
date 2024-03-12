fn SaveClipboard() -> Element {
    rsx! {
        div { class: "relative w-1/2 {align} max-w-md leading-8",
            h2 { class: "mb-6 text-3xl leading-tight md:text-4xl md:leading-tight lg:text-3xl lg:leading-tight font-heading font-mono font-bold",
                "{title}"
            }
        }
    };

    rsx! {
        div { "hello world", "hello world", "hello world" }
    }
}
