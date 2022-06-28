use crate::Buffer;
use dioxus_rsx::*;
use quote::ToTokens;
use std::fmt::{Result, Write};
use syn::AngleBracketedGenericArguments;

enum ShortOptimization {
    // Special because we want to print the closing bracket immediately
    Empty,

    // Special optimization to put everything on the same line
    Oneliner,

    // Optimization where children flow but props remain fixed on top
    PropsOnTop,

    // The noisiest optimization where everything flows
    NoOpt,
}

impl Buffer {
    pub fn write_component(
        &mut self,
        Component {
            name,
            fields,
            children,
            manual_props,
            prop_gen_args,
        }: &Component,
    ) -> Result {
        self.write_component_name(name, prop_gen_args)?;

        // decide if we have any special optimizations
        // Default with none, opt the cases in one-by-one
        let mut opt_level = ShortOptimization::NoOpt;

        // check if we have a lot of attributes
        let is_short_attr_list = self.is_short_fields(fields, manual_props).is_some();
        let is_small_children = self.is_short_children(children).is_some();

        // if we have few attributes and a lot of children, place the attrs on top
        if is_short_attr_list && !is_small_children {
            opt_level = ShortOptimization::PropsOnTop;
        }

        // even if the attr is long, it should be put on one line
        if !is_short_attr_list && (fields.len() <= 1 && manual_props.is_none()) {
            if children.is_empty() {
                opt_level = ShortOptimization::Oneliner;
            } else {
                opt_level = ShortOptimization::PropsOnTop;
            }
        }

        // if we have few children and few attributes, make it a one-liner
        if is_short_attr_list && is_small_children {
            opt_level = ShortOptimization::Oneliner;
        }

        // If there's nothing at all, empty optimization
        if fields.is_empty() && children.is_empty() {
            opt_level = ShortOptimization::Empty;
        }

        match opt_level {
            ShortOptimization::Empty => {}
            ShortOptimization::Oneliner => {
                write!(self.buf, " ")?;

                self.write_component_fields(fields, manual_props, true)?;

                if !children.is_empty() && !fields.is_empty() {
                    write!(self.buf, ", ")?;
                }

                for child in children {
                    self.write_ident(child)?;
                }

                write!(self.buf, " ")?;
            }

            ShortOptimization::PropsOnTop => {
                write!(self.buf, " ")?;
                self.write_component_fields(fields, manual_props, true)?;

                if !children.is_empty() && !fields.is_empty() {
                    write!(self.buf, ",")?;
                }

                self.write_body_indented(children)?;
                self.tabbed_line()?;
            }

            ShortOptimization::NoOpt => {
                self.write_component_fields(fields, manual_props, false)?;
                self.write_body_indented(children)?;
                self.tabbed_line()?;
            }
        }

        write!(self.buf, "}}")?;
        Ok(())
    }

    fn write_component_name(
        &mut self,
        name: &syn::Path,
        generics: &Option<AngleBracketedGenericArguments>,
    ) -> Result {
        let mut name = name.to_token_stream().to_string();
        name.retain(|c| !c.is_whitespace());

        write!(self.buf, "{name}")?;

        if let Some(generics) = generics {
            let mut written = generics.to_token_stream().to_string();
            written.retain(|c| !c.is_whitespace());

            write!(self.buf, "{}", written)?;
        }

        write!(self.buf, " {{")?;

        Ok(())
    }

    fn write_component_fields(
        &mut self,
        fields: &[ComponentField],
        manual_props: &Option<syn::Expr>,
        sameline: bool,
    ) -> Result {
        let mut field_iter = fields.iter().peekable();

        while let Some(field) = field_iter.next() {
            if !sameline {
                self.indented_tabbed_line()?;
            }

            let name = &field.name;
            match &field.content {
                ContentField::ManExpr(exp) => {
                    let out = prettyplease::unparse_expr(exp);
                    write!(self.buf, "{}: {}", name, out)?;
                }
                ContentField::Formatted(s) => {
                    write!(self.buf, "{}: \"{}\"", name, s.value())?;
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
                }
            }

            if field_iter.peek().is_some() || manual_props.is_some() {
                write!(self.buf, ",")?;

                if sameline {
                    write!(self.buf, " ")?;
                }
            }
        }

        if let Some(exp) = manual_props {
            if !sameline {
                self.indented_tabbed_line()?;
            }
            self.write_manual_props(exp)?;
        }

        Ok(())
    }
    pub fn is_short_fields(
        &self,
        fields: &[ComponentField],
        manual_props: &Option<syn::Expr>,
    ) -> Option<usize> {
        let attr_len = fields
            .iter()
            .map(|field| match &field.content {
                ContentField::ManExpr(exp) => exp.to_token_stream().to_string().len(),
                ContentField::Formatted(s) => s.value().len() ,
                ContentField::OnHandlerRaw(_) => 100000,
            } + 10)
            .sum::<usize>() + self.indent * 4;

        match manual_props {
            Some(p) => {
                let content = prettyplease::unparse_expr(p);
                if content.len() + attr_len > 80 {
                    return None;
                }
                let mut lines = content.lines();
                lines.next().unwrap();

                if lines.next().is_none() {
                    Some(attr_len + content.len())
                } else {
                    None
                }
            }
            None => Some(attr_len),
        }
    }

    fn write_manual_props(&mut self, exp: &syn::Expr) -> Result {
        /*
        We want to normalize the expr to the appropriate indent level.
        */

        let formatted = prettyplease::unparse_expr(exp);

        let mut lines = formatted.lines();

        let first_line = lines.next().unwrap();

        write!(self.buf, "..{first_line}")?;
        for line in lines {
            self.indented_tabbed_line()?;
            write!(self.buf, "{line}")?;
        }

        Ok(())
    }
}
