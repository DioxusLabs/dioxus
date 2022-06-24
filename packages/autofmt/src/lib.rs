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
pub struct FormattedBlock {
    pub formatted: String,
    pub start: usize,
    pub end: usize,
}

/*


*/
pub fn get_format_blocks(contents: &str) -> Vec<FormattedBlock> {
    let mut matches = levenshtein_search(b"rsx! {", contents.as_bytes()).peekable();

    let mut cur_match: Option<Match> = None;

    let mut formatted_blocks = Vec::new();

    let mut last_bracket_end = 0;

    for item in matches {
        let Match { start, end, k } = item;

        if start < last_bracket_end {
            continue;
        }

        let remaining = &contents[end - 1..];

        if let Some(bracket_end) = find_bracket_end(remaining) {
            let sub_string = &contents[end..bracket_end + end - 1];

            last_bracket_end = bracket_end + end - 1;

            // with the edge brackets
            // println!("{}", &contents[end - 1..bracket_end + end]);

            if let Some(new) = fmt_block(sub_string) {
                if !new.is_empty() {
                    println!("{}", &contents[end + 1..bracket_end + end - 1]);
                    println!("{}", new);

                    let stripped = &contents[end + 1..bracket_end + end - 1];
                    if stripped == new {
                        println!("no changes necessary");
                    }

                    // if we have code to push, we want the code to end up on the right lines with the right indentation

                    let mut output = String::new();
                    writeln!(output).unwrap();

                    for line in new.lines() {
                        writeln!(output, "        {}", line).ok();
                    }

                    formatted_blocks.push(FormattedBlock {
                        formatted: output,
                        start: end,
                        end: end + bracket_end - 1,
                    });
                }
            } else {
                panic!("failed to format block: {}", sub_string);
            }
        } else {
            panic!("failed to find end of block: {}", remaining);
        }
    }

    formatted_blocks
}

pub fn fmt_block(block: &str) -> Option<String> {
    let mut raw_lines = block.split('\n').collect::<Vec<_>>();

    let parsed: CallBody = syn::parse_str(block).ok()?;

    let mut buf = String::new();

    for node in parsed.roots.iter() {
        write_ident(&mut buf, &raw_lines, node, 0).ok()?;
    }

    Some(buf)
}

pub fn write_ident(
    buf: &mut String,
    lines: &[&str],
    node: &BodyNode,
    indent: usize,
) -> fmt::Result {
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
            write!(buf, "{name} {{")?;

            let total_attr_len = attributes
                .iter()
                .map(|attr| match &attr.attr {
                    ElementAttr::AttrText { name, value } => value.value().len(),
                    ElementAttr::AttrExpression { name, value } => 10,
                    ElementAttr::CustomAttrText { name, value } => value.value().len(),
                    ElementAttr::CustomAttrExpression { name, value } => 10,
                    ElementAttr::EventTokens { name, tokens } => 1000000,
                    ElementAttr::Meta(_) => todo!(),
                })
                .sum::<usize>();

            let is_long_attr_list = total_attr_len > 80;

            if let Some(key) = key {
                let key = key.value();
                if is_long_attr_list {
                    write_tabs(buf, indent + 1)?;
                } else {
                    write!(buf, " ")?;
                }
                write!(buf, "key: \"{key}\"")?;

                if !attributes.is_empty() {
                    writeln!(buf, ",")?;
                }
            }

            let mut attr_iter = attributes.iter().peekable();
            while let Some(attr) = attr_iter.next() {
                if is_long_attr_list {
                    writeln!(buf)?;
                    write_tabs(buf, indent + 1)?;
                } else {
                    write!(buf, " ")?;
                }

                match &attr.attr {
                    ElementAttr::AttrText { name, value } => {
                        write!(buf, "{name}: \"{value}\"", value = value.value())?;
                    }
                    ElementAttr::AttrExpression { name, value } => {
                        let out = prettyplease::unparse_expr(value);
                        write!(buf, "{}: {}", name, out)?;
                    }

                    ElementAttr::CustomAttrText { name, value } => {
                        write!(
                            buf,
                            "\"{name}\": \"{value}\"",
                            name = name.value(),
                            value = value.value()
                        )?;
                    }

                    ElementAttr::CustomAttrExpression { name, value } => {
                        let out = prettyplease::unparse_expr(value);
                        write!(buf, "\"{}\": {}", name.value(), out)?;
                    }

                    ElementAttr::EventTokens { name, tokens } => {
                        let out = prettyplease::unparse_expr(tokens);

                        dbg!(&out);

                        let mut lines = out.split('\n').peekable();
                        let first = lines.next().unwrap();

                        // a one-liner for whatever reason
                        // Does not need a new line
                        if lines.peek().is_none() {
                            write!(buf, "{}: {}", name, first)?;
                        } else {
                            writeln!(buf, "{}: {}", name, first)?;

                            while let Some(line) = lines.next() {
                                write_tabs(buf, indent + 1)?;
                                write!(buf, "{}", line)?;
                                if lines.peek().is_none() {
                                    write!(buf, "")?;
                                } else {
                                    writeln!(buf)?;
                                }
                            }
                        }
                    }
                    ElementAttr::Meta(_) => {}
                }

                if attr_iter.peek().is_some() || !children.is_empty() {
                    write!(buf, ",")?;
                }
            }

            if children.len() == 1 && children[0].is_litstr() && !is_long_attr_list {
                if let BodyNode::Text(text) = &children[0] {
                    let text_val = text.value();
                    if total_attr_len + text_val.len() > 80 {
                        writeln!(buf)?;
                        write_tabs(buf, indent + 1)?;
                        writeln!(buf, "\"{}\"", text.value())?;
                        write_tabs(buf, indent)?;
                    } else {
                        write!(buf, " \"{}\" ", text.value())?;
                    }
                }
            } else {
                if is_long_attr_list || !children.is_empty() {
                    writeln!(buf)?;
                }

                for child in children {
                    write_ident(buf, lines, child, indent + 1)?;
                }

                if is_long_attr_list || !children.is_empty() {
                    write_tabs(buf, indent)?;
                } else {
                    write!(buf, " ")?;
                }
            }

            writeln!(buf, "}}")?;
        }
        BodyNode::Component(component) => {
            let Component {
                name,
                body,
                children,
                manual_props,
            } = component;

            let mut name = name.to_token_stream().to_string();
            name.retain(|c| !c.is_whitespace());

            write_tabs(buf, indent)?;
            write!(buf, "{name} {{")?;

            if !body.is_empty() || !children.is_empty() {
                writeln!(buf)?;
            }

            for field in body {
                write_tabs(buf, indent + 1)?;
                let name = &field.name;
                match &field.content {
                    ContentField::ManExpr(exp) => {
                        let out = prettyplease::unparse_expr(exp);
                        writeln!(buf, "{}: {},", name, out)?;
                    }
                    ContentField::Formatted(s) => {
                        writeln!(buf, "{}: \"{}\",", name, s.value())?;
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
                write_ident(buf, lines, child, indent + 1)?;
            }

            if !body.is_empty() || !children.is_empty() {
                write_tabs(buf, indent)?;
            }

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
        BodyNode::RawExpr(exp) => {
            use syn::spanned::Spanned;

            let placement = exp.span();

            let start = placement.start();
            let end = placement.end();

            let num_spaces_desired = (indent * 4) as isize;
            let first = lines[start.line - 1];
            let num_spaces_real = first.chars().take_while(|c| c.is_whitespace()).count() as isize;
            let offset = num_spaces_real - num_spaces_desired;

            for line_id in start.line - 1..end.line {
                let line = lines[line_id];

                // trim the leading whitespace

                if offset < 0 {
                    for _ in 0..-offset {
                        write!(buf, " ")?;
                    }

                    writeln!(buf, "{}", line)?;
                } else {
                    let offset = offset as usize;

                    let right = &line[offset..];
                    writeln!(buf, "{}", right)?;
                }
            }

            // let toks = exp.to_token_stream();

            // let out = prettyplease::unparse_expr(exp);
            // let mut lines = out.split('\n').peekable();
            // for line in lines {
            //     write_tabs(buf, indent)?;
            //     writeln!(buf, "{}", line)?;
            //     // writeln!(buf)?;
            // }
            // write_tabs(buf, indent)?;
            // let first = lines.next().unwrap();
            // write!(buf, "{}", name, first)?;
            // writeln!(buf)?;

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
