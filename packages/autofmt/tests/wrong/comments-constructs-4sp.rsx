fn app() -> Element {
    rsx! {
        // Comments between the name of an element and its opening brace
        div { // after the name
            "child"
        }
        div { // below the name
            class: "a",
            "child"
        }
        div { // after the name
            // after the brace
            // above an attribute
            class: "a",
        }
        div { // of an empty element
        }
        Comp::<T> { // after the generics
            // and below them
            value: 1,
        }

        // Comments between the name of an attribute and its value
        div {
            class:
                // above the value
                "a",
            id: // after the colon
                "b",
            width: // before the colon
                "c",
            onclick: // after the colon
                // and above the value
                move |_| {
                    first();
                    // inside the value
                    second();
                },
            height:
                // above a shorthand value
                height,
            "data-x":
                // above an if value
                if a { "x" } else { "y" },
            "child"
        }
        Comp {
            value: // after the colon of a prop
                1,
        }

        // Comments inside an if attribute value
        div {
            class: if a {
                // above the value
                "x"
            } else {
                "y" // after the value
            },
            id: if a { // after the opening brace
                "x"
                // below the value
            } // after the closing brace
            // above an else if
            else if b {
                // between blank lines
                "y"
            }
            // above an else
            else {
                "z"
            }, // after the attribute
            width: if a
                // inside the condition
                && b {
                "1"
            },
            onclick: if a {
                // above an expression
                move |_| {
                    // inside an expression
                    first();
                }
            } else {
                move |_| {}
            },
            "child"
        }
        div {
            class: if a {
                // on the same line as the opening brace
                "x"
            },
        }

        // Comments inside the header of a for loop or an if
        for item in items
            // between method calls
            .iter()
            .filter(|item| item.is_ok()) // after a method call
        {
            div {}
        }
        for (i, // inside a pattern
            item) in items {
            div {}
        }
        if a
            // inside a condition
            && b // at the end of a condition
        {
            div {}
        } else if c // after an else if
            || d {
            div {}
        } else {
            div {}
        }

        // These only look like comments, so the headers are formatted as usual
        if a == "http://example.com" && b == '"' {
            div {}
        }
        for item in items.iter().map(|item| r#"// "not" a comment"#) {
            div {}
        }
    }
}
