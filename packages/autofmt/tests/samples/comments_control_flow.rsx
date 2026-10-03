rsx! {
    // above a for loop
    for i in 0..10 { // after the opening brace
        // above the first child
        div { "{i}" } // after a child
        // before the closing brace
    } // after the closing brace

    for i in 0..10 {
        // a for loop with only a comment
    }

    for i in 0..10 { // a comment after the opening brace of an empty loop
    }

    for item in some_really_long_iterator_name
        .iter()
        .filter(|x| x.is_enabled())
        .map(|x| x.transform_into_something())
    { // after the opening brace of a multi-line loop
        // above the first child
        div {}
        // before the closing brace
    }

    // above an if chain
    if a { // after the opening brace of the if
        // above the first child of the if
        div { "a" } // after a child of the if
        // before the closing brace of the if
    } else if b { // after the opening brace of the else if
        // above the first child of the else if
        div { "b" }
        // before the closing brace of the else if
    } else { // after the opening brace of the else
        // above the first child of the else
        div { "c" }
        // before the closing brace of the else
    } // after the closing brace of the chain

    if a {
        // an if with only a comment
    } else if b {
        // an else if with only a comment
    } else {
        // an else with only a comment
    }

    if let Some(x) = a { // after the opening brace
        "{x}" // after a text node
    } else if let Some(y) = b {
        // only a comment
    } // after the closing brace of an else if

    // Bodies without comments stay empty
    for i in 0..10 {}
    if a {} else if b {} else {}

    // Blank lines around the comments of a body
    if a {
        div {}

        // after a blank line

        // and another
    }

    // Nested control flow
    div {
        // above the loop
        for a in b {
            // above the if
            if a {
                // above the inner loop
                for c in d {
                    // only a comment
                }
                // before the closing brace of the if
            }
            // before the closing brace of the loop
        }
        // before the closing brace of the element
    }

    // Control flow below attributes
    ul { class: "list", // after the last attribute
        for i in items { // after the opening brace
            li { "{i}" }
        }
    }

    // above a match
    match x {
        // above an arm
        1 => rsx! {
            // above a child
            div { "a" } // after a child
            // before the closing brace
        },
        // above the last arm
        _ => rsx! {
            // only a comment
        }, // after an arm
        // before the closing brace of the match
    } // after the closing brace of the match

    // above an expression
    {children} // after an expression

    // above an iterator
    {items.iter().map(|i| rsx! { // after the opening brace
        // above a child
        div { "{i}" } // after a child
        // before the closing brace
    })}
}
