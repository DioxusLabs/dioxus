use crate::{util::*, write_ident};
use dioxus_rsx::*;
use quote::ToTokens;
use std::fmt::{self, Write};

pub fn write_component(
    component: &Component,
    buf: &mut String,
    indent: usize,
    lines: &[&str],
) -> Result<(), fmt::Error> {
    let Component {
        name,
        body,
        children,
        manual_props,
        prop_gen_args,
    } = component;
    let mut name = name.to_token_stream().to_string();
    name.retain(|c| !c.is_whitespace());
    write_tabs(buf, indent)?;
    write!(buf, "{name}")?;
    if let Some(generics) = prop_gen_args {
        let mut written = generics.to_token_stream().to_string();
        written.retain(|c| !c.is_whitespace());
        write!(buf, "{}", written)?;
    }
    write!(buf, " {{")?;
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
    Ok(())
}
