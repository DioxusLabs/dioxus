fn app() -> Element {
    rsx! {
        table {
            {
                items
                    .iter()
                    .map(|x| {
                        // A comment in the closure
                        let y = f(x);

                        rsx! {
                            tr {
                                td { "one" }
                                td { "two" }
                                td {
                                    a { href: "x", "four" }
                                }
                            }
                        }
                    })
            }
        }
    }
}
