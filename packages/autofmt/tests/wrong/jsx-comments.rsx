rsx! {
    // Before a tag
    div {
        // Before an attribute
        class: "a",
        id, // After an attribute
        // Before the first child
        h1 { "Hello" } // After a tag

        img { src: "image.png" } // After a self-closing tag
        // Before the closing tag
    }
    section {
        // Only a comment
    }
}
