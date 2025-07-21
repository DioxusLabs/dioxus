use crate::{buffer::Buffer, IndentOptions};
use dioxus_rsx::*;
use proc_macro2::{LineColumn, Span};
use quote::ToTokens;
use regex::Regex;
use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    fmt::{Result, Write},
};
use syn::{spanned::Spanned, token::Brace, Expr};

#[derive(Debug)]
pub struct Writer<'a> {
    pub raw_src: &'a str,
    pub src: Vec<&'a str>,
    pub cached_formats: HashMap<LineColumn, String>,
    pub out: Buffer,
    pub invalid_exprs: Vec<Span>,
}

impl<'a> Writer<'a> {
    pub fn new(raw_src: &'a str, indent: IndentOptions) -> Self {
        Self {
            src: raw_src.lines().collect(),
            raw_src,
            out: Buffer {
                indent,
                ..Default::default()
            },
            cached_formats: HashMap::new(),
            invalid_exprs: Vec::new(),
        }
    }

    pub fn consume(self) -> Option<String> {
        Some(self.out.buf)
    }

    pub fn write_rsx_call(&mut self, body: &CallBody) -> Result {
        if body.body.roots.is_empty() {
            return Ok(());
        }

        if Self::is_short_rsx_call(&body.body.roots) {
            write!(self.out, " ")?;
            self.write_ident(&body.body.roots[0])?;
            write!(self.out, " ")?;
        } else {
            self.out.new_line()?;
            self.write_body_indented(&body.body.roots)?;
            self.write_trailing_body_comments(body)?;
        }

        Ok(())
    }

    fn write_trailing_body_comments(&mut self, body: &CallBody) -> Result {
        if let Some(span) = body.span {
            self.out.indent_level += 1;
            let comments = self.accumulate_comments(span.span().end());
            if !comments.is_empty() {
                self.out.new_line()?;
                self.apply_comments(comments)?;
                self.out.buf.pop(); // remove the trailing newline, forcing us to end at the end of the comment
            }
            self.out.indent_level -= 1;
        }
        Ok(())
    }

    // Expects to be written directly into place
    pub fn write_ident(&mut self, node: &BodyNode) -> Result {
        match node {
            BodyNode::Element(el) => self.write_element(el),
            BodyNode::Component(component) => self.write_component(component),
            BodyNode::Text(text) => self.write_text_node(text),
            BodyNode::RawExpr(expr) => self.write_expr_node(expr),
            BodyNode::ForLoop(forloop) => self.write_for_loop(forloop),
            BodyNode::IfChain(ifchain) => self.write_if_chain(ifchain),
        }?;

        let span = Self::final_span_of_node(node);

        self.write_inline_comments(span.end(), 0)?;

        Ok(())
    }

    /// Check if the rsx call is short enough to be inlined
    pub(crate) fn is_short_rsx_call(roots: &[BodyNode]) -> bool {
        // eventually I want to use the _text length, so shutup now
        #[allow(clippy::match_like_matches_macro)]
        match roots {
            [] => true,
            [BodyNode::Text(_text)] => true,
            _ => false,
        }
    }

    fn write_element(&mut self, el: &Element) -> Result {
        let Element {
            name,
            raw_attributes: attributes,
            children,
            spreads,
            brace,
            ..
        } = el;

        write!(self.out, "{name} ")?;
        self.write_rsx_block(attributes, spreads, children, &brace.unwrap_or_default())?;

        Ok(())
    }

    fn write_component(
        &mut self,
        Component {
            name,
            fields,
            children,
            generics,
            spreads,
            brace,
            ..
        }: &Component,
    ) -> Result {
        // Write the path by to_tokensing it and then removing all whitespace
        let mut name = name.to_token_stream().to_string();
        name.retain(|c| !c.is_whitespace());
        write!(self.out, "{name}")?;

        // Same idea with generics, write those via the to_tokens method and then remove all whitespace
        if let Some(generics) = generics {
            let mut written = generics.to_token_stream().to_string();
            written.retain(|c| !c.is_whitespace());
            write!(self.out, "{written}")?;
        }

        write!(self.out, " ")?;
        self.write_rsx_block(fields, spreads, &children.roots, &brace.unwrap_or_default())?;

        Ok(())
    }

    fn write_text_node(&mut self, text: &TextNode) -> Result {
        self.out.write_text(&text.input)
    }

    fn write_expr_node(&mut self, expr: &ExprNode) -> Result {
        self.write_partial_expr(expr.expr.as_expr(), expr.span())
    }

    fn write_for_loop(&mut self, forloop: &ForLoop) -> std::fmt::Result {
        write!(
            self.out,
            "for {} in ",
            forloop.pat.clone().into_token_stream(),
        )?;

        self.write_inline_expr(&forloop.expr)?;

        if forloop.body.is_empty() {
            write!(self.out, "}}")?;
            return Ok(());
        }

        self.out.new_line()?;
        self.write_body_indented(&forloop.body.roots)?;

        self.out.tabbed_line()?;
        write!(self.out, "}}")?;

        Ok(())
    }

    fn write_if_chain(&mut self, ifchain: &IfChain) -> std::fmt::Result {
        // Recurse in place by setting the next chain
        let mut branch = Some(ifchain);

        while let Some(chain) = branch {
            let IfChain {
                if_token,
                cond,
                then_branch,
                else_if_branch,
                else_branch,
                ..
            } = chain;

            write!(self.out, "{} ", if_token.to_token_stream(),)?;

            self.write_inline_expr(cond)?;

            self.out.new_line()?;
            self.write_body_indented(&then_branch.roots)?;

            if let Some(else_if_branch) = else_if_branch {
                // write the closing bracket and else
                self.out.tabbed_line()?;
                write!(self.out, "}} else ")?;

                branch = Some(else_if_branch);
            } else if let Some(else_branch) = else_branch {
                self.out.tabbed_line()?;
                write!(self.out, "}} else {{")?;

                self.out.new_line()?;
                self.write_body_indented(&else_branch.roots)?;
                branch = None;
            } else {
                branch = None;
            }
        }

        self.out.tabbed_line()?;
        write!(self.out, "}}")?;

        Ok(())
    }

    /// An expression within a for or if block that might need to be spread out across several lines
    fn write_inline_expr(&mut self, expr: &Expr) -> std::fmt::Result {
        let unparsed = self.unparse_expr(expr);
        let mut lines = unparsed.lines();
        let first_line = lines.next().ok_or(std::fmt::Error)?;

        write!(self.out, "{first_line}")?;

        let mut was_multiline = false;

        for line in lines {
            was_multiline = true;
            self.out.tabbed_line()?;
            write!(self.out, "{line}")?;
        }

        if was_multiline {
            self.out.tabbed_line()?;
            write!(self.out, "{{")?;
        } else {
            write!(self.out, " {{")?;
        }

        Ok(())
    }

    // Push out the indent level and write each component, line by line
    fn write_body_indented(&mut self, children: &[BodyNode]) -> Result {
        self.out.indent_level += 1;
        self.write_body_nodes(children)?;
        self.out.indent_level -= 1;
        Ok(())
    }

    pub fn write_body_nodes(&mut self, children: &[BodyNode]) -> Result {
        let mut iter = children.iter().peekable();

        while let Some(child) = iter.next() {
            if self.current_span_is_primary(child.span().start()) {
                self.write_comments(child.span().start())?;
            };
            self.out.tab()?;
            self.write_ident(child)?;
            if iter.peek().is_some() {
                self.out.new_line()?;
            }
        }

        Ok(())
    }

    /// Basically elements and components are the same thing
    ///
    /// This writes the contents out for both in one function, centralizing the annoying logic like
    /// key handling, breaks, closures, etc
    fn write_rsx_block(
        &mut self,
        attributes: &[Attribute],
        spreads: &[Spread],
        children: &[BodyNode],
        brace: &Brace,
    ) -> Result {
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

        // Write the opening brace
        write!(self.out, "{{")?;

        // decide if we have any special optimizations
        // Default with none, opt the cases in one-by-one
        let mut opt_level = ShortOptimization::NoOpt;

        // check if we have a lot of attributes
        let attr_len = self.is_short_attrs(attributes, spreads);
        let is_short_attr_list = (attr_len + self.out.indent_level * 4) < 80;
        let children_len = self
            .is_short_children(children)
            .map_err(|_| std::fmt::Error)?;
        let has_trailing_comments = self.has_trailing_comments(children, brace);
        let is_small_children = children_len.is_some() && !has_trailing_comments;

        // if we have one long attribute and a lot of children, place the attrs on top
        if is_short_attr_list && !is_small_children {
            opt_level = ShortOptimization::PropsOnTop;
        }

        // even if the attr is long, it should be put on one line
        // However if we have childrne we need to just spread them out for readability
        if !is_short_attr_list
            && attributes.len() <= 1
            && spreads.is_empty()
            && !has_trailing_comments
        {
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
        if attributes.is_empty()
            && children.is_empty()
            && spreads.is_empty()
            && !has_trailing_comments
        {
            opt_level = ShortOptimization::Empty;

            // Write comments if they exist
            self.write_inline_comments(brace.span.span().start(), 1)?;
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

                let mut children_iter = children.iter().peekable();
                while let Some(child) = children_iter.next() {
                    self.write_ident(child)?;
                    if children_iter.peek().is_some() {
                        write!(self.out, " ")?;
                    }
                }

                write!(self.out, " ")?;
            }

            ShortOptimization::PropsOnTop => {
                if !attributes.is_empty() {
                    write!(self.out, " ")?;
                }

                self.write_attributes(attributes, spreads, true, brace, has_children)?;

                if !children.is_empty() {
                    self.out.new_line()?;
                    self.write_body_indented(children)?;
                }

                self.out.tabbed_line()?;
            }

            ShortOptimization::NoOpt => {
                self.write_inline_comments(brace.span.span().start(), 1)?;
                self.out.new_line()?;
                self.write_attributes(attributes, spreads, false, brace, has_children)?;

                if !children.is_empty() {
                    self.out.new_line()?;
                    self.write_body_indented(children)?;
                }

                self.out.tabbed_line()?;
            }
        }

        // Write trailing comments
        if matches!(
            opt_level,
            ShortOptimization::NoOpt | ShortOptimization::PropsOnTop
        ) && self.leading_row_is_empty(brace.span.span().end())
        {
            let comments = self.accumulate_comments(brace.span.span().end());
            if !comments.is_empty() {
                self.apply_comments(comments)?;
                self.out.tab()?;
            }
        }

        write!(self.out, "}}")?;

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

        let has_attributes = !attributes.is_empty() || !spreads.is_empty();

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
                self.out.indented_tab()?;
            }

            match attr {
                AttrType::Attr(attr) => self.write_attribute(attr)?,
                AttrType::Spread(attr) => self.write_spread_attribute(&attr.expr)?,
            }

            let span = match attr {
                AttrType::Attr(attr) => attr
                    .comma
                    .as_ref()
                    .map(|c| c.span())
                    .unwrap_or_else(|| self.final_span_of_attr(attr)),
                AttrType::Spread(attr) => attr.span(),
            };

            let has_more = attr_iter.peek().is_some();
            let should_finish_comma = has_attributes && has_children || !props_same_line;

            if has_more || should_finish_comma {
                write!(self.out, ",")?;
            }

            if !props_same_line {
                self.write_inline_comments(span.end(), 0)?;
            }

            if props_same_line && !has_more {
                self.write_inline_comments(span.end(), 0)?;
            }

            if props_same_line && has_more {
                write!(self.out, " ")?;
            }

            if !props_same_line && has_more {
                self.out.new_line()?;
            }
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
            AttributeName::BuiltIn(name) => write!(self.out, "{}", name),
            AttributeName::Custom(name) => write!(self.out, "{}", name.to_token_stream()),
            AttributeName::Spread(_) => unreachable!(),
        }
    }

    fn write_attribute_value(&mut self, value: &AttributeValue) -> Result {
        match value {
            AttributeValue::IfExpr(if_chain) => {
                self.write_attribute_if_chain(if_chain)?;
            }
            AttributeValue::AttrLiteral(value) => {
                write!(self.out, "{value}")?;
            }
            AttributeValue::Shorthand(value) => {
                write!(self.out, "{value}")?;
            }
            AttributeValue::EventTokens(closure) => {
                self.out.indent_level += 1;
                self.write_partial_expr(closure.as_expr(), closure.span())?;
                self.out.indent_level -= 1;
            }
            AttributeValue::AttrExpr(value) => {
                self.out.indent_level += 1;
                self.write_partial_expr(value.as_expr(), value.span())?;
                self.out.indent_level -= 1;
            }
        }

        Ok(())
    }

    fn write_attribute_if_chain(&mut self, if_chain: &IfAttributeValue) -> Result {
        let cond = self.unparse_expr(&if_chain.condition);
        write!(self.out, "if {cond} {{ ")?;
        self.write_attribute_value(&if_chain.then_value)?;
        write!(self.out, " }}")?;
        match if_chain.else_value.as_deref() {
            Some(AttributeValue::IfExpr(else_if_chain)) => {
                write!(self.out, " else ")?;
                self.write_attribute_if_chain(else_if_chain)?;
            }
            Some(other) => {
                write!(self.out, " else {{ ")?;
                self.write_attribute_value(other)?;
                write!(self.out, " }}")?;
            }
            None => {}
        }

        Ok(())
    }

    fn write_attr_comments(&mut self, brace: &Brace, attr_span: Span) -> Result {
        // There's a chance this line actually shares the same line as the previous
        // Only write comments if the comments actually belong to this line
        //
        // to do this, we check if the attr span starts on the same line as the brace
        // if it doesn't, we write the comments
        let brace_line = brace.span.span().start().line;
        let attr_line = attr_span.start().line;

        if brace_line != attr_line {
            self.write_comments(attr_span.start())?;
        }

        Ok(())
    }

    fn write_inline_comments(&mut self, final_span: LineColumn, offset: usize) -> Result {
        let line = final_span.line;
        let column = final_span.column;
        let Some(src_line) = self.src.get(line - 1) else {
            return Ok(());
        };

        // the line might contain emoji or other unicode characters - this will cause issues
        let Some(mut whitespace) = src_line.get(column..).map(|s| s.trim()) else {
            return Ok(());
        };

        if whitespace.is_empty() {
            return Ok(());
        }

        whitespace = whitespace[offset..].trim();

        if whitespace.starts_with("//") {
            write!(self.out, " {whitespace}")?;
        }

        Ok(())
    }
    fn accumulate_comments(&mut self, loc: LineColumn) -> VecDeque<usize> {
        // collect all comments upwards
        // make sure we don't collect the comments of the node that we're currently under.
        let start = loc;
        let line_start = start.line - 1;

        let mut comments = VecDeque::new();

        let Some(lines) = self.src.get(..line_start) else {
            return comments;
        };

        for (id, line) in lines.iter().enumerate().rev() {
            if line.trim().starts_with("//") || line.is_empty() && id != 0 {
                if id != 0 {
                    comments.push_front(id);
                }
            } else {
                break;
            }
        }

        comments
    }
    fn apply_comments(&mut self, mut comments: VecDeque<usize>) -> Result {
        while let Some(comment_line) = comments.pop_front() {
            let Some(line) = self.src.get(comment_line) else {
                continue;
            };

            let line = &line.trim();

            if line.is_empty() {
                self.out.new_line()?;
            } else {
                self.out.tab()?;
                writeln!(self.out, "{}", line.trim())?;
            }
        }
        Ok(())
    }

    fn write_comments(&mut self, loc: LineColumn) -> Result {
        let comments = self.accumulate_comments(loc);
        self.apply_comments(comments)?;

        Ok(())
    }

    fn attr_value_len(&mut self, value: &AttributeValue) -> usize {
        match value {
            AttributeValue::IfExpr(if_chain) => {
                let condition_len = self.retrieve_formatted_expr(&if_chain.condition).len();
                let value_len = self.attr_value_len(&if_chain.then_value);
                let if_len = 2;
                let brace_len = 2;
                let space_len = 2;
                let else_len = if_chain
                    .else_value
                    .as_ref()
                    .map(|else_value| self.attr_value_len(else_value) + 1)
                    .unwrap_or_default();
                condition_len + value_len + if_len + brace_len + space_len + else_len
            }
            AttributeValue::AttrLiteral(lit) => lit.to_string().len(),
            AttributeValue::Shorthand(expr) => {
                let span = &expr.span();
                span.end().line - span.start().line
            }
            AttributeValue::AttrExpr(expr) => expr
                .as_expr()
                .map(|expr| self.attr_expr_len(&expr))
                .unwrap_or(100000),
            AttributeValue::EventTokens(closure) => closure
                .as_expr()
                .map(|expr| self.attr_expr_len(&expr))
                .unwrap_or(100000),
        }
    }

    fn attr_expr_len(&mut self, expr: &Expr) -> usize {
        let out = self.retrieve_formatted_expr(expr);
        if out.contains('\n') {
            100000
        } else {
            out.len()
        }
    }

    fn is_short_attrs(&mut self, attributes: &[Attribute], spreads: &[Spread]) -> usize {
        let mut total = 0;

        // No more than 3 attributes before breaking the line
        if attributes.len() > 3 {
            return 100000;
        }

        for attr in attributes {
            if self.current_span_is_primary(attr.span().start()) {
                if let Some(lines) = self.src.get(..attr.span().start().line - 1) {
                    'line: for line in lines.iter().rev() {
                        match (line.trim().starts_with("//"), line.is_empty()) {
                            (true, _) => return 100000,
                            (_, true) => continue 'line,
                            _ => break 'line,
                        }
                    }
                };
            }

            let name_len = match &attr.name {
                AttributeName::BuiltIn(name) => {
                    let name = name.to_string();
                    name.len()
                }
                AttributeName::Custom(name) => name.value().len() + 2,
                AttributeName::Spread(_) => unreachable!(),
            };
            total += name_len;

            if attr.can_be_shorthand() {
                total += 2;
            } else {
                total += self.attr_value_len(&attr.value);
            }

            total += 6;
        }

        for spread in spreads {
            let expr_len = self.retrieve_formatted_expr(&spread.expr).len();
            total += expr_len + 3;
        }

        total
    }

    fn write_todo_body(&mut self, brace: &Brace) -> std::fmt::Result {
        let span = brace.span.span();
        let start = span.start();
        let end = span.end();

        if start.line == end.line {
            return Ok(());
        }

        writeln!(self.out)?;

        for idx in start.line..end.line {
            let Some(line) = self.src.get(idx) else {
                continue;
            };
            if line.trim().starts_with("//") {
                for _ in 0..self.out.indent_level + 1 {
                    write!(self.out, "    ")?
                }
                writeln!(self.out, "{}", line.trim())?;
            }
        }

        for _ in 0..self.out.indent_level {
            write!(self.out, "    ")?
        }

        Ok(())
    }

    fn write_partial_expr(&mut self, expr: syn::Result<Expr>, src_span: Span) -> Result {
        let Ok(expr) = expr else {
            self.invalid_exprs.push(src_span);
            return Err(std::fmt::Error);
        };

        thread_local! {
            static COMMENT_REGEX: Regex = Regex::new("\"[^\"]*\"|(//.*)").unwrap();
        }

        let pretty_expr = self.retrieve_formatted_expr(&expr).to_string();

        // Adding comments back to the formatted expression
        let source_text = src_span.source_text().unwrap_or_default();
        let mut source_lines = source_text.lines().peekable();
        let mut output = String::from("");
        let mut printed_empty_line = false;

        if source_lines.peek().is_none() {
            output = pretty_expr;
        } else {
            for line in pretty_expr.lines() {
                let compacted_pretty_line = line.replace(" ", "").replace(",", "");
                let trimmed_pretty_line = line.trim();

                // Nested expressions might have comments already. We handle writing all of those
                // at the outer level, so we skip them here
                if trimmed_pretty_line.starts_with("//") {
                    continue;
                }

                if !output.is_empty() {
                    output.push('\n');
                }

                // pull down any source lines with whitespace until we hit a line that matches our current line.
                while let Some(src) = source_lines.peek() {
                    let trimmed_src = src.trim();

                    // Write comments and empty lines as they are
                    if trimmed_src.starts_with("//") || trimmed_src.is_empty() {
                        if !trimmed_src.is_empty() {
                            // Match the whitespace of the incoming source line
                            for s in line.chars().take_while(|c| c.is_whitespace()) {
                                output.push(s);
                            }

                            // Bump out the indent level if the line starts with a closing brace (ie we're at the end of a block)
                            if matches!(trimmed_pretty_line.chars().next(), Some(')' | '}' | ']')) {
                                output.push_str(self.out.indent.indent_str());
                            }

                            printed_empty_line = false;
                            output.push_str(trimmed_src);
                            output.push('\n');
                        } else if !printed_empty_line {
                            output.push('\n');
                            printed_empty_line = true;
                        }

                        _ = source_lines.next();
                        continue;
                    }

                    let compacted_src_line = src.replace(" ", "").replace(",", "");

                    // If this source line matches our pretty line, we stop pulling down
                    if compacted_src_line.contains(&compacted_pretty_line) {
                        break;
                    }

                    // Otherwise, consume this source line and keep going
                    _ = source_lines.next();
                }

                // Once all whitespace is written, write the pretty line
                output.push_str(line);
                printed_empty_line = false;

                // And then pull the corresponding source line
                let source_line = source_lines.next();

                // And then write any inline comments
                if let Some(source_line) = source_line {
                    if let Some(captures) = COMMENT_REGEX.with(|f| f.captures(source_line)) {
                        if let Some(comment) = captures.get(1) {
                            output.push_str(" // ");
                            output.push_str(comment.as_str().replace("//", "").trim());
                        }
                    }
                }
            }
        }

        self.write_mulitiline_tokens(output)?;

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
                if !line.trim().is_empty() {
                    self.out.tab()?;
                }

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

    fn write_spread_attribute(&mut self, attr: &Expr) -> Result {
        let formatted = self.unparse_expr(attr);

        let mut lines = formatted.lines();

        let first_line = lines.next().unwrap();

        write!(self.out, "..{first_line}")?;
        for line in lines {
            self.out.indented_tabbed_line()?;
            write!(self.out, "{line}")?;
        }

        Ok(())
    }

    // check if the children are short enough to be on the same line
    // We don't have the notion of current line depth - each line tries to be < 80 total
    // returns the total line length if it's short
    // returns none if the length exceeds the limit
    // I think this eventually becomes quadratic :(
    fn is_short_children(&mut self, children: &[BodyNode]) -> syn::Result<Option<usize>> {
        if children.is_empty() {
            return Ok(Some(0));
        }

        // Any comments push us over the limit automatically
        if self.children_have_comments(children) {
            return Ok(None);
        }

        let res = match children {
            [BodyNode::Text(ref text)] => Some(text.input.to_string_with_quotes().len()),

            // TODO: let rawexprs to be inlined
            [BodyNode::RawExpr(ref expr)] => {
                let pretty = self.retrieve_formatted_expr(&expr.expr.as_expr()?);
                if pretty.contains('\n') {
                    None
                } else {
                    Some(pretty.len() + 2)
                }
            }

            // TODO: let rawexprs to be inlined
            [BodyNode::Component(ref comp)]
            // basically if the component is completely empty, we can inline it
                if comp.fields.is_empty()
                    && comp.children.is_empty()
                    && comp.spreads.is_empty() =>
            {
                Some(
                    comp.name
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string().len() + 2)
                        .sum::<usize>(),
                )
            }

            // Feedback on discord indicates folks don't like combining multiple children on the same line
            // We used to do a lot of math to figure out if we should expand out the line, but folks just
            // don't like it.
            _ => None,
        };

        Ok(res)
    }

    fn children_have_comments(&self, children: &[BodyNode]) -> bool {
        for child in children {
            if self.current_span_is_primary(child.span().start()) {
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

    // make sure the comments are actually relevant to this element.
    // test by making sure this element is the primary element on this line (nothing else before it)
    fn current_span_is_primary(&self, location: LineColumn) -> bool {
        self.leading_row_is_empty(LineColumn {
            line: location.line,
            column: location.column + 1,
        })
    }

    fn leading_row_is_empty(&self, location: LineColumn) -> bool {
        let Some(line) = self.src.get(location.line - 1) else {
            return false;
        };

        let Some(sub) = line.get(..location.column - 1) else {
            return false;
        };

        sub.trim().is_empty()
    }

    #[allow(clippy::map_entry)]
    fn retrieve_formatted_expr(&mut self, expr: &Expr) -> Cow<'_, str> {
        let loc = expr.span().start();

        // never cache expressions that are spanless
        if loc.line == 1 && loc.column == 0 {
            return self.unparse_expr(expr).into();
        }

        if !self.cached_formats.contains_key(&loc) {
            let formatted = self.unparse_expr(expr);
            self.cached_formats.insert(loc, formatted);
        }

        self.cached_formats
            .get(&loc)
            .expect("Just inserted the parsed expr, so it should be in the cache")
            .as_str()
            .into()
    }

    fn final_span_of_node(node: &BodyNode) -> Span {
        // Get the ending span of the node
        match node {
            BodyNode::Element(el) => el
                .brace
                .as_ref()
                .map(|b| b.span.span())
                .unwrap_or_else(|| el.name.span()),
            BodyNode::Component(el) => el
                .brace
                .as_ref()
                .map(|b| b.span.span())
                .unwrap_or_else(|| el.name.span()),
            BodyNode::Text(txt) => txt.input.span(),
            BodyNode::RawExpr(exp) => exp.span(),
            BodyNode::ForLoop(f) => f.brace.span.span(),
            BodyNode::IfChain(i) => match i.else_brace {
                Some(b) => b.span.span(),
                None => i.then_brace.span.span(),
            },
        }
    }

    fn final_span_of_attr(&self, attr: &Attribute) -> Span {
        match &attr.value {
            AttributeValue::Shorthand(s) => s.span(),
            AttributeValue::AttrLiteral(l) => l.span(),
            AttributeValue::EventTokens(closure) => closure.body.span(),
            AttributeValue::AttrExpr(exp) => exp.span(),
            AttributeValue::IfExpr(ex) => ex
                .else_value
                .as_ref()
                .map(|v| v.span())
                .unwrap_or_else(|| ex.then_value.span()),
        }
    }

    fn has_trailing_comments(&self, children: &[BodyNode], brace: &Brace) -> bool {
        let brace_span = brace.span.span();

        let Some(last_node) = children.last() else {
            return false;
        };

        // Check for any comments after the last node between the last brace
        let final_span = Self::final_span_of_node(last_node);
        let final_span = final_span.end();
        let mut line = final_span.line;
        let mut column = final_span.column;
        loop {
            let Some(src_line) = self.src.get(line - 1) else {
                return false;
            };

            // the line might contain emoji or other unicode characters - this will cause issues
            let Some(mut whitespace) = src_line.get(column..).map(|s| s.trim()) else {
                return false;
            };

            let offset = 0;
            whitespace = whitespace[offset..].trim();

            if whitespace.starts_with("//") {
                return true;
            }

            if line == brace_span.end().line {
                // If we reached the end of the brace span, stop
                break;
            }

            line += 1;
            column = 0; // reset column to the start of the next line
        }

        false
    }
}
