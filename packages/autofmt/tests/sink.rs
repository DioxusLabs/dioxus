use dioxus_autofmt::*;
use proc_macro2::TokenStream as TokenStream2;
use syn::{Attribute, Meta};

#[test]
fn formats_block() {
    let block = r#"
        div {
                                    div {
                                    class: "asd",
                                    class: "asd",class: "asd",class: "asd",class: "asd",class: "asd",
                                    key: "ddd",
                                    onclick: move |_| {
                                        let blah = 120;
                                                    true
                                    },
                                    blah: 123,
                                    onclick: move |_| {
                                        let blah = 120;
                                                    true
                                    },
                                    onclick: move |_| {
                                        let blah = 120;
                                                    true
                                    },
                                    onclick: move |_| {
                                        let blah = 120;
                                                    true
                                    },

                                    div {
                                        div {
                                            "hi"
                                        }
                                        h2 {
                            class: "asd",
                                        }
                                    }
            }
        }
    "#;

    let formatted = fmt_block(block).unwrap();

    print!("{formatted}");
}

#[test]
fn parse_comment() {
    let block = r#"
    div {
        adsasd: "asd", // this is a comment
    }
        "#;

    let parsed: TokenStream2 = syn::parse_str(block).unwrap();

    dbg!(parsed);
}

#[test]
fn formats_component() {
    let block = r#"
    Component {
        adsasd: "asd", // this is a comment
        onclick: move |_| {
            let blah = 120;
            let blah = 120;
        },
    }
"#;

    let formatted = fmt_block(block).unwrap();

    print!("{formatted}");
}

#[test]
fn formats_component_man_props() {
    let block = r#"
    Component {
        ..MyProps {
            val: 123
        },
        adsasd: "asd", // this is a comment
        onclick: move |_| {
            let blah = 120;
        },
    }
"#;

    let formatted = fmt_block(block).unwrap();

    print!("{formatted}");
}
