use crate::buffer::Buffer;
use dioxus_rsx::*;
use proc_macro2::{LineColumn, Span};
use quote::ToTokens;
use std::{
    collections::{HashMap, VecDeque},
    fmt::{Result, Write},
};
use syn::{spanned::Spanned, token::Brace, Expr};

#[derive(Debug)]
pub struct Writer<'a> {
    pub raw_src: &'a str,
    pub src: Vec<&'a str>,
    pub cached_formats: HashMap<LineColumn, String>,
    pub comments: VecDeque<usize>,
    pub out: Buffer,
    pub invalid_exprs: Vec<Span>,
}

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

impl<'a> Writer<'a> {
    pub fn new(raw_src: &'a str) -> Self {
        let src = raw_src.lines().collect();
        Self {
            raw_src,
            src,
            cached_formats: HashMap::new(),
            comments: VecDeque::new(),
            out: Buffer::default(),
            invalid_exprs: Vec::new(),
        }
    }

    pub fn consume(self) -> Option<String> {
        Some(self.out.buf)
    }

    pub fn write_rsx_call(&mut self, body: &TemplateBody) -> Result {
        match body.roots.len() {
            0 => {}
            1 if matches!(body.roots[0], BodyNode::Text(_)) => {
                write!(self.out, " ")?;
                self.write_ident(&body.roots[0])?;
                write!(self.out, " ")?;
            }
            _ => {
                self.out.new_line()?;
                self.write_body_indented(&body.roots)?
            }
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
        }
    }

    pub fn write_element(&mut self, el: &Element) -> Result {
        let Element {
            name,
            raw_attributes: attributes,
            children,
            spreads,
            brace,
            ..
        } = el;

        /*
            1. Write the tag
            2. Write the key
            3. Write the attributes
            4. Write the children
        */

        write!(self.out, "{name} {{")?;

        let brace = brace.unwrap_or_default();
        self.write_rsx_block(attributes, spreads, children, &brace)?;

        write!(self.out, "}}")?;

        Ok(())
    }

    pub fn write_component(
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

        write!(self.out, " {{")?;

        self.write_rsx_block(fields, spreads, &children.roots, brace)?;

        write!(self.out, "}}")?;

        Ok(())
    }

    pub fn write_text_node(&mut self, text: &TextNode) -> Result {
        self.out.write_text(&text.input)
    }

    pub fn write_expr_node(&mut self, expr: &ExprNode) -> Result {
        self.write_partial_expr(expr.expr.as_expr(), expr.span())
    }

    pub fn write_attr_comments(&mut self, brace: &Brace, attr_span: Span) -> Result {
        // There's a chance this line actually shares the same line as the previous
        // Only write comments if the comments actually belong to this line
        //
        // to do this, we check if the attr span starts on the same line as the brace
        // if it doesn't, we write the comments
        let brace_line = brace.span.span().start().line;
        let attr_line = attr_span.start().line;

        if brace_line != attr_line {
            self.write_comments(attr_span)?;
        }

        Ok(())
    }

    pub fn write_inline_comments(&mut self, final_span: LineColumn) -> Result {
        let line = final_span.line;
        let column = final_span.column;
        let mut whitespace = self.src[line - 1][column..].trim();

        if whitespace.is_empty() {
            return Ok(());
        }

        whitespace = whitespace[1..].trim();

        if whitespace.starts_with("//") {
            write!(self.out, " {whitespace}")?;
        }

        Ok(())
    }

    pub fn write_comments(&mut self, child: Span) -> Result {
        // collect all comments upwards
        // make sure we don't collect the comments of the node that we're currently under.
        let start = child.start();
        let line_start = start.line - 1;

        for (id, line) in self.src[..line_start].iter().enumerate().rev() {
            if line.trim().starts_with("//") || line.is_empty() && id != 0 {
                if id != 0 {
                    self.comments.push_front(id);
                }
            } else {
                break;
            }
        }

        while let Some(comment_line) = self.comments.pop_front() {
            let line = &self.src[comment_line].trim();

            if line.is_empty() {
                self.out.new_line()?;
            } else {
                self.out.tab()?;
                writeln!(self.out, "{}", line.trim())?;
            }
        }

        Ok(())
    }

    pub fn write_body_nodes(&mut self, children: &[BodyNode]) -> Result {
        let mut iter = children.iter().peekable();
        while let Some(child) = iter.next() {
            if self.current_span_is_primary(child.span()) {
                self.write_comments(child.span())?;
            };

            self.out.tab()?;
            self.write_ident(child)?;

            if iter.peek().is_some() {
                self.out.new_line()?;
            }
        }

        Ok(())
    }

    // Push out the indent level and write each component, line by line
    pub fn write_body_indented(&mut self, children: &[BodyNode]) -> Result {
        self.out.indent_level += 1;

        self.write_body_nodes(children)?;

        self.out.indent_level -= 1;
        Ok(())
    }

    pub fn write_body_no_indent(&mut self, children: &[BodyNode]) -> Result {
        self.write_body_nodes(children)?;

        Ok(())
    }

    pub(crate) fn attr_value_len(&mut self, value: &AttributeValue) -> usize {
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

    pub(crate) fn is_short_attrs(&mut self, attributes: &[Attribute], spreads: &[Spread]) -> usize {
        let mut total = 0;

        // No more than 3 attributes before breaking the line
        if attributes.len() > 3 {
            return 100000;
        }

        for attr in attributes {
            if self.current_span_is_primary(attr.span()) {
                'line: for line in self.src[..attr.span().start().line - 1].iter().rev() {
                    match (line.trim().starts_with("//"), line.is_empty()) {
                        (true, _) => return 100000,
                        (_, true) => continue 'line,
                        _ => break 'line,
                    }
                }
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

    #[allow(clippy::map_entry)]
    pub fn retrieve_formatted_expr(&mut self, expr: &Expr) -> &str {
        let loc = expr.span().start();

        if !self.cached_formats.contains_key(&loc) {
            let formatted = self.unparse_expr(expr);
            self.cached_formats.insert(loc, formatted);
        }

        self.cached_formats
            .get(&loc)
            .expect("Just inserted the parsed expr, so it should be in the cache")
            .as_str()
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
        let first_line = lines
            .next()
            .expect("All exprs should have at least one line");
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
        let children_len = self
            .is_short_children(children)
            .map_err(|_| std::fmt::Error)?;
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
            self.write_inline_comments(brace.span.span().start())?;
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
                // panic!("Props on top is not supported");
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
                self.write_inline_comments(brace.span.span().start())?;
                self.out.new_line()?;
                self.write_attributes(attributes, spreads, false, brace, has_children)?;

                if !children.is_empty() {
                    self.out.new_line()?;
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
                AttrType::Attr(attr) => attr.value.span(),
                AttrType::Spread(attr) => attr.span(),
            };

            let has_more = attr_iter.peek().is_some();
            let should_finish_comma = has_attributes && has_children;

            if has_more || should_finish_comma {
                write!(self.out, ",")?;
            }

            if !props_same_line {
                self.write_inline_comments(span.end())?;
            }

            if props_same_line && !has_more {
                self.write_inline_comments(span.end())?;
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
                self.write_partial_closure(closure)?;
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
                write!(self.out, "else ")?;
                self.write_attribute_if_chain(else_if_chain)?;
            }
            Some(other) => {
                write!(self.out, "else {{")?;
                self.write_attribute_value(other)?;
                write!(self.out, " }}")?;
            }
            None => {}
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
                self.out.tab()?;
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
        // push out the indent level of the body of the closure
        // This ensures it doesnt get written to the same level as the parent
        self.write_partial_expr(closure.as_expr(), closure.span())?;
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
    pub fn is_short_children(&mut self, children: &[BodyNode]) -> syn::Result<Option<usize>> {
        if children.is_empty() {
            // todo: allow elements with comments but no children
            // like div { /* comment */ }
            // or
            // div {
            //  // some helpful
            // }
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
        };

        Ok(res)
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
    fn write_todo_body(&mut self, brace: &Brace) -> std::fmt::Result {
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
                writeln!(self.out, "{}", line.trim())?;
            }
        }

        for _ in 0..self.out.indent_level {
            write!(self.out, "    ")?
        }

        Ok(())
    }

    pub(crate) fn write_partial_expr(&mut self, expr: syn::Result<Expr>, src_span: Span) -> Result {
        let Ok(expr) = expr else {
            self.invalid_exprs.push(src_span);
            return Err(std::fmt::Error);
        };

        let pretty_expr = self.retrieve_formatted_expr(&expr).to_string();
        self.write_mulitiline_tokens(pretty_expr)?;

        Ok(())
    }
}
