use crate::{prettier_please::unparse_expr, Writer};
use dioxus_rsx::*;
use proc_macro2::Span;
use quote::ToTokens;
use std::{
    fmt::Result,
    fmt::{self, Write},
};
use syn::{spanned::Spanned, token::Brace, Expr};

#[derive(Debug)]
enum ShortOptimization {
    /// Special because we want to print the closing bracket immediately
    ///
    /// IE
    /// `div {}` instead of `div { }`
    Empty,

    /// Special optimization to put everything on the same line and add some buffer spaces
    ///
    /// IE
    ///
    /// `div { "asdasd" }` instead of a multiline variant
    Oneliner,

    /// Optimization where children flow but props remain fixed on top
    PropsOnTop,

    /// The noisiest optimization where everything flows
    NoOpt,
}

impl Writer<'_> {
    /// Basically elements and components are the same thing
    ///
    /// This writes the contents out for both in one function, centralizing the annoying logic like
    /// key handling, breaks, closures, etc
    pub fn write_rsx_block(
        &mut self,
        attributes: &[Attribute],
        spreads: &[Spread],
        children: &[BodyNode],
        brace: &Brace,
    ) -> Result {
        // decide if we have any special optimizations
        // Default with none, opt the cases in one-by-one
        let mut opt_level = ShortOptimization::NoOpt;

        // check if we have a lot of attributes
        let attr_len = self.is_short_attrs(attributes, spreads);
        let is_short_attr_list = (attr_len + self.out.indent_level * 4) < 80;
        let children_len = self.is_short_children(children);
        let is_small_children = children_len.is_some();

        // if we have one long attribute and a lot of children, place the attrs on top
        if is_short_attr_list && !is_small_children {
            opt_level = ShortOptimization::PropsOnTop;
        }

        // even if the attr is long, it should be put on one line
        // However if we have childrne we need to just spread them out for readability
        if !is_short_attr_list && attributes.len() <= 1 && spreads.is_empty() {
            if children.is_empty() {
                opt_level = ShortOptimization::Oneliner;
            } else {
                opt_level = ShortOptimization::PropsOnTop;
            }
        }

        // if we have few children and few attributes, make it a one-liner
        if is_short_attr_list && is_small_children {
            if children_len.unwrap() + attr_len + self.out.indent_level * 4 < 100 {
                opt_level = ShortOptimization::Oneliner;
            } else {
                opt_level = ShortOptimization::PropsOnTop;
            }
        }

        // If there's nothing at all, empty optimization
        if attributes.is_empty() && children.is_empty() && spreads.is_empty() {
            opt_level = ShortOptimization::Empty;

            // Write comments if they exist
            self.write_todo_body(brace)?;
        }

        // multiline handlers bump everything down
        if attr_len > 1000 || self.out.indent.split_line_attributes() {
            opt_level = ShortOptimization::NoOpt;
        }

        let has_children = !children.is_empty();

        match opt_level {
            ShortOptimization::Empty => {}
            ShortOptimization::Oneliner => {
                write!(self.out, " ")?;

                self.write_attributes(attributes, spreads, true, brace, has_children)?;

                if !children.is_empty() && !attributes.is_empty() {
                    write!(self.out, " ")?;
                }

                for child in children.iter() {
                    self.write_ident(child)?;
                }

                write!(self.out, " ")?;
            }

            ShortOptimization::PropsOnTop => {
                if !attributes.is_empty() {
                    write!(self.out, " ")?;
                }

                self.write_attributes(attributes, spreads, true, brace, has_children)?;

                if !children.is_empty() {
                    self.write_body_indented(children)?;
                }

                self.out.tabbed_line()?;
            }

            ShortOptimization::NoOpt => {
                self.write_attributes(attributes, spreads, false, brace, has_children)?;

                if !children.is_empty() {
                    self.write_body_indented(children)?;
                }

                self.out.tabbed_line()?;
            }
        }

        Ok(())
    }

    fn write_attributes(
        &mut self,
        attributes: &[Attribute],
        spreads: &[Spread],
        props_same_line: bool,
        brace: &Brace,
        has_children: bool,
    ) -> Result {
        enum AttrType<'a> {
            Attr(&'a Attribute),
            Spread(&'a Spread),
        }

        let mut attr_iter = attributes
            .iter()
            .map(AttrType::Attr)
            .chain(spreads.iter().map(AttrType::Spread))
            .peekable();

        while let Some(attr) = attr_iter.next() {
            self.out.indent_level += 1;

            if !props_same_line {
                self.write_attr_comments(
                    brace,
                    match attr {
                        AttrType::Attr(attr) => attr.span(),
                        AttrType::Spread(attr) => attr.expr.span(),
                    },
                )?;
            }

            self.out.indent_level -= 1;

            if !props_same_line {
                self.out.indented_tabbed_line()?;
            }

            match attr {
                AttrType::Attr(attr) => self.write_attribute(attr)?,
                AttrType::Spread(attr) => self.write_spread_attribute(&attr.expr)?,
            }

            if attr_iter.peek().is_some() {
                write!(self.out, ",")?;

                if props_same_line {
                    write!(self.out, " ")?;
                }
            }
        }

        let has_attributes = !attributes.is_empty() || !spreads.is_empty();

        if has_attributes && has_children {
            write!(self.out, ",")?;
        }

        Ok(())
    }

    fn write_attribute(&mut self, attr: &Attribute) -> Result {
        self.write_attribute_name(&attr.name)?;

        // if the attribute is a shorthand, we don't need to write the colon, just the name
        if !attr.can_be_shorthand() {
            write!(self.out, ": ")?;
            self.write_attribute_value(&attr.value)?;
        }

        Ok(())
    }

    fn write_attribute_name(&mut self, attr: &AttributeName) -> Result {
        match attr {
            AttributeName::BuiltIn(name) => {
                write!(self.out, "{}", name)?;
            }
            AttributeName::Custom(name) => {
                write!(self.out, "{}", name.to_token_stream())?;
            }
            AttributeName::Spread(_) => unreachable!(),
        }

        Ok(())
    }

    fn write_attribute_value(&mut self, value: &AttributeValue) -> Result {
        match value {
            AttributeValue::AttrOptionalExpr { condition, value } => {
                write!(
                    self.out,
                    "if {condition} {{ ",
                    condition = unparse_expr(condition),
                )?;
                self.write_attribute_value(value)?;
                write!(self.out, " }}")?;
            }
            AttributeValue::AttrLiteral(value) => {
                write!(self.out, "{value}")?;
            }
            AttributeValue::Shorthand(value) => {
                write!(self.out, "{value}")?;
            }
            AttributeValue::EventTokens(closure) => {
                self.write_partial_closure(closure)?;
            }

            AttributeValue::AttrExpr(value) => {
                let Ok(expr) = value.as_expr() else {
                    return Err(fmt::Error);
                };

                let pretty_expr = self.retrieve_formatted_expr(&expr).to_string();
                self.write_mulitiline_tokens(pretty_expr)?;
            }
        }

        Ok(())
    }

    fn write_mulitiline_tokens(&mut self, out: String) -> Result {
        let mut lines = out.split('\n').peekable();
        let first = lines.next().unwrap();

        // a one-liner for whatever reason
        // Does not need a new line
        if lines.peek().is_none() {
            write!(self.out, "{first}")?;
        } else {
            writeln!(self.out, "{first}")?;

            while let Some(line) = lines.next() {
                self.out.indented_tab()?;
                write!(self.out, "{line}")?;
                if lines.peek().is_none() {
                    write!(self.out, "")?;
                } else {
                    writeln!(self.out)?;
                }
            }
        }

        Ok(())
    }

    /// Write out the special PartialClosure type from the rsx crate
    /// Basically just write token by token until we hit the block and then try and format *that*
    /// We can't just ToTokens
    fn write_partial_closure(&mut self, closure: &PartialClosure) -> Result {
        // Write the pretty version of the closure
        if let Ok(expr) = closure.as_expr() {
            let pretty_expr = self.retrieve_formatted_expr(&expr).to_string();
            self.write_mulitiline_tokens(pretty_expr)?;
            return Ok(());
        }

        // If we can't parse the closure, writing it is also a failure
        // rustfmt won't be able to parse it either so no point in trying
        Err(fmt::Error)
    }

    fn write_spread_attribute(&mut self, attr: &Expr) -> Result {
        let formatted = unparse_expr(attr);

        let mut lines = formatted.lines();

        let first_line = lines.next().unwrap();

        write!(self.out, "..{first_line}")?;
        for line in lines {
            self.out.indented_tabbed_line()?;
            write!(self.out, "{line}")?;
        }

        Ok(())
    }

    // make sure the comments are actually relevant to this element.
    // test by making sure this element is the primary element on this line
    pub fn current_span_is_primary(&self, location: Span) -> bool {
        let start = location.start();
        let line_start = start.line - 1;

        let beginning = self
            .src
            .get(line_start)
            .filter(|this_line| this_line.len() > start.column)
            .map(|this_line| this_line[..start.column].trim())
            .unwrap_or_default();

        beginning.is_empty()
    }

    // check if the children are short enough to be on the same line
    // We don't have the notion of current line depth - each line tries to be < 80 total
    // returns the total line length if it's short
    // returns none if the length exceeds the limit
    // I think this eventually becomes quadratic :(
    pub fn is_short_children(&mut self, children: &[BodyNode]) -> Option<usize> {
        if children.is_empty() {
            // todo: allow elements with comments but no children
            // like div { /* comment */ }
            // or
            // div {
            //  // some helpful
            // }
            return Some(0);
        }

        // Any comments push us over the limit automatically
        if self.children_have_comments(children) {
            return None;
        }

        match children {
            [BodyNode::Text(ref text)] => Some(text.input.to_string_with_quotes().len()),

            // TODO: let rawexprs to be inlined
            [BodyNode::RawExpr(ref expr)] => Some(get_expr_length(expr.span())),

            // TODO: let rawexprs to be inlined
            [BodyNode::Component(ref comp)] if comp.fields.is_empty() => Some(
                comp.name
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string().len() + 2)
                    .sum::<usize>(),
            ),

            // Feedback on discord indicates folks don't like combining multiple children on the same line
            // We used to do a lot of math to figure out if we should expand out the line, but folks just
            // don't like it.
            _ => None,
        }
    }

    fn children_have_comments(&self, children: &[BodyNode]) -> bool {
        for child in children {
            if self.current_span_is_primary(child.span()) {
                'line: for line in self.src[..child.span().start().line - 1].iter().rev() {
                    match (line.trim().starts_with("//"), line.is_empty()) {
                        (true, _) => return true,
                        (_, true) => continue 'line,
                        _ => break 'line,
                    }
                }
            }
        }

        false
    }

    /// empty everything except for some comments
    fn write_todo_body(&mut self, brace: &Brace) -> fmt::Result {
        let span = brace.span.span();
        let start = span.start();
        let end = span.end();

        if start.line == end.line {
            return Ok(());
        }

        writeln!(self.out)?;

        for idx in start.line..end.line {
            let line = &self.src[idx];
            if line.trim().starts_with("//") {
                for _ in 0..self.out.indent_level + 1 {
                    write!(self.out, "    ")?
                }
                writeln!(self.out, "{}", line.trim()).unwrap();
            }
        }

        for _ in 0..self.out.indent_level {
            write!(self.out, "    ")?
        }

        Ok(())
    }
}

fn get_expr_length(span: Span) -> usize {
    let (start, end) = (span.start(), span.end());
    if start.line == end.line {
        end.column - start.column
    } else {
        10000
    }
}
