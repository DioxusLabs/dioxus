

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            div {
                key: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
                class: "asdasd",
            }
        }
        h1 {"hi"}
        h1 {"hi"}
        h1 {"hi"}
        h1 {"hi"}
  div {
        div {
            key: "ddd",
            class: "asd",
            class: "asd",
            class: "asd",
            class: "asd",
            class: "asd",
            class: "asd",
            blah: 123,
            onclick: move |_| {
                let blah = 120;
                true
            },
            onclick: move |_| {
                let blah = 120;
                true
            },
            onclick: move |_| {
                let blah = 120;
                true
            },
            onclick: move |_| {
                let blah = 120;
                true
            },
            div {
                div { "hi" }
                h2 { class: "asd" }
            }
            Component {}

            // Generics
            Component<Generic> {}
        }
    }

    div { adsasd: "asd",
        h1 { "asd" }
        div {
            div { "hello" }
            div { "goodbye" }
            div { class: "broccoli", div { "hi" } }
            div { class: "broccolibroccolibroccolibroccolibroccolibroccolibroccolibroccolibroccolibroccoli",
                div { "hi" }
            }
            div {
                class: "alksdjasd",
                onclick: move |_| {
                    liberty!();
                },
                div { "hi" }
            }
            commented {
                // is unparalled
                class: "asdasd",

                // My genius
                div { "hi" }

                div {}
            }
        }
    }


    // Components
    Component {
        adsasd: "asd",

        // this is a comment
        onclick: move |_| {
            let blah = 120;
            let blah = 122;
        }
    }

    div {
        Component {
            adsasd: "asd",
            onclick: move |_| {
                let a = a;
            },
            div { "thing" }
        }
        Component {
            asdasd: "asdasd",
            asdasd: "asdasdasdasdasdasdasdasdasdasd",
            ..Props {
                a: 10,
                b: 20
            }
        }
        Component {
            asdasd: "asdasd",
            ..Props {
                a: 10,
                b: 20,
                c: {
                    fn main() {}
                },
            }
            "content"
        }
    }

    div {
        a: "1234567891012345678910123456789101234567891012345678910123456789101234567891012345678910123456789101234567891012345678910",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123"
    }

    // Short attributes
    div { a: "123", a: "123", a: "123", a: "123", a: "123", a: "123", a: "123", a: "123", a: "123" }

    // Compression
    h3 { class: "mb-2 text-xl font-bold", "Invite Member" }
    a { class: "text-white", "Send invitation" }

    // Props on tops
    h3 { class: "mb-2 text-xl font-bold mb-2 text-xl font-bold mb-2 text-xl font-bold mb-2 text-xl font-bold mb-2 text-xl font-bold",
        "Invite Member"
    }

    // No children, minimal props
    img { class: "mb-6 mx-auto h-24", src: "artemis-assets/images/friends.png", alt: "" }

    // One level compression
    div {
        a { class: "py-2 px-3 bg-indigo-500 hover:bg-indigo-600 rounded text-xs text-white", href: "#", "Send invitation" }
    }

    // Tiny component
    Component { a: 123 }

    // Expressions
    ul {
        div {}
        (0..10).map(|f| rsx! {
            li { "hi" }
        })
        div {}
    }

    // Complex nesting with components
    button {
        class: "flex items-center pl-3 py-3 pr-2 text-gray-500 hover:bg-indigo-50 rounded",
        onclick: move |evt| {
            show_user_menu.set(!show_user_menu.get());
            evt.cancel_bubble();
        },
        onclick: move |evt| show_user_menu.set(!show_user_menu.get()),
        span { class: "inline-block mr-4", icons::icon_14 {} }
        span { "Settings" }
    }

    // Complex nesting with handlers
    li {
        Link { class: "flex items-center pl-3 py-3 pr-4 {active_class} rounded", to: "{to}",
            span { class: "inline-block mr-3", icons::icon_0 {} }
            span { "{name}" }
            children.is_some().then(|| rsx! {
                span {
                    class: "inline-block ml-auto hover:bg-gray-500",
                    onclick: move |evt| {
                        // open.set(!open.get());
                        evt.cancel_bubble();
                    },
                    icons::icon_8 {}
                }
            })
        }
        div { class: "px-4",
            is_current.then(|| rsx!{ children })
        }
    }

    // No nesting
    Component {
        adsasd: "asd",
        onclick: move |_| {
            let blah = 120;
        }
    }

    // Component path
    my::thing::Component {
        adsasd: "asd",
        onclick: move |_| {
            let blah = 120;
        }
    }

    })
}
