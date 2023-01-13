rsx! {
    // Does this work?
    for i in b {
        // Hey it works?
        div {}
    }

    // Some ifchain
    if a > 10 {
        //
        rsx! { div {} }
    }
}
