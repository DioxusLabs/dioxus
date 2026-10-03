rsx! {
    // A comment above a single short attribute
    div {
        // above
        class: "a",
    }

    // A comment after a single short attribute
    div {
        class: "a", // after
    }

    // Comments after each short attribute
    div {
        class: "a", // after a
        id: "b", // after b
    }

    // A comment between short attributes
    div {
        class: "a",
        // between
        id: "b",
    }

    // A comment after the last attribute, with nothing following it
    div {
        class: "a",
        // after the last attribute
    }

    // A comment after the only attribute, with a short child
    div { class: "a", // after
        "child"
    }

    // Comments after every attribute, with a short child
    div {
        class: "a", // after a
        id: "b", // after b
        "child"
    }

    // A comment after the last attribute, with several children
    div { class: "a", id: "b", // after b
        div {}
        div {}
    }

    // Several comments and blank lines above attributes
    div {
        // one
        // two

        // three
        class: "a",

        // four
        id: "b",
    }

    // Comments on attributes that do not fit on one line
    div {
        class: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", // after a
        id: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", // after b
        // above the child
        div {}
    }

    // Comments around a multi-line event handler
    div {
        // above
        onclick: move |_| {
            // inside
            let a = 1;
            let b = 2;
        }, // after
        // above the child
        "child"
    }

    // Comments around conditional attributes
    div {
        // above
        class: if a { "x" } else { "y" }, // after
    }

    // Comments around the other kinds of attribute
    Comp {
        // above the key
        key: "{id}", // after the key
        // above a shorthand
        a, // after a shorthand
        // above a custom attribute
        "data-x": "1", // after a custom attribute
        // above a block
        b: {
            // inside a block
            x
        }, // after a block
    }

    // A comment above a spread
    div {
        class: "a",
        // above
        ..attrs,
    }

    // A comment after a spread
    div {
        class: "a",
        ..attrs, // after
    }

    // A comment above a spread that is the only attribute
    div {
        // above
        ..attrs,
    }

    // Comments around a spread that is followed by a child
    div {
        // above
        ..attrs, // after
        // above the child
        "child"
    }

    // Comments everywhere around attributes and a spread
    Comp {
        // above a
        a: 1, // after a
        // above the spread
        ..props, // after the spread
        // after everything
    }

    // Spreads without comments still collapse
    div { ..attrs, "child" }
    div { ..attrs,
        div {}
        div {}
    }
}
