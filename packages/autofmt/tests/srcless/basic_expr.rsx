    div {
        "hi"
        {children}
    }
    Fragment {
        Fragment {
            Fragment {
                Fragment {
                    Fragment {
                        div { "Finally have a real node!" }
                    }
                }
            }
        }
    }
    div { class, "hello world" }
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
        class: "asdasd",
        onclick: move |_| {
            let a = 10;
            let b = 40;
            let c = 50;
        },
        "hi"
    }
    div { class: "asd", "Jon" }
