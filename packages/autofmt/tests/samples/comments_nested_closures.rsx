rsx! {
    button {
        onclick: move |_| async move {
            let nested = move || {
                // nested closure comment
                ()
            };

            nested();
        },
        "nested closure"
    }

    button {
        onclick: move |_| async move {
            let nested = move || async move {
                // nested async closure comment
                ()
            };

            nested().await;
        },
        "nested async closure"
    }
}
