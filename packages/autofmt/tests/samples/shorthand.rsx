#[component]
fn SomePassthru(class: String, id: String, children: Element) -> Element {
    rsx! {
        div {
            // Comments
            class,
            "hello world"
        }
        h1 { class, "hello world" }

        h1 { class, {children} }

        h1 { class, id, {children} }

        h1 { class,
            "hello world"
            {children}
        }

        h1 { id,
            "hello world"
            {children}
        }

        Other { class, children }

        Other { class,
            "hello world"
            {children}
        }

        div {
            // My comment here 1
            // My comment here 2
            // My comment here 3
            // My comment here 4
            class: "asdasd",

            // Comment here
            onclick: move |_| {
                let a = 10;
                let b = 40;
                let c = 50;
            },

            // my comment

            // This here
            "hi"
        }

        // Comment head
        div { class: "asd", "Jon" }

        // Comment head
        div {
            // Collapse
            class: "asd",
            "Jon"
        }
    }
}
