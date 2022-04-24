//! pretty printer for rsx!
use dioxus_rsx::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use std::{
    fmt::{self, Write},
    ptr::NonNull,
};
use syn::{
    buffer::TokenBuffer,
    parse::{ParseBuffer, ParseStream},
};
use triple_accel::{levenshtein_search, Match};

mod prettyplease;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq, Hash)]
pub struct ForamttedBlock {
    pub formatted: String,
    pub start: usize,
    pub end: usize,
}

/*
TODO: nested rsx! calls

*/
pub fn formmat_document(contents: &str) -> Vec<ForamttedBlock> {
    let mut matches = levenshtein_search(b"rsx! {", contents.as_bytes()).peekable();

    let mut cur_match: Option<Match> = None;

    let mut formatted_blocks = Vec::new();

    while let Some(item) = matches.next() {
        let Match { start, end, k } = item;

        match cur_match {
            Some(ref this_match) => {
                // abort nested matches - these get handled automatically
                if start < this_match.end {
                    continue;
                } else {
                    cur_match = Some(item);
                }
            }
            None => {
                cur_match = Some(item);
            }
        }

        let remaining = &contents[end - 1..];

        if let Some(bracket_end) = find_bracket_end(remaining) {
            let sub_string = &contents[end..bracket_end + end - 1];

            if let Some(new) = fmt_block(sub_string) {
                if !new.is_empty() {
                    formatted_blocks.push(ForamttedBlock {
                        formatted: new,
                        start: end,
                        end: end + bracket_end - 1,
                    });
                }
            }
        }
    }

    formatted_blocks
}

pub fn fmt_block(block: &str) -> Option<String> {
    let parsed: CallBody = syn::parse_str(block).ok()?;

    let mut buf = String::new();

    for node in parsed.roots.iter() {
        write_ident(&mut buf, node, 0).ok()?;
    }

    Some(buf)
}

pub fn write_ident(buf: &mut String, node: &BodyNode, indent: usize) -> fmt::Result {
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
        BodyNode::Component(component) => {
            let Component {
                name,
                body,
                children,
                manual_props,
            } = component;

            let name = name.to_token_stream().to_string();

            write_tabs(buf, indent)?;
            writeln!(buf, "{name} {{")?;

            for field in body {
                write_tabs(buf, indent + 1)?;
                let name = &field.name;
                match &field.content {
                    ContentField::ManExpr(exp) => {
                        let out = prettyplease::unparse_expr(exp);
                        writeln!(buf, "{}: {},", name, out)?;
                    }
                    ContentField::Formatted(s) => {
                        writeln!(buf, "{}: {},", name, s.value())?;
                    }
                    ContentField::OnHandlerRaw(exp) => {
                        let out = prettyplease::unparse_expr(exp);
                        let mut lines = out.split('\n').peekable();
                        let first = lines.next().unwrap();
                        write!(buf, "{}: {}", name, first)?;
                        for line in lines {
                            writeln!(buf)?;
                            write_tabs(buf, indent + 1)?;
                            write!(buf, "{}", line)?;
                        }
                        writeln!(buf, ",")?;
                    }
                }
            }

            if let Some(exp) = manual_props {
                write_tabs(buf, indent + 1)?;
                let out = prettyplease::unparse_expr(exp);
                let mut lines = out.split('\n').peekable();
                let first = lines.next().unwrap();
                write!(buf, "..{}", first)?;
                for line in lines {
                    writeln!(buf)?;
                    write_tabs(buf, indent + 1)?;
                    write!(buf, "{}", line)?;
                }
                writeln!(buf)?;
            }

            for child in children {
                write_ident(buf, child, indent + 1)?;
            }

            write_tabs(buf, indent)?;
            writeln!(buf, "}}")?;

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
            let val = att.to_string();
            write_tabs(buf, indent)?;
            writeln!(buf, "{}", val)?;
            // }
            // match att {}
        }
    }

    Ok(())
}

pub fn write_tabs(f: &mut dyn Write, num: usize) -> std::fmt::Result {
    for _ in 0..num {
        write!(f, "    ")?
    }
    Ok(())
}

fn find_bracket_end(contents: &str) -> Option<usize> {
    let mut depth = 0;
    let mut i = 0;

    for c in contents.chars() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
        }

        if depth == 0 {
            return Some(i);
        }

        i += 1;
    }

    None
}
