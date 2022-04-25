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

    let formatted = get_format_blocks(block);

    print!("{formatted:?}");
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

    let formatted = get_format_blocks(src);

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

    let formatted = get_format_blocks(&src);

    let block = formatted.into_iter().next().unwrap();

    // println!("{}", &src[block.start..block.end]);
    // println!("{}", &src[block.start..block.end - 1]);

    src.replace_range(
        block.start - 1..block.end + 1,
        &format!("{{ {}        }}", &block.formatted),
        // &format!("{{ {}    }}", &block.formatted),
    );

    println!("{src}");

    println!("\tthing");

    // for block in formatted {
    //     println!("{}", block.formatted);
    // }
}
