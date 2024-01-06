rsx! {
    div { "hello world!" }
    div { "hello world!", "goodbye world!" }

    // Simple div
    div { "hello world!" }

    // Compression with attributes
    div { key: "a", class: "ban", style: "color: red" }

    // Nested one level
    div { div { "nested" } }

    // Nested two level
    div {
        div { h1 { "highly nested" } }
    }

    // Anti-Nested two level
    div {
        div {
            div { h1 { "highly nested" } }
        }
    }

    // Compression
    h3 { class: "mb-2 text-xl font-bold", "Invite Member" }
    a { class: "text-white", "Send invitation" }

    // Props on tops
    h3 { class: "mb-2 text-xl font-bold mb-2 text-xl font-bold mb-2 text-xl font-bold mb-2 text-xl font-bold mb-2 text-xl font-bold",
        "Invite Member"
    }

    // No children, minimal props
    img { class: "mb-6 mx-auto h-24", src: "artemis-assets/images/friends.png" }

    // One level compression
    div {
        a {
            class: "py-2 px-3 bg-indigo-500 hover:bg-indigo-600 rounded text-xs text-white",
            href: "#",
            "Send invitation"
        }
    }

    // Components
    Component { ..Props {} }

    // multiline
    div {
        class: "asdaskjdhaskjdjaslkdjlakdjaslkdjaslkd asdaskjdhaskjdjaslkdjlakdjaslkdjaslkdasdaskjdhaskjdjaslkdjlakdjaslkdjaslkd",
        multiple: "asd",
        "hi"
    }
}
