rsx! {
    div {
        match true {
            true => rsx! {
                if true {
                    span { "a" }
                }
                if true {
                    span { "b" }
                }
                span { "c" }
            },
            false => rsx! {},
        }
    }
}
