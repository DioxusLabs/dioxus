rsx! {
    Component {
        label: {
            let url = "https://example.com";

            // comment before formatted tail expr
            format!("{url}/docs")
        },
    }

    Component {
        status: {
            // comment before branch
            if is_ready() {
                "ready"
            } else {
                "waiting"
            }
        },
    }

    div {
        data_mode: {
            // comment before block expr
            if is_ready() {
                "ready"
            } else {
                "waiting"
            }
        },
    }
}
