use crate::{ifmt_to_string, writer::Location, Writer};
use dioxus_rsx::*;
use quote::ToTokens;
use std::fmt::{Result, Write};
use syn::{spanned::Spanned, AngleBracketedGenericArguments};

#[derive(Debug)]
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

impl Writer<'_> {
    pub fn write_component(
        &mut self,
        Component {
            name,
            fields,
            children,
            manual_props,
            prop_gen_args,
            ..
        }: &Component,
    ) -> Result {
        self.write_component_name(name, prop_gen_args)?;

        // decide if we have any special optimizations
        // Default with none, opt the cases in one-by-one
        let mut opt_level = ShortOptimization::NoOpt;

        // check if we have a lot of attributes
        let attr_len = self.field_len(fields, manual_props);
        let is_short_attr_list = attr_len < 80;
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
        if fields.is_empty() && children.is_empty() && manual_props.is_none() {
            opt_level = ShortOptimization::Empty;
        }

        // multiline handlers bump everything down
        if attr_len > 1000 || self.out.indent.split_line_attributes() {
            opt_level = ShortOptimization::NoOpt;
        }

        // Useful for debugging
        // dbg!(
        //     name.to_token_stream().to_string(),
        //     &opt_level,
        //     attr_len,
        //     is_short_attr_list,
        //     is_small_children
        // );

        match opt_level {
            ShortOptimization::Empty => {}
            ShortOptimization::Oneliner => {
                write!(self.out, " ")?;

                self.write_component_fields(fields, manual_props, true)?;

                if !children.is_empty() && !fields.is_empty() {
                    write!(self.out, ", ")?;
                }

                for child in children {
                    self.write_ident(child)?;
                }

                write!(self.out, " ")?;
            }

            ShortOptimization::PropsOnTop => {
                write!(self.out, " ")?;
                self.write_component_fields(fields, manual_props, true)?;

                if !children.is_empty() && !fields.is_empty() {
                    write!(self.out, ",")?;
                }

                self.write_body_indented(children)?;
                self.out.tabbed_line()?;
            }

            ShortOptimization::NoOpt => {
                self.write_component_fields(fields, manual_props, false)?;

                if !children.is_empty() && !fields.is_empty() {
                    write!(self.out, ",")?;
                }

                self.write_body_indented(children)?;
                self.out.tabbed_line()?;
            }
        }

        write!(self.out, "}}")?;
        Ok(())
    }

    fn write_component_name(
        &mut self,
        name: &syn::Path,
        generics: &Option<AngleBracketedGenericArguments>,
    ) -> Result {
        let mut name = name.to_token_stream().to_string();
        name.retain(|c| !c.is_whitespace());

        write!(self.out, "{name}")?;

        if let Some(generics) = generics {
            let mut written = generics.to_token_stream().to_string();
            written.retain(|c| !c.is_whitespace());

            write!(self.out, "{written}")?;
        }

        write!(self.out, " {{")?;

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
                self.out.indented_tabbed_line().unwrap();
            }

            let name = &field.name;
            match &field.content {
                ContentField::ManExpr(exp) => {
                    let out = prettyplease::unparse_expr(exp);
                    let mut lines = out.split('\n').peekable();
                    let first = lines.next().unwrap();
                    write!(self.out, "{name}: {first}")?;
                    for line in lines {
                        self.out.new_line()?;
                        self.out.indented_tab()?;
                        write!(self.out, "{line}")?;
                    }
                }
                ContentField::Formatted(s) => {
                    write!(
                        self.out,
                        "{}: {}",
                        name,
                        s.source.as_ref().unwrap().to_token_stream()
                    )?;
                }
                ContentField::Shorthand(e) => {
                    write!(self.out, "{}", e.to_token_stream())?;
                }
                ContentField::OnHandlerRaw(exp) => {
                    let out = prettyplease::unparse_expr(exp);
                    let mut lines = out.split('\n').peekable();
                    let first = lines.next().unwrap();
                    write!(self.out, "{name}: {first}")?;
                    for line in lines {
                        self.out.new_line()?;
                        self.out.indented_tab()?;
                        write!(self.out, "{line}")?;
                    }
                }
            }

            if field_iter.peek().is_some() || manual_props.is_some() {
                write!(self.out, ",")?;

                if sameline {
                    write!(self.out, " ")?;
                }
            }
        }

        if let Some(exp) = manual_props {
            if !sameline {
                self.out.indented_tabbed_line().unwrap();
            }
            self.write_manual_props(exp)?;
        }

        Ok(())
    }

    pub fn field_len(
        &mut self,
        fields: &[ComponentField],
        manual_props: &Option<syn::Expr>,
    ) -> usize {
        let attr_len = fields
            .iter()
            .map(|field| match &field.content {
                ContentField::Formatted(s) => ifmt_to_string(s).len() ,
                ContentField::Shorthand(e) => e.to_token_stream().to_string().len(),
                ContentField::OnHandlerRaw(exp) | ContentField::ManExpr(exp) => {
                    let formatted = prettyplease::unparse_expr(exp);
                    let len = if formatted.contains('\n') {
                        10000
                    } else {
                        formatted.len()
                    };
                    self.cached_formats.insert(Location::new(exp.span().start()) , formatted);
                    len
                },
            } + 10)
            .sum::<usize>();

        match manual_props {
            Some(p) => {
                let content = prettyplease::unparse_expr(p);
                if content.len() + attr_len > 80 {
                    return 100000;
                }
                let mut lines = content.lines();
                lines.next().unwrap();

                if lines.next().is_none() {
                    attr_len + content.len()
                } else {
                    100000
                }
            }
            None => attr_len,
        }
    }

    fn write_manual_props(&mut self, exp: &syn::Expr) -> Result {
        /*
        We want to normalize the expr to the appropriate indent level.
        */

        let formatted = prettyplease::unparse_expr(exp);

        let mut lines = formatted.lines();

        let first_line = lines.next().unwrap();

        write!(self.out, "..{first_line}")?;
        for line in lines {
            self.out.indented_tabbed_line()?;
            write!(self.out, "{line}")?;
        }

        Ok(())
    }
}
