rsx! {
    button {
        onclick: move |_| async move {
            let url = "https://example.com";

            // keep this comment with the block, not the string literal
            println!("{url}");
        },
        "strings"
    }

    Component {
        message: {
            let prefix = "value://";

            // comment before final expression that contains //
            format!("{prefix}done")
        },
    }
}
