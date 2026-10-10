rsx! {
    div {
        button {
            onclick: move |_| {
                spawn(async move {
                    match Ok::<(), ()>(()) {
                        Ok(()) => {
                            let script = format!(
                                r#"
                                requestAnimationFrame(() => {{
                                    console.log("restore");
                                }});
                                "#,
                            );
                            let _ = script;
                        }
                        Err(_) => {}
                    }
                });
            },
            "raw string in a nested closure"
        }
        button {
            onclick: move |_| {
                let indented = "first
                    second";
                let flush_left = "first
second";
                let _ = (indented, flush_left);
            },
            "plain strings"
        }
        {
            let inner = rsx! {
                button {
                    onclick: move |_| {
                        let script = r#"
                            console.log("nested");
                        "#;
                        let _ = script;
                    },
                    "raw string in a nested rsx block"
                }
            };
            inner
        }
    }
}
