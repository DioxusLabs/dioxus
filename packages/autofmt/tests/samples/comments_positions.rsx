// A macro with only a comment in it
fn only_comment() -> Element {
    rsx! {
        // only a comment
    }
}

// A comment after the opening brace of the macro
fn after_open() -> Element {
    rsx! { // after the opening brace
        div {}
    }
}

fn after_open_only() -> Element {
    rsx! { // after the opening brace of an empty macro
    }
}

fn after_open_text() -> Element {
    rsx! { // after the opening brace, before a single text node
        "text"
    }
}

// A comment after the opening brace of a macro that is nested in an expression
fn after_open_nested() -> Element {
    rsx! {
        Comp {
            header: rsx! { // after the opening brace of an empty macro
            },
            footer: rsx! { // after the opening brace, before a single text node
                "text"
            },
            body: rsx! { // after the opening brace
                div { "x" }
                // before the closing brace
            },
            other: rsx! { "text" },
        }
        {items.iter().map(|i| rsx! { // after the opening brace, in a closure
            "{i}"
        })}
        {
            let a = rsx! { // after the opening brace, in a statement
                "a"
            };
            rsx! { // after the opening brace, in a tail expression
                {a}
            }
        }
    }
}

// Comments above a macro are not part of it
fn above_macro() -> Element {
    // above a macro that fits on one line
    let a = rsx! {
        div { class: "a", "b" }
    };
    // above an empty macro
    let b = rsx! {};
    // above a macro with several roots
    rsx! {
        {a}
        {b}
    }
}

fn roots() -> Element {
    rsx! {
        // above the first root
        div {} // after the first root

        // on its own between roots

        // above the second root
        "text" // after a text root
        // above a component root
        Comp {} // after a component root
        // after the last root
    }
}

fn bodies() -> Element {
    rsx! {
        div { // after the opening brace
            "child"
        }

        div { // after the opening brace
            class: "a",
            "child"
        }

        div { // after the opening brace of an empty element
        }

        Comp { // after the opening brace of an empty component
        }

        div {
            // only a comment
        }

        div {
            // only comments

            // separated by a blank line
        }

        Comp {
            // only a comment
        }

        div { class: "a",
            "child"
            // before the closing brace
        }

        div {
            "child"

            // before the closing brace, after a blank line

            // and another
        }

        Comp {
            // above a child component
            Comp {} // after a child component
            // before the closing brace
        }

        div {
            div {
                div {
                    "a" // after the innermost child
                    // before the innermost closing brace
                } // after the innermost closing brace
                // before the middle closing brace
            } // after the middle closing brace
            // before the outer closing brace
        } // after the outer closing brace

        div { class: "a", "child" } // after an element on one line
        div { "child" } // after an element on one line
        Comp { a: 1 } // after a component on one line
    }
}

// A comment stops an element from being collapsed onto one line
fn collapsing() -> Element {
    rsx! {
        div {
            // above a short text node
            "a"
        }

        div {
            "a" // after a short text node
        }

        div {
            // above an empty component
            Comp {}
        }

        div {
            // above a short expression
            {x}
        }

        div { class: "a",
            // above a short text node, below an attribute
            "a"
        }
    }
}

fn text() -> Element {
    rsx! {
        div {
            "a" // after the first
            "b" // after the second
            // above the third
            "c"
        }
    }
}

// Things that look like comments, and comments that look like other things
fn lookalikes() -> Element {
    rsx! {
        div { class: "http://example.com", // after a string with slashes
            "see // this is not a comment" // after text with slashes
            a { href: "https://example.com", "// text" }
            {format!("// {x}")} // after an expression with slashes
        }

        div {
            // a comment // with more slashes
            // http://example.com
            "child" // after // with more slashes
        }

        div {
            //
            class: "a", //
            //no space
            "child" //no space
            //
        }

        div {
            // div { class: "commented out",
            //     p { "A" }
            // }
            p { "B" }
            // p { "C" }
        }
    }
}

// Columns are counted in characters, not bytes
fn unicode() -> Element {
    rsx! {
        div {
            class: "🦀", // after an attribute with an emoji 🦀
            // ünïcödé above an attribute
            id: "ü",
            // ünïcödé above a child
            "héllo 🦀" // after text with an emoji
            // 🦀 before the closing brace
        }
    }
}
