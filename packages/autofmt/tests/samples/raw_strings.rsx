rsx! {
    // Raw text strings
    button { r#"Click me"#, r##"Click me"##, r######"Click me"######, r#"dynamic {1}"# }

    // Raw attribute strings
    div {
        width: r#"10px"#,
        height: r##"{10}px"##,
        "raw-attr": r###"raw-attr"###,
        "raw-attr2": r###"{100}"###
    }
}
