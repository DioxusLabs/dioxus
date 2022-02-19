//! pretty printer for rsx!
use syn::{Attribute, Meta};

use crate::*;
use std::fmt::{self, Write};

fn fmt_block(block: &str) -> Option<String> {
    let parsed: CallBody = syn::parse_str(block).ok()?;

    let mut buf = String::new();

    for node in parsed.roots.iter() {
        write_ident(&mut buf, node, 0).ok()?;
    }

    Some(buf)
}

fn write_ident(buf: &mut dyn Write, node: &BodyNode, indent: usize) -> fmt::Result {
    match node {
        BodyNode::Element(el) => {
            let Element {
                name,
                key,
                attributes,
                children,
                _is_static,
            } = el;

            write_tabs(buf, indent)?;
            writeln!(buf, "{name} {{")?;

            if let Some(key) = key {
                let key = key.value();
                write_tabs(buf, indent + 1)?;
                write!(buf, "key: \"{key}\"")?;
                if !attributes.is_empty() {
                    writeln!(buf, ",")?;
                }
            }

            for attr in attributes {
                write_tabs(buf, indent + 1)?;
                match &attr.attr {
                    ElementAttr::AttrText { name, value } => {
                        writeln!(buf, "{name}: \"{value}\",", value = value.value())?;
                    }
                    ElementAttr::AttrExpression { name, value } => {
                        let out = prettyplease::unparse_expr(value);
                        writeln!(buf, "{}: {},", name, out)?;
                    }

                    ElementAttr::CustomAttrText { name, value } => todo!(),
                    ElementAttr::CustomAttrExpression { name, value } => todo!(),

                    ElementAttr::EventTokens { name, tokens } => {
                        let out = prettyplease::unparse_expr(tokens);

                        let mut lines = out.split('\n').peekable();
                        let first = lines.next().unwrap();
                        writeln!(buf, "{}: {}", name, first)?;

                        while let Some(line) = lines.next() {
                            write_tabs(buf, indent + 1)?;
                            write!(buf, "{}", line)?;
                            // writeln!(buf, "{}", line)?;
                            if lines.peek().is_none() {
                                writeln!(buf, ",")?;
                            } else {
                                writeln!(buf)?;
                            }
                        }
                    }
                    ElementAttr::Meta(_) => {}
                }
            }

            for child in children {
                write_ident(buf, child, indent + 1)?;
            }

            write_tabs(buf, indent)?;
            writeln!(buf, "}}")?;
        }
        BodyNode::Component(_) => {
            //
            // write!(buf, "{}", " ".repeat(ident))
        }
        BodyNode::Text(t) => {
            //
            // write!(buf, "{}", " ".repeat(ident))
            write_tabs(buf, indent)?;
            writeln!(buf, "\"{}\"", t.value())?;
        }
        BodyNode::RawExpr(_) => {
            //
            // write!(buf, "{}", " ".repeat(ident))
        }
        BodyNode::Meta(att) => {
            //
            // if att.path.segments.last().unwrap().ident == "doc" {
            let val = att.tokens.to_string();
            write_tabs(buf, indent)?;
            writeln!(buf, "{}", val)?;
            // }
            // match att {}
        }
    }

    Ok(())
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
            }
        }
    "#;

    let formatted = fmt_block(block).unwrap();

    print!("{formatted}");
}

fn write_tabs(f: &mut dyn Write, num: usize) -> std::fmt::Result {
    for _ in 0..num {
        write!(f, "    ")?
    }
    Ok(())
}

#[test]
fn parse_comment() {
    let block = r#"
    div {
        adsasd: "asd", // this is a comment
    }
        "#;

    // let parsed: CallBody = syn::parse_str(block).ok()?;

    let parsed: TokenStream2 = syn::parse_str(block).unwrap();

    dbg!(parsed);
}
