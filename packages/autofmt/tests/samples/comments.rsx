rsx! {
    div {
        // Comments
        class: "asdasd",
        "hello world"
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

    // comments inline
    div { // inline
        // Collapse
        class: "asd", // super inline
        class: "asd", // super inline
        "Jon" // all the inline
        // Comments at the end too
    }

    // please dont eat me 1
    div { // please dont eat me 2
        // please dont eat me 3
    }

    // please dont eat me 1
    div { // please dont eat me 2
        // please dont eat me 3
        abc: 123,
    }

    // please dont eat me 1
    div {
        // please dont eat me 3
        abc: 123,
    }

    div {
        // I am just a comment
    }

    div {
        "text"
        // I am just a comment
    }

    div {
        div {}
        // I am just a comment
    }

    div {
        {some_expr()}
        // I am just a comment
    }

    div {
        "text" // I am just a comment
    }

    div {
        div {} // I am just a comment
    }

    div {
        {some_expr()} // I am just a comment
    }

    div {
        // Please dont eat me 1
        div {
            // Please dont eat me 2
        }
        // Please dont eat me 3
    }

    div {
        "hi"
        // Please dont eat me 1
    }
    div {
        "hi" // Please dont eat me 1
        // Please dont eat me 2
    }

    // Please dont eat me 2
    Component {}

    // Please dont eat me 1
    Component {
        // Please dont eat me 2
    }

    // Please dont eat me 1
    Component {
        // Please dont eat me 2
    }

    div {
        {
            // Please dont eat me 1
            let millis = timer
                .with(|t| {
                    t.duration()
                        .saturating_sub(
                            t.started_at.map(|x| x.elapsed()).unwrap_or(Duration::ZERO),
                        )
                        .as_millis()
                });

            // Please dont eat me 2
            format!(
                "{:02}:{:02}:{:02}.{:01}",
                millis / 1000 / 3600 % 3600, // Please dont eat me 3
                millis / 1000 / 60 % 60,
                millis / 1000 % 60,

                // Please dont eat me 4
                millis / 100 % 10,
            );

            // booo //
            let b = { yay };

            // boo // boo
            let a = {
                let a = "123 // boo 123";
                // boo // boo
                asdb
            };

            format!("{b} {a}")
            // ennd
        }
    }

    div {
        // booo //
        {yay}

        // boo // boo
        {
            let a = "123 // boo 123";
            // boo // boo
            rsx! { "{a}" }
        }
    }

    div {
        input {
            r#type: "number",
            min: 0,
            max: 99,
            value: format!("{:02}", timer.read().hours),
            oninput: move |e| {
                // A comment inside an expression
                timer.write().hours = e.value().parse().unwrap_or(0);
            },
        }

        input {
            r#type: "number",
            min: 0,
            max: 59,
            value: format!("{:02}", timer.read().minutes),
            oninput: move |e| {
                // A comment inside an expression
                timer.write().minutes = e.value().parse().unwrap_or(0);

                // A comment inside an expression
            },
        }

        input {
            r#type: "number",
            min: 0,
            max: 59,
            value: format!("{:02}", timer.read().seconds),
            oninput: move |e| {
                // A comment inside an expression
                timer.write().seconds = e.value().parse().unwrap_or(0);
                // A comment inside an expression
            },
        }
    }

    {
        rsx! { "{a}" }
    }

    {
        rsx! { "one" }
    }

    div {}

    {
        rsx! { "one two three" }
    }

    // Please dont eat me 1
    //
    // Please dont eat me 2
}
