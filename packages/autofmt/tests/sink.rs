use dioxus_autofmt::*;
use proc_macro2::TokenStream as TokenStream2;
use syn::{Attribute, Meta};

fn test_block(wrong: &str, right: &str) {
    let formatted = fmt_block(wrong).unwrap();
    assert_eq!(formatted, right);
}

fn print_block(wrong: &str) {
    let formatted = fmt_block(wrong).unwrap();
    println!("{}", formatted);
}

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

                                    Component {}

                                    Component<Generic> {}
            }
        }
    "#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
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
fn print_cases() {
    print_block(
        r#"
        div {
            adsasd: "asd",
                h1 {"asd"}
                div {
                    div {
                        "hello"
                    }
                    div {
                        "goodbye"
                    }
                    div { class: "broccoli",
                        div {
                            "hi"
                        }
                    }
                    div { class: "broccolibroccolibroccolibroccolibroccolibroccolibroccolibroccolibroccolibroccoli",
                        div {
                            "hi"
                        }
                    }
                    div { class: "alksdjasd", onclick: move |_| {
                        // hi!
                        liberty!();
                    },
                        div {
                            "hi"
                        }
                    }

                    commented {
                        // is unparalled
                        class: "asdasd",

                        // My genius
                        div {
                            "hi"
                        }

                        div {

                        }
                    }
                }
        }
    "#,
    );
}

#[test]
fn format_comments() {
    let block = r#"
    div {
        adsasd: "asd", block: "asd",


        // this is a comment
        "hello"

        // this is a comment 1

        // this is a comment 2
        "hello"

        div {
            // this is a comment
            "hello"
        }

        div {
            // empty space
        }
    }
        "#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
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

    println!("{formatted}");
}

#[test]
fn formats_element() {
    let block = r#"
    div {
        a: "1234567891012345678910123456789101234567891012345678910123456789101234567891012345678910123456789101234567891012345678910",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
    }
"#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
}

#[test]
fn formats_element_short() {
    let block = r#"
    div {
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
        a: "123",
    }
"#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
}

#[test]
fn formats_element_nested() {
    let block = r#"
    h3 {
        class: "mb-2 text-xl font-bold",
        "Invite Member"
    }
"#;

    let formatted = fmt_block(block).unwrap();

    assert_eq!(
        formatted,
        r#"h3 { class: "mb-2 text-xl font-bold", "Invite Member" }"#
    );
}

#[test]
fn formats_element_props_on_top() {
    let block = r#"
    h3 {
        class: "mb-2 text-xl font-bold mb-2 text-xl font-bold mb-2 text-xl font-bold mb-2 text-xl font-bold mb-2 text-xl font-bold",

        "Invite Member"
    }
"#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
}

#[test]
fn formats_element_nested_no_trailing_tabs() {
    let block = r#"
    img { class: "mb-6 mx-auto h-24", src: "artemis-assets/images/friends.png", alt: "",                }
"#;

    let formatted = fmt_block(block).unwrap();

    assert_eq!(
        formatted,
        r#"img { class: "mb-6 mx-auto h-24", src: "artemis-assets/images/friends.png", alt: "" }"#
    );
}

#[test]
fn formats_element_with_correct_indent() {
    let block = r###"
    div {

                        a { class: "py-2 px-3 bg-indigo-500 hover:bg-indigo-600 rounded text-xs text-white", href: "#",
                            "Send invitation"
          }
    }

"###;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
}

#[test]
fn small_elements_and_text_are_small() {
    let block = r###"
                        a { class: "text-white",
                            "Send invitation"
          }
"###;

    let formatted = fmt_block(block).unwrap();

    assert_eq!(formatted, r#"a { class: "text-white", "Send invitation" }"#);
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

    println!("{formatted}");
}

#[test]
fn formats_component_tiny() {
    let block = r#"
    Component { a: 123
    }
"#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
}

#[test]
fn formats_exprs() {
    let block = r#"
    ul {
        div {}
        (0..10).map(|f| rsx! {
            li { "hi" }
        })
        div {}
    }
"#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
}

#[test]
fn formats_exprs_neg_indent() {
    let block = r#"
            ul {
        (0..10).map(|f| rsx!{
            li {
                "hi"
            }
        })
    }
"#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
}

#[test]
fn formats_exprs_handlers() {
    let block = r#"
            button {
                class: "flex items-center pl-3 py-3 pr-2 text-gray-500 hover:bg-indigo-50 rounded",
                onclick: move |evt| {
                    show_user_menu.set(!show_user_menu.get());            evt.cancel_bubble();        },

                onclick: move |evt|

                show_user_menu.set(!show_user_menu.get()),
                span { class: "inline-block mr-4",
                    icons::icon_14 {}
                }
                span { "Settings" }
            }
"#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
}

#[test]
fn formats_complex() {
    let block = r#"
        li {
            Link {
                class: "flex items-center pl-3 py-3 pr-4 {active_class} rounded",
                to: "{to}",
                span { class: "inline-block mr-3",
                    icons::icon_0 {}
                }
                span { "{name}" }
                children.is_some().then(|| rsx!{
                    span {
                        class: "inline-block ml-auto hover:bg-gray-500",
                        onclick: move |evt| {
                            // open.set(!open.get());
                            evt.cancel_bubble();
                        },
                        icons::icon_8 {}
                    }
                })
            }
            div {
                class: "px-4",
                is_current.then(|| rsx!{ children })
                // open.then(|| rsx!{ children })
            }
        }
"#;

    let formatted = fmt_block(block).unwrap();

    println!("{formatted}");
}

#[test]
fn formats_document() {
    let block = r#"
rsx!{
    Component {
        adsasd: "asd", // this is a comment
        onclick: move |_| {
            let blah = 120;
        },
    }
}


"#;

    let formatted = fmt_file(block);

    println!("{formatted:?}");
}

#[test]
fn component_path_mod_style() {
    let block = r#"
rsx!{
    my::thing::Component {
        adsasd: "asd", // this is a comment
        onclick: move |_| {
            let blah = 120;
        },
    }
}
"#;

    let formatted = fmt_file(block);

    println!("{formatted:?}");
}

#[test]
fn formats_valid_rust_src() {
    let src = r#"
//
rsx! {
    div {}
    div {
        h3 {"asd"
        }
    }
}
"#;

    let formatted = fmt_file(src);

    println!("{formatted:?}");
}

#[test]
fn formats_valid_rust_src_with_indents() {
    let mut src = r#"
#[inline_props]
fn NavItem<'a>(cx: Scope, to: &'static str, children: Element<'a>, icon: Shape) -> Element {
    const ICON_SIZE: u32 = 36;

    rsx! {
        div {

            h1 {"thing"}


        }




    }
}
"#
    .to_string();

    let formatted = fmt_file(&src);

    let block = formatted.into_iter().next().unwrap();

    src.replace_range(
        block.start - 1..block.end + 1,
        &format!("{{ {}    }}", &block.formatted),
    );
}

#[test]
fn formats_multiple_blocks() {
    let mut src = r#"
#[inline_props]
fn NavItem<'a>(cx: Scope, to: &'static str, children: Element<'a>, icon: Shape) -> Element {
    const ICON_SIZE: u32 = 36;

    rsx! {
        div {

            h1 {"thing"}


        }


    }

    rsx! {
        div {

            Ball {
                a: rsx!{
                    "asdasd"
                }
            }
        }
    }
}
#[inline_props]
fn NavItem<'a>(cx: Scope, to: &'static str, children: Element<'a>, icon: Shape) -> Element {
    const ICON_SIZE: u32 = 36;

    rsx! {
        div {

            h1 {"thing"}


        }


    }

    rsx! {
        div {

            Ball {
                a: rsx!{
                    "asdasd"
                }
            }
        }
    }
}
"#
    .to_string();

    let formatted = fmt_file(&src);

    dbg!(&formatted);

    let block = formatted.into_iter().next().unwrap();

    src.replace_range(
        block.start - 1..block.end + 1,
        &format!("{{ {}    }}", &block.formatted),
    );
}

#[test]
fn empty_blocks() {
    let mut src = r###"
pub fn Alert(cx: Scope) -> Element {
    cx.render(rsx! {
        div { }
    })
}
"###
    .to_string();

    let formatted = fmt_file(&src);

    dbg!(&formatted);
}
