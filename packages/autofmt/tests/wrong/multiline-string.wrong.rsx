fn app() -> Element {
    let name = "aaa";
    rsx! {
        input {
            placeholder: format!("bbb
                             {name}"),
            "data-shortcut": "alt+a",
        }
        div {
            style: format!(r#"padding: {x}px;
                        color: red;"#),
            "hi"
        }
    }
}
