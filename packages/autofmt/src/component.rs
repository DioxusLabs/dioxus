use crate::Buffer;
use dioxus_rsx::*;
use quote::ToTokens;
use std::fmt::{self, Result, Write};

impl Buffer {
    pub fn write_component(
        &mut self,
        Component {
            name,
            body,
            children,
            manual_props,
            prop_gen_args,
        }: &Component,
        lines: &[&str],
    ) -> Result {
        let mut name = name.to_token_stream().to_string();
        name.retain(|c| !c.is_whitespace());
        self.tab()?;
        write!(self.buf, "{name}")?;

        if let Some(generics) = prop_gen_args {
            let mut written = generics.to_token_stream().to_string();
            written.retain(|c| !c.is_whitespace());
            write!(self.buf, "{}", written)?;
        }

        write!(self.buf, " {{")?;

        if !body.is_empty() || !children.is_empty() {
            self.new_line()?;
        }

        for field in body {
            self.indented_tab()?;
            let name = &field.name;
            match &field.content {
                ContentField::ManExpr(exp) => {
                    let out = prettyplease::unparse_expr(exp);
                    writeln!(self.buf, "{}: {},", name, out)?;
                }
                ContentField::Formatted(s) => {
                    writeln!(self.buf, "{}: \"{}\",", name, s.value())?;
                }
                ContentField::OnHandlerRaw(exp) => {
                    let out = prettyplease::unparse_expr(exp);
                    let mut lines = out.split('\n').peekable();
                    let first = lines.next().unwrap();
                    write!(self.buf, "{}: {}", name, first)?;
                    for line in lines {
                        self.new_line()?;
                        self.indented_tab()?;
                        write!(self.buf, "{}", line)?;
                    }
                    writeln!(self.buf, ",")?;
                }
            }
        }

        if let Some(exp) = manual_props {
            self.indented_tab()?;
            let out = prettyplease::unparse_expr(exp);
            let mut lines = out.split('\n').peekable();
            let first = lines.next().unwrap();
            write!(self.buf, "..{}", first)?;
            for line in lines {
                self.new_line()?;
                self.indented_tab()?;
                write!(self.buf, "{}", line)?;
            }
            self.new_line()?;
        }

        for child in children {
            self.write_indented_ident(lines, child)?;
        }

        if !body.is_empty() || !children.is_empty() {
            self.tab()?;
        }
        writeln!(self.buf, "}}")?;
        Ok(())
    }
}
