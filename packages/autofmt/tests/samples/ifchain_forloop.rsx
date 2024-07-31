rsx! {
    // Does this work?
    for i in b {
        // Hey it works?
        div {}
    }

    // Some ifchain
    if a > 10 {
        //
        div {}
    } else if a > 20 {
        h1 {}
    } else if a > 20 {
        h1 {}
    } else if a > 20 {
        h1 {}
    } else if a > 20 {
        h1 {}
    } else if a > 20 {
        h1 {}
    } else if a > 20 {
        h1 {}
    } else {
        h3 {}
    }

    div {
        class: "asdasd",
        class: if expr { "asdasd" } else { "asdasd" },
        class: if expr { "asdasd" },
        class: if expr { "asdasd" } else if expr { "asdasd" } else { "asdasd" },

        // comments?
        class: if expr { "asdasd" } else if expr { "asdasd" } else { "asdasd" }, // comments!!?
        // comments?
    }
}
