use crate::{IndentOptions, buffer::Buffer};
use dioxus_rsx::*;
use proc_macro2::{LineColumn, Span};
use quote::ToTokens;
use regex::Regex;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
    fmt::{Result, Write},
};
use syn::{Expr, spanned::Spanned, token::Brace};

/// The source locations of the delimiters around a body: the braces of an element, a component,
/// or a `for`/`if` block, or the tags of the tag syntax (`<div> </div>`)
///
/// Comments are not part of the parsed rsx, so they are recovered by looking at the source text
/// around these locations.
#[derive(Debug, Clone, Copy)]
struct BodyDelimiters {
    /// The start of the token that opens the body: `{`, or the `>` (`/` if self-closing) of a tag
    open: LineColumn,

    /// The end of the token that starts the closing delimiter: `}`, or the `<` of a closing tag
    close: LineColumn,

    /// The end of the name that the body belongs to, if comments could be written between the two
    name_end: Option<LineColumn>,

    /// Whether the node was written in the tag syntax
    is_tag: bool,
}

impl BodyDelimiters {
    /// The delimiters of a node whose name ends at `name_end`
    ///
    /// Returns `None` for tags that are incomplete, which can't be formatted
    fn new(delimiter: &NodeDelimiter, name_end: LineColumn) -> Option<Self> {
        match delimiter {
            NodeDelimiter::Brace(brace) => Some(Self {
                name_end: Some(name_end),
                ..brace.into()
            }),
            NodeDelimiter::Missing => Some((&Brace::default()).into()),
            NodeDelimiter::Tag(tag) if tag.is_complete() => {
                let gt = tag.gt?;
                Some(Self {
                    open: match tag.slash {
                        Some(slash) => slash.span.start(),
                        None => gt.span.start(),
                    },
                    close: match &tag.close {
                        Some(close) => close.lt.span.end(),
                        None => gt.span.end(),
                    },
                    name_end: Some(name_end),
                    is_tag: true,
                })
            }
            NodeDelimiter::Tag(_) => None,
        }
    }
}

impl From<&Brace> for BodyDelimiters {
    fn from(brace: &Brace) -> Self {
        let span = brace.span.span();
        Self {
            open: span.start(),
            close: span.end(),
            name_end: None,
            is_tag: false,
        }
    }
}

/// Where the attributes and spreads of an element or component have comments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttrComments {
    None,

    /// Only after the last attribute, on the same line
    AfterLast,

    /// Above an attribute, or after one that isn't the last
    Any,
}

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
        if body.body().roots.is_empty() {
            return self.write_trailing_body_comments(body);
        }

        if Self::is_short_rsx_call(&body.body().roots) {
            write!(self.out, " ")?;
            self.write_ident(&body.body().roots[0])?;
            write!(self.out, " ")?;
        } else {
            self.out.new_line()?;
            self.write_body_indented(&body.body().roots)?;
            self.write_trailing_body_comments(body)?;
        }

        Ok(())
    }

    fn write_trailing_body_comments(&mut self, body: &CallBody) -> Result {
        // The comments are on the lines above the closing delimiter of the macro. If anything else
        // is on its line then the lines above it are not part of the body.
        if let Some(span) = body.span()
            && self.leading_row_is_empty(span.end())
        {
            self.out.indent_level += 1;
            let comments = self.accumulate_full_line_comments(span.end());
            if self.has_real_comment(&comments) {
                self.out.new_line()?;
                self.apply_line_comments(comments)?;
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
            BodyNode::SyntheticBoundary(body) => self.write_synthetic_boundary(body),
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
            delimiter,
            ..
        } = el;

        let delimiters =
            BodyDelimiters::new(delimiter, name.span().end()).ok_or(std::fmt::Error)?;

        if delimiters.is_tag {
            return self.write_tag(&name.to_string(), attributes, spreads, children, delimiters);
        }

        write!(self.out, "{name} ")?;
        self.write_rsx_block(attributes, spreads, children, delimiters)?;

        Ok(())
    }

    fn write_component(
        &mut self,
        Component {
            name: path,
            fields,
            children,
            generics,
            spreads,
            delimiter,
            ..
        }: &Component,
    ) -> Result {
        let name_end = match generics {
            Some(generics) => generics.gt_token.span().end(),
            None => path.span().end(),
        };
        let delimiters = BodyDelimiters::new(delimiter, name_end).ok_or(std::fmt::Error)?;

        // Write the path by to_tokensing it and then removing all whitespace
        let mut name = path.to_token_stream().to_string();
        name.retain(|c| !c.is_whitespace());

        // Same idea with generics, write those via the to_tokens method and then remove all whitespace
        if let Some(generics) = generics {
            let mut written = generics.to_token_stream().to_string();
            written.retain(|c| !c.is_whitespace());

            // The tag syntax doesn't use the turbofish: `<Outlet<R>>`
            if delimiters.is_tag {
                written = written.trim_start_matches("::").to_string();
            }

            name.push_str(&written);
        }

        if delimiters.is_tag {
            return self.write_tag(&name, fields, spreads, &children.roots, delimiters);
        }

        write!(self.out, "{name} ")?;
        self.write_rsx_block(fields, spreads, &children.roots, delimiters)?;

        Ok(())
    }

    /// Write an element or component that was written in the tag syntax, keeping it in that
    /// syntax: `<div class="a">"hello"</div>` or `<img src="..." />`
    fn write_tag(
        &mut self,
        name: &str,
        attributes: &[Attribute],
        spreads: &[Spread],
        children: &[BodyNode],
        delimiters: BodyDelimiters,
    ) -> Result {
        enum AttrType<'a> {
            Attr(&'a Attribute),
            Spread(&'a Spread),
        }

        write!(self.out, "<{name}")?;

        // A comment after the name stays there, with the rest of the open tag on the lines below
        let name_comment = delimiters
            .name_end
            .and_then(|name_end| self.inline_comment(name_end, 0));
        if let Some(comment) = name_comment {
            write!(self.out, " {comment}")?;
        }

        let attrs: Vec<_> = attributes
            .iter()
            .map(AttrType::Attr)
            .chain(spreads.iter().map(AttrType::Spread))
            .collect();

        // The start and end of each attribute in the source, which is where its comments are
        let attr_spans: Vec<_> = attrs
            .iter()
            .map(|attr| match attr {
                AttrType::Attr(attr) => (attr.span().start(), self.end_of_attr(attr, delimiters)),
                AttrType::Spread(spread) => (
                    self.start_of_tag_spread(spread),
                    self.end_of_spread(spread, delimiters),
                ),
            })
            .collect();
        let has_attr_comments = name_comment.is_some()
            || attr_spans.iter().any(|(start, end)| {
                self.has_leading_comments(*start) || self.inline_comment(*end, 0).is_some()
            })
            || attributes
                .iter()
                .any(|attr| !self.attr_value_comments(attr).is_empty());

        // Decide if the attributes fit in the open tag or need to be split across lines.
        // Comments on attributes can only be kept if each attribute is on its own line.
        let attr_len = self.is_short_attrs(attributes, spreads);
        let is_short_attr_list = (attr_len + self.out.indent_level * 4) < 80
            && !self.out.indent.split_line_attributes()
            && !has_attr_comments;

        for (attr, (start, end)) in attrs.iter().zip(attr_spans) {
            if is_short_attr_list {
                write!(self.out, " ")?;
            } else {
                self.out.new_line()?;

                if self.current_span_is_primary(start) {
                    self.out.indent_level += 1;
                    self.write_comments(start)?;
                    self.out.indent_level -= 1;
                }

                self.out.indented_tab()?;
            }

            match attr {
                AttrType::Attr(attr) => self.write_tag_attribute(attr, !is_short_attr_list)?,
                AttrType::Spread(spread) => {
                    write!(self.out, "{{")?;
                    self.write_spread_attribute(&spread.expr)?;
                    write!(self.out, "}}")?;
                }
            }

            if !is_short_attr_list {
                self.write_inline_comments(end, 0)?;
            }
        }

        // The `>` or `/>` goes on its own line if the attributes are split across lines
        if !is_short_attr_list {
            self.out.tabbed_line()?;
        }

        let has_open_tag_comment = self.brace_has_trailing_comments(delimiters);

        if children.is_empty() {
            // Self-closing tags
            if !self.body_has_comments(delimiters) {
                if is_short_attr_list {
                    write!(self.out, " ")?;
                }
                write!(self.out, "/>")?;
                return Ok(());
            }

            // A body that only has comments in it
            write!(self.out, ">")?;
            self.write_comment_only_body(delimiters)?;
            write!(self.out, "</{name}>")?;
            return Ok(());
        }

        write!(self.out, ">")?;

        // Inline a single short child: `<h1>"hello"</h1>`
        let children_len = self
            .is_short_children(children)
            .map_err(|_| std::fmt::Error)?;
        let is_small_children = is_short_attr_list
            && !has_open_tag_comment
            && !self.has_trailing_comments(children, delimiters)
            && children_len.is_some_and(|len| {
                len + attr_len + name.len() * 2 + self.out.indent_level * 4 < 100
            });

        if is_small_children {
            for child in children {
                self.write_ident(child)?;
            }
        } else {
            self.write_inline_comments(delimiters.open, 1)?;
            self.out.new_line()?;
            self.write_body_indented(children)?;
            self.write_closing_line(delimiters)?;
        }

        write!(self.out, "</{name}>")?;

        Ok(())
    }

    /// Write an attribute in the tag syntax: `name`, `name="literal"` or `name={expr}`
    fn write_tag_attribute(&mut self, attr: &Attribute, on_own_line: bool) -> Result {
        match &attr.name {
            // Dashed custom attributes don't need to be quoted in the tag syntax: `data-count="1"`
            AttributeName::Custom(name)
                if name
                    .value()
                    .split('-')
                    .all(|seg| syn::parse_str::<syn::Ident>(seg).is_ok()) =>
            {
                write!(self.out, "{}", name.value())?
            }
            name => self.write_attribute_name(name)?,
        }

        // Comments before the value stay there, with the value on a line of its own below them
        let comments = self.attr_value_comments(attr);
        let has_comments = !comments.is_empty();

        if attr.can_be_shorthand() && !has_comments {
            return Ok(());
        }

        write!(self.out, "=")?;

        if has_comments {
            let name_line = attr.name.span().end().line;
            self.out.indent_level += 1;
            for (line, comment) in comments {
                if line == name_line {
                    write!(self.out, " {comment}")?;
                } else {
                    self.out.new_line()?;
                    self.out.indented_tab()?;
                    write!(self.out, "{comment}")?;
                }
            }
            self.out.new_line()?;
            self.out.indented_tab()?;
        }

        match &attr.value {
            AttributeValue::AttrLiteral(value) => write!(self.out, "{value}")?,
            // The lines of a multiline if chain are indented relative to the attribute
            value @ AttributeValue::IfExpr(_) if on_own_line => {
                write!(self.out, "{{")?;
                self.out.indent_level += 1;
                self.write_attribute_value(value)?;
                self.out.indent_level -= 1;
                write!(self.out, "}}")?;
            }
            value => {
                write!(self.out, "{{")?;
                self.write_attribute_value(value)?;
                write!(self.out, "}}")?;
            }
        }

        if has_comments {
            self.out.indent_level -= 1;
        }

        Ok(())
    }

    fn write_text_node(&mut self, text: &TextNode) -> Result {
        self.out.write_text(&text.input)
    }

    fn write_expr_node(&mut self, expr: &ExprNode) -> Result {
        self.write_partial_expr(expr.expr.as_expr(), expr.span())
    }

    fn write_synthetic_boundary(&mut self, body: &TemplateBody) -> Result {
        let mut roots = body.roots.iter();
        if let Some(first) = roots.next() {
            self.write_ident(first)?;
            for node in roots {
                self.out.new_line()?;
                self.out.tab()?;
                self.write_ident(node)?;
            }
        }
        Ok(())
    }

    fn write_for_loop(&mut self, forloop: &ForLoop) -> std::fmt::Result {
        let (start, end) = (
            forloop.for_token.span().start(),
            forloop.brace.span.span().start(),
        );
        if self.has_comments_between(start, end) {
            self.write_header_source(start, end)?;
        } else {
            write!(self.out, "for {} in ", self.unparse_pat(&forloop.pat),)?;
            self.write_inline_expr(&forloop.expr)?;
        }
        self.write_block_body(&forloop.body.roots, (&forloop.brace).into())?;
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
                then_brace,
                then_branch,
                else_if_branch,
                else_brace,
                else_branch,
                ..
            } = chain;

            let (start, end) = (if_token.span().start(), then_brace.span.span().start());
            if self.has_comments_between(start, end) {
                self.write_header_source(start, end)?;
            } else {
                write!(self.out, "{} ", if_token.to_token_stream(),)?;
                self.write_inline_expr(cond)?;
            }
            self.write_block_body(&then_branch.roots, then_brace.into())?;

            if let Some(else_if_branch) = else_if_branch {
                write!(self.out, "}}")?;
                self.write_else(then_brace, else_if_branch.if_token.span())?;
                branch = Some(else_if_branch);
            } else if let Some(else_branch) = else_branch {
                let else_brace = else_brace.unwrap_or_default();
                write!(self.out, "}}")?;
                self.write_else(then_brace, else_brace.span.span())?;
                write!(self.out, "{{")?;
                self.write_block_body(&else_branch.roots, (&else_brace).into())?;
                branch = None;
            } else {
                branch = None;
            }
        }

        write!(self.out, "}}")?;

        Ok(())
    }

    /// Writes the `else` that follows the closing brace of a branch, up to what comes `next`
    ///
    /// Comments can't go on the same line as the `else`, so if there are any between the brace
    /// and `next` then they are left where they are, with the `else` on a line of its own.
    fn write_else(&mut self, then_brace: &Brace, next: Span) -> Result {
        let close = then_brace.span.span().end();

        let inline_comment = self.inline_comment(close, 0);
        let comments: Vec<&str> = (close.line..next.start().line.saturating_sub(1))
            .filter_map(|idx| self.src.get(idx).map(|line| line.trim()))
            .filter(|line| line.starts_with("//"))
            .collect();

        if inline_comment.is_none() && comments.is_empty() {
            return write!(self.out, " else ");
        }

        self.write_inline_comments(close, 0)?;
        for comment in comments {
            self.out.tabbed_line()?;
            write!(self.out, "{comment}")?;
        }
        self.out.tabbed_line()?;
        write!(self.out, "else ")
    }

    /// Writes the body of a `for` or `if` block, from after its opening brace up to its closing
    /// brace
    fn write_block_body(&mut self, children: &[BodyNode], delimiters: BodyDelimiters) -> Result {
        if children.is_empty() {
            return self.write_comment_only_body(delimiters);
        }

        self.write_inline_comments(delimiters.open, 1)?;
        self.out.new_line()?;
        self.write_body_indented(children)?;
        self.write_closing_line(delimiters)
    }

    /// Writes the header of a `for` or `if` as it is in the source, up to and including its
    /// opening brace. Comments within a header have no place in its formatted form, so a header
    /// that has them is only re-indented.
    fn write_header_source(&mut self, start: LineColumn, end: LineColumn) -> Result {
        let source = self.source_between(start, end);
        let source = source.trim();
        let ends_with_comment = line_comments(source)
            .last()
            .is_some_and(|comment| comment.end == source.len());

        let mut lines = source.lines();
        write!(self.out, "{}", lines.next().unwrap_or_default().trim_end())?;

        // Lines after the first keep their indentation relative to each other
        let indent_of = |line: &str| line.len() - line.trim_start().len();
        let shared_indent = lines
            .clone()
            .filter(|line| !line.trim().is_empty())
            .map(indent_of)
            .min()
            .unwrap_or_default();
        for line in lines {
            self.out.new_line()?;
            if !line.trim().is_empty() {
                self.out.indented_tab()?;
                let line = line.get(shared_indent..).unwrap_or(line.trim_start());
                write!(self.out, "{}", line.trim_end())?;
            }
        }

        if ends_with_comment {
            self.out.tabbed_line()?;
            write!(self.out, "{{")
        } else {
            write!(self.out, " {{")
        }
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
        let mut is_first = true;

        while let Some(child) = iter.next() {
            let start = child.first_token_span().start();
            if self.current_span_is_primary(start) {
                let comments = self.accumulate_full_line_comments(start);
                let has_real_comment = comments
                    .iter()
                    .any(|&id| self.src.get(id).is_some_and(|l| l.trim().starts_with("//")));
                if has_real_comment || !is_first {
                    self.apply_line_comments(comments)?;
                }
            };
            is_first = false;
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
        delimiters: BodyDelimiters,
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
        let has_postbrace_comments = self.brace_has_trailing_comments(delimiters);
        let is_short_attr_list =
            ((attr_len + self.out.indent_level * 4) < 80) && !has_postbrace_comments;
        let children_len = self
            .is_short_children(children)
            .map_err(|_| std::fmt::Error)?;
        let has_attributes = !attributes.is_empty() || !spreads.is_empty();
        let attr_comments = self.attr_comments(attributes, spreads, delimiters);
        let has_trailing_comments = self.has_trailing_comments(children, delimiters);
        // A comment after the last attribute ends its line, so the children can't follow on it
        let is_small_children = children_len.is_some()
            && !has_trailing_comments
            && attr_comments != AttrComments::AfterLast;
        // Attributes with comments each need their own line. A comment after the last one is the
        // exception, as only the children come after it.
        let split_commented_attrs = match attr_comments {
            AttrComments::None => {
                has_attributes && children.is_empty() && self.has_closing_comments(delimiters)
            }
            AttrComments::AfterLast => children.is_empty(),
            AttrComments::Any => true,
        };

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
            && !has_postbrace_comments
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
            self.write_comment_only_body(delimiters)?;
        }

        // multiline handlers bump everything down, but not empty blocks
        if !matches!(opt_level, ShortOptimization::Empty)
            && (attr_len > 1000 || split_commented_attrs || self.out.indent.split_line_attributes())
        {
            opt_level = ShortOptimization::NoOpt;
        }

        let has_children = !children.is_empty();

        match opt_level {
            ShortOptimization::Empty => {}
            ShortOptimization::Oneliner => {
                write!(self.out, " ")?;

                self.write_attributes(attributes, spreads, true, delimiters, has_children)?;

                if !children.is_empty() && has_attributes {
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
                if has_attributes {
                    write!(self.out, " ")?;
                }

                self.write_attributes(attributes, spreads, true, delimiters, has_children)?;

                if !children.is_empty() {
                    self.out.new_line()?;
                    self.write_body_indented(children)?;
                }

                self.write_closing_line(delimiters)?;
            }

            ShortOptimization::NoOpt => {
                self.write_opening_comments(delimiters)?;
                self.out.new_line()?;
                self.write_attributes(attributes, spreads, false, delimiters, has_children)?;

                if !children.is_empty() {
                    if !attributes.is_empty() || !spreads.is_empty() {
                        self.out.new_line()?;
                    }
                    self.write_body_indented(children)?;
                }

                self.write_closing_line(delimiters)?;
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
        delimiters: BodyDelimiters,
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
                    delimiters,
                    match attr {
                        AttrType::Attr(attr) => attr.span(),
                        AttrType::Spread(attr) => attr.span(),
                    },
                )?;
            }

            self.out.indent_level -= 1;

            if !props_same_line {
                self.out.indented_tab()?;
            }

            match attr {
                AttrType::Attr(attr) => self.write_attribute(attr, !props_same_line)?,
                AttrType::Spread(attr) => self.write_spread_attribute(&attr.expr)?,
            }

            let attr_end = match attr {
                AttrType::Attr(attr) => self.end_of_attr(attr, delimiters),
                AttrType::Spread(attr) => self.end_of_spread(attr, delimiters),
            };

            let has_more = attr_iter.peek().is_some();
            let should_finish_comma = has_attributes && has_children || !props_same_line;

            if has_more || should_finish_comma {
                write!(self.out, ",")?;
            }

            if !props_same_line {
                self.write_inline_comments(attr_end, 0)?;
            }

            if props_same_line && !has_more {
                self.write_inline_comments(attr_end, 0)?;
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

    /// Writes an attribute, which is either on a line of its own or shares one with the opening
    /// brace of its element
    fn write_attribute(&mut self, attr: &Attribute, own_line: bool) -> Result {
        self.write_attribute_name(&attr.name)?;

        let comments = self.attr_value_comments(attr);
        if !comments.is_empty() {
            return self.write_commented_attribute_value(attr, comments);
        }

        if !attr.can_be_shorthand() {
            if let AttributeValue::IfExpr(if_chain) = &attr.value {
                let inline_len = self.attr_value_len(&attr.value);
                let line_budget = 80usize.saturating_sub(self.out.indent_level * 4);
                if inline_len > line_budget {
                    self.out.indent_level += 1;
                    if own_line {
                        write!(self.out, ": ")?;
                    } else {
                        write!(self.out, ":")?;
                        self.out.new_line()?;
                        self.out.tab()?;
                    }
                    self.write_attribute_if_chain_multiline(if_chain)?;
                    self.out.indent_level -= 1;
                    return Ok(());
                }
            }
            write!(self.out, ": ")?;
            self.write_attribute_value(&attr.value)?;
        }

        Ok(())
    }

    /// Writes the value of an attribute that has comments between its name and its value. The
    /// value goes on a line of its own, below the comments.
    fn write_commented_attribute_value(
        &mut self,
        attr: &Attribute,
        comments: Vec<(usize, &str)>,
    ) -> Result {
        write!(self.out, ":")?;

        let name_line = attr.name.span().end().line;
        self.out.indent_level += 1;
        for (line, comment) in comments {
            if line == name_line {
                write!(self.out, " {comment}")?;
            } else {
                self.out.new_line()?;
                self.out.indented_tab()?;
                write!(self.out, "{comment}")?;
            }
        }
        self.out.new_line()?;
        self.out.indented_tab()?;
        if let AttributeValue::IfExpr(if_chain) = &attr.value {
            self.out.indent_level += 1;
            self.write_attribute_if_chain(if_chain)?;
            self.out.indent_level -= 1;
        } else {
            self.write_attribute_value(&attr.value)?;
        }
        self.out.indent_level -= 1;

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
        let inline_len = self.attr_value_len(&AttributeValue::IfExpr(if_chain.clone()));
        let line_budget = 80usize.saturating_sub(self.out.indent_level * 4);

        if inline_len <= line_budget {
            self.write_attribute_if_chain_inline(if_chain)
        } else {
            self.write_attribute_if_chain_multiline(if_chain)
        }
    }

    fn write_attribute_if_chain_inline(&mut self, if_chain: &IfAttributeValue) -> Result {
        let cond = self.unparse_expr(&if_chain.if_expr.cond);
        write!(self.out, "if {cond} {{ ")?;
        self.write_attribute_value(&if_chain.then_value)?;
        write!(self.out, " }}")?;
        match if_chain.else_value.as_deref() {
            Some(AttributeValue::IfExpr(else_if_chain)) => {
                write!(self.out, " else ")?;
                self.write_attribute_if_chain_inline(else_if_chain)?;
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

    fn write_attribute_if_chain_multiline(&mut self, if_chain: &IfAttributeValue) -> Result {
        let if_expr = &if_chain.if_expr;
        let then_brace = &if_expr.then_branch.brace_token;

        let (start, end) = (
            if_expr.if_token.span().start(),
            then_brace.span.span().start(),
        );
        if self.has_comments_between(start, end) {
            self.write_header_source(start, end)?;
        } else {
            let cond = self.unparse_expr(&if_expr.cond);
            write!(self.out, "if {cond} {{")?;
        }
        self.write_attribute_if_branch(&if_chain.then_value, then_brace.into())?;

        match if_chain.else_value.as_deref() {
            Some(AttributeValue::IfExpr(else_if_chain)) => {
                self.write_else(then_brace, else_if_chain.if_expr.if_token.span())?;
                self.write_attribute_if_chain_multiline(else_if_chain)?;
            }
            Some(other) => {
                let else_brace = match if_expr.else_branch.as_ref().map(|(_, expr)| &**expr) {
                    Some(Expr::Block(block)) => block.block.brace_token,
                    _ => Brace::default(),
                };
                self.write_else(then_brace, else_brace.span.span())?;
                write!(self.out, "{{")?;
                self.write_attribute_if_branch(other, (&else_brace).into())?;
            }
            None => {}
        }

        Ok(())
    }

    /// Writes the value of one branch of an `if` attribute value and the comments around it, from
    /// after the opening brace of the branch through to its closing brace
    fn write_attribute_if_branch(
        &mut self,
        value: &AttributeValue,
        delimiters: BodyDelimiters,
    ) -> Result {
        self.write_inline_comments(delimiters.open, 1)?;
        self.out.new_line()?;

        self.out.indent_level += 1;
        let start = value.span().start();
        if self.has_leading_comments(start) {
            let mut comments = self.accumulate_full_line_comments(start);
            let is_blank = |id: &usize| self.src.get(*id).is_none_or(|l| l.trim().is_empty());
            while comments.front().is_some_and(is_blank) {
                comments.pop_front();
            }
            self.apply_line_comments(comments)?;
        }
        self.out.tab()?;
        if let AttributeValue::IfExpr(if_chain) = value {
            self.write_attribute_if_chain(if_chain)?;
        } else {
            // Expressions are written as if they were one level in from the indent level
            self.out.indent_level -= 1;
            self.write_attribute_value(value)?;
            self.out.indent_level += 1;
        }
        self.write_inline_comments(value.span().end(), 0)?;
        self.out.indent_level -= 1;

        self.write_closing_line(delimiters)?;
        write!(self.out, "}}")
    }

    fn write_attr_comments(&mut self, delimiters: BodyDelimiters, attr_span: Span) -> Result {
        // There's a chance this line actually shares the same line as the previous
        // Only write comments if the comments actually belong to this line
        //
        // to do this, we check if the attr span starts on the same line as the brace
        // if it doesn't, we write the comments
        let brace_line = delimiters.open.line;
        let attr_line = attr_span.start().line;

        if brace_line != attr_line {
            // Only write comments if the line is empty before the attribute start
            let row_start = self.text_before(attr_span.start()).unwrap_or("");
            if !row_start.trim().is_empty() {
                return Ok(());
            }

            self.write_comments(attr_span.start())?;
        }

        Ok(())
    }

    /// The start of a spread in the tag syntax, including the brace it is wrapped in: `{..spread}`
    fn start_of_tag_spread(&self, spread: &Spread) -> LineColumn {
        let mut start = spread.span().start();
        if let Some(before) = self.text_before(start)
            && before.trim_end().ends_with('{')
        {
            start.column = before.trim_end().chars().count() - 1;
        }
        start
    }

    fn write_inline_comments(&mut self, final_span: LineColumn, offset: usize) -> Result {
        if let Some(comment) = self.inline_comment(final_span, offset) {
            write!(self.out, " {comment}")?;
        }

        Ok(())
    }

    /// The comment that follows a location on the same line, skipping `offset` bytes first
    pub(crate) fn inline_comment(&self, location: LineColumn, offset: usize) -> Option<&'a str> {
        // don't emit whitespace if the span is messed up for some reason
        if location.line == 1 && location.column == 0 {
            return None;
        };

        let rest = self.text_after(location)?.trim();
        let rest = rest.get(offset..)?.trim();

        rest.starts_with("//").then_some(rest)
    }

    fn accumulate_full_line_comments(&mut self, loc: LineColumn) -> VecDeque<usize> {
        // collect all comments upwards
        // make sure we don't collect the comments of the node that we're currently under.
        let start = loc;
        let line_start = start.line - 1;

        let mut comments = VecDeque::new();

        // don't emit whitespace if the span is messed up for some reason
        if loc.line == 1 && loc.column == 0 {
            return comments;
        };

        let Some(lines) = self.src.get(..line_start) else {
            return comments;
        };

        // We go backwards to collect comments and empty lines. We only want to keep one empty line,
        // the rest should be `//` comments
        let mut last_line_was_empty = false;
        for (id, line) in lines.iter().enumerate().rev() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                comments.push_front(id);
                last_line_was_empty = false;
            } else if trimmed.is_empty() {
                if !last_line_was_empty {
                    comments.push_front(id);
                    last_line_was_empty = true;
                }

                continue;
            } else {
                break;
            }
        }

        // If there is more than 1 comment, make sure the first comment is not an empty line
        if comments.len() > 1
            && let Some(&first) = comments.back()
            && self.src[first].trim().is_empty()
        {
            comments.pop_back();
        }

        comments
    }

    fn apply_line_comments(&mut self, mut comments: VecDeque<usize>) -> Result {
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
        let comments = self.accumulate_full_line_comments(loc);
        self.apply_line_comments(comments)?;
        Ok(())
    }

    fn span_has_line_comments(&self, span: Span) -> bool {
        span.source_text().is_some_and(|source| {
            source
                .lines()
                .any(|line| line.trim_start().starts_with("//"))
        })
    }

    fn attr_value_len(&mut self, value: &AttributeValue) -> usize {
        match value {
            AttributeValue::IfExpr(if_chain) => {
                let span = if_chain.if_expr.span();
                if self.has_comments_between(span.start(), span.end()) {
                    return 100000;
                }

                let condition_len = self.retrieve_formatted_expr(&if_chain.if_expr.cond).len();
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
                .map(|expr| {
                    if self.span_has_line_comments(expr.span()) {
                        100000
                    } else {
                        self.attr_expr_len(&expr)
                    }
                })
                .unwrap_or(100000),
            AttributeValue::EventTokens(closure) => closure
                .as_expr()
                .map(|expr| {
                    if self.span_has_line_comments(expr.span()) {
                        100000
                    } else {
                        self.attr_expr_len(&expr)
                    }
                })
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
            if self.has_leading_comments(attr.span().start()) {
                return 100000;
            }

            total += match &attr.name {
                AttributeName::BuiltIn(name) => {
                    let name = name.to_string();
                    name.len()
                }
                AttributeName::Custom(name) => name.value().len() + 2,
                AttributeName::Spread(_) => unreachable!(),
            };

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

    /// Writes a body that has nothing in it but comments, leaving the closing delimiter to the
    /// caller. A body without comments is left empty so that it closes on the same line.
    fn write_comment_only_body(&mut self, delimiters: BodyDelimiters) -> Result {
        let BodyDelimiters { open, close, .. } = delimiters;

        let has_inline_comment = self.brace_has_trailing_comments(delimiters);
        self.write_opening_comments(delimiters)?;

        // Keep the comments, and one of the blank lines between each of them
        let mut lines: Vec<&str> = Vec::new();
        for idx in open.line..close.line.saturating_sub(1) {
            let line = self.src.get(idx).map_or("", |line| line.trim());
            let follows_comment = lines.last().is_some_and(|last| !last.is_empty());
            if line.starts_with("//") || (line.is_empty() && follows_comment) {
                lines.push(line);
            }
        }
        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }

        if lines.is_empty() && !has_inline_comment {
            return Ok(());
        }

        self.out.new_line()?;
        for line in lines {
            if !line.is_empty() {
                self.out.indented_tab()?;
                write!(self.out, "{line}")?;
            }
            self.out.new_line()?;
        }
        self.out.tab()
    }

    /// Whether a body with no attributes or children has comments in it
    fn body_has_comments(&self, delimiters: BodyDelimiters) -> bool {
        let BodyDelimiters { open, close, .. } = delimiters;

        self.brace_has_trailing_comments(delimiters)
            || (open.line..close.line.saturating_sub(1)).any(|idx| {
                self.src
                    .get(idx)
                    .is_some_and(|line| line.trim().starts_with("//"))
            })
    }

    /// Starts the line of a closing delimiter, first writing the comments on the lines above it
    fn write_closing_line(&mut self, delimiters: BodyDelimiters) -> Result {
        self.out.new_line()?;

        if self.leading_row_is_empty(delimiters.close) {
            let comments = self.accumulate_full_line_comments(delimiters.close);
            if self.has_real_comment(&comments) {
                self.out.indent_level += 1;
                self.apply_line_comments(comments)?;
                self.out.indent_level -= 1;
            }
        }

        self.out.tab()
    }

    /// Whether there are comments on the lines above a closing delimiter
    fn has_closing_comments(&mut self, delimiters: BodyDelimiters) -> bool {
        self.leading_row_is_empty(delimiters.close) && {
            let comments = self.accumulate_full_line_comments(delimiters.close);
            self.has_real_comment(&comments)
        }
    }

    /// Whether any of the collected lines is a comment rather than a blank line
    fn has_real_comment(&self, lines: &VecDeque<usize>) -> bool {
        lines
            .iter()
            .any(|&id| self.src.get(id).is_some_and(|l| l.trim().starts_with("//")))
    }

    /// Where the attributes and spreads have comments: on the lines above them, or after them on
    /// the same line
    fn attr_comments(
        &self,
        attributes: &[Attribute],
        spreads: &[Spread],
        delimiters: BodyDelimiters,
    ) -> AttrComments {
        if attributes
            .iter()
            .any(|attr| !self.attr_value_comments(attr).is_empty())
        {
            return AttrComments::Any;
        }

        let attributes = attributes
            .iter()
            .map(|attr| (attr.span().start(), self.end_of_attr(attr, delimiters)));
        let spreads = spreads.iter().map(|spread| {
            (
                spread.span().start(),
                self.end_of_spread(spread, delimiters),
            )
        });

        let mut comments = AttrComments::None;
        let mut iter = attributes.chain(spreads).peekable();
        while let Some((start, end)) = iter.next() {
            let is_last = iter.peek().is_none();
            if self.has_leading_comments(start) {
                return AttrComments::Any;
            }
            if self.inline_comment(end, 0).is_some() {
                if !is_last {
                    return AttrComments::Any;
                }
                comments = AttrComments::AfterLast;
            }
        }

        comments
    }

    /// Whether the lines directly above a location are comments. Only true if nothing else comes
    /// before the location on its own line, as the comments would then belong to that instead.
    fn has_leading_comments(&self, location: LineColumn) -> bool {
        if !self.current_span_is_primary(location) {
            return false;
        }

        let Some(lines) = self.src.get(..location.line - 1) else {
            return false;
        };

        // Blank lines can separate the comments from the location
        lines
            .iter()
            .rev()
            .map(|line| line.trim())
            .find(|line| !line.is_empty())
            .is_some_and(|line| line.starts_with("//"))
    }

    /// The end of an attribute, including its trailing comma
    fn end_of_attr(&self, attr: &Attribute, delimiters: BodyDelimiters) -> LineColumn {
        match &attr.comma {
            Some(comma) => comma.span().end(),
            None => self.end_of_tag_value(delimiters, self.total_span_of_attr(attr).end()),
        }
    }

    /// The end of a spread, including its trailing comma
    fn end_of_spread(&self, spread: &Spread, delimiters: BodyDelimiters) -> LineColumn {
        let mut end = spread.expr.span().end();

        // The comma of a spread isn't exposed, so look for it in the source
        if let Some(rest) = self.text_after(end)
            && let Some((whitespace, _)) = rest.split_once(',')
            && whitespace.trim().is_empty()
        {
            end.column += whitespace.chars().count() + 1;
        }

        self.end_of_tag_value(delimiters, end)
    }

    /// The values of attributes in the tag syntax can be wrapped in a brace that is not part of
    /// the value itself: `class={value}`. This moves the end of a value past that brace.
    fn end_of_tag_value(&self, delimiters: BodyDelimiters, mut end: LineColumn) -> LineColumn {
        if delimiters.is_tag
            && let Some(rest) = self.text_after(end)
            && let Some((whitespace, _)) = rest.split_once('}')
            && whitespace.trim().is_empty()
        {
            end.column += whitespace.chars().count() + 1;
        }
        end
    }

    fn write_partial_expr(&mut self, expr: syn::Result<Expr>, src_span: Span) -> Result {
        let Ok(expr) = expr else {
            self.invalid_exprs.push(src_span);
            return Err(std::fmt::Error);
        };

        thread_local! {
            static COMMENT_REGEX: Regex = Regex::new("\"[^\"]*\"|(//.*)").unwrap();
        }

        let pretty = self.retrieve_formatted_expr(&expr).to_string();
        let source = src_span.source_text().unwrap_or_default();
        let source_has_line_comments = source
            .lines()
            .any(|line| line.trim_start().starts_with("//"));
        let mut src_lines = source.lines().peekable();

        // Comments already in pretty output (from nested rsx!) - skip these from source
        let pretty_comments: HashSet<_> = pretty
            .lines()
            .filter(|l| l.trim().starts_with("//"))
            .map(|l| l.trim())
            .collect();

        let mut out = String::new();

        if src_lines.peek().is_none() {
            out = pretty;
        } else {
            for line in pretty.lines() {
                let trimmed = line.trim();
                let compacted = line.replace(" ", "").replace(",", "");

                // Pretty comments: consume matching source lines, preserve preceding empty lines
                if trimmed.starts_with("//") {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    let mut had_empty = false;
                    while let Some(s) = src_lines.peek() {
                        let t = s.trim();
                        if t.is_empty() {
                            had_empty = true;
                            src_lines.next();
                        } else if t == trimmed {
                            src_lines.next();
                            break;
                        } else {
                            break;
                        }
                    }
                    if had_empty {
                        out.push('\n');
                    }
                    out.push_str(line);
                    continue;
                }

                // Pretty empty lines: preserve and sync with source
                if trimmed.is_empty() {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    while src_lines
                        .peek()
                        .map(|s| s.trim().is_empty())
                        .unwrap_or(false)
                    {
                        src_lines.next();
                    }
                    continue;
                }

                if !out.is_empty() {
                    out.push('\n');
                }

                // Scan source for comments/empty lines before the matching line
                let mut pending_comments = Vec::new();
                let mut had_empty = false;
                let mut multiline: Option<Vec<&str>> = None;

                while let Some(src) = src_lines.peek() {
                    let src_trimmed = src.trim();

                    if src_trimmed.is_empty() || src_trimmed.starts_with("//") {
                        if src_trimmed.is_empty() {
                            if pending_comments.is_empty() {
                                had_empty = true;
                            }
                        } else if !pretty_comments.contains(src_trimmed) {
                            pending_comments.push(src_trimmed);
                        }
                        src_lines.next();
                        continue;
                    }

                    let src_compacted = src.replace(" ", "").replace(",", "");

                    // Exact match
                    if src_compacted.contains(&compacted) {
                        break;
                    }

                    // Multi-line method chain (e.g., foo\n  .bar()\n  .baz())
                    if !src_compacted.is_empty() && compacted.starts_with(&src_compacted) {
                        let is_call = src_trimmed.ends_with('(')
                            || src_trimmed.ends_with(',')
                            || src_trimmed.ends_with('{');
                        let is_commented_block =
                            source_has_line_comments && src_trimmed.ends_with('{');
                        if is_commented_block || !is_call {
                            multiline = Some(vec![*src]);
                            break;
                        }
                    }

                    // Non-matching line - clear pending and skip
                    pending_comments.clear();
                    had_empty = false;
                    src_lines.next();
                    break;
                }

                // Output empty line if needed
                if had_empty {
                    out.push('\n');
                }

                // Output pending comments
                for comment in &pending_comments {
                    for c in line.chars().take_while(|c| c.is_whitespace()) {
                        out.push(c);
                    }
                    if matches!(trimmed.chars().next(), Some(')' | '}' | ']')) {
                        out.push_str(self.out.indent.indent_str());
                    }
                    out.push_str(comment);
                    out.push('\n');
                }

                // Handle multi-line method chains
                if let Some(mut ml) = multiline {
                    src_lines.next();
                    let mut acc = ml[0].replace(" ", "").replace(",", "");

                    while let Some(src) = src_lines.peek() {
                        let t = src.trim();
                        if t.starts_with("//") {
                            ml.push(src);
                            src_lines.next();
                            continue;
                        }
                        if t.is_empty() {
                            src_lines.next();
                            continue;
                        }

                        acc.push_str(&src.replace(" ", "").replace(",", ""));
                        ml.push(src);

                        if acc.contains(&compacted) {
                            src_lines.next();
                            break;
                        }

                        let cont = t.starts_with('.')
                            || t.starts_with("&&")
                            || t.starts_with("||")
                            || matches!(t.chars().next(), Some('+' | '-' | '*' | '/' | '?'));

                        if cont || compacted.starts_with(&acc) {
                            src_lines.next();
                            continue;
                        }
                        break;
                    }

                    // Write multi-line with adjusted indentation
                    let base_indent = if source_has_line_comments && ml[0].trim_end().ends_with('{')
                    {
                        ml.iter()
                            .skip(1)
                            .filter(|line| !line.trim().is_empty())
                            .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
                            .min()
                            .unwrap_or(0)
                    } else {
                        ml[0].chars().take_while(|c| c.is_whitespace()).count()
                    };
                    let target: String = line.chars().take_while(|c| c.is_whitespace()).collect();

                    for (i, src_line) in ml.iter().enumerate() {
                        let indent = src_line.chars().take_while(|c| c.is_whitespace()).count();
                        out.push_str(&target);
                        for _ in 0..indent.saturating_sub(base_indent) {
                            out.push(' ');
                        }
                        out.push_str(src_line.trim());
                        if i < ml.len() - 1 {
                            out.push('\n');
                        }
                    }
                } else {
                    // Single line - output pretty line and capture inline comments
                    out.push_str(line);
                    if let Some(src_line) = src_lines.next()
                        && let Some(cap) = COMMENT_REGEX.with(|r| r.captures(src_line))
                        && let Some(c) = cap.get(1)
                    {
                        out.push_str(" // ");
                        out.push_str(c.as_str().replace("//", "").trim());
                    }
                }
            }
        }

        self.write_mulitiline_tokens(out)?;
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
            [BodyNode::Text(text)] => Some(text.input.to_string_with_quotes().len()),

            // TODO: let rawexprs to be inlined
            [BodyNode::RawExpr(expr)] => {
                let pretty = self.retrieve_formatted_expr(&expr.expr.as_expr()?);
                if pretty.contains('\n') {
                    None
                } else {
                    Some(pretty.len() + 2)
                }
            }

            // TODO: let rawexprs to be inlined
            [BodyNode::Component(comp)]
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
        children
            .iter()
            .any(|child| self.has_leading_comments(child.first_token_span().start()))
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
        let Some(column) = location.column.checked_sub(1) else {
            return false;
        };

        self.text_before(LineColumn { column, ..location })
            .is_some_and(|before| before.trim().is_empty())
    }

    /// The text of a line that comes before a location on it
    fn text_before(&self, location: LineColumn) -> Option<&'a str> {
        let line = self.src.get(location.line.checked_sub(1)?)?;
        line.get(..Self::byte_offset(line, location.column)?)
    }

    /// The text of a line that comes after a location on it
    fn text_after(&self, location: LineColumn) -> Option<&'a str> {
        let line = self.src.get(location.line.checked_sub(1)?)?;
        line.get(Self::byte_offset(line, location.column)?..)
    }

    /// Columns count characters, which are not all one byte long
    fn byte_offset(line: &str, column: usize) -> Option<usize> {
        line.char_indices()
            .map(|(idx, _)| idx)
            .chain([line.len()])
            .nth(column)
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
            BodyNode::Element(el) => el.delimiter.close_span().unwrap_or_else(|| el.name.span()),
            BodyNode::Component(el) => el.delimiter.close_span().unwrap_or_else(|| el.name.span()),
            BodyNode::Text(txt) => txt.input.span(),
            BodyNode::RawExpr(exp) => exp.span(),
            BodyNode::ForLoop(f) => f.brace.span.span(),
            BodyNode::IfChain(chain) => {
                // The closing brace is that of the last branch
                let mut last = chain;
                while let Some(next) = &last.else_if_branch {
                    last = next;
                }
                match last.else_brace {
                    Some(b) => b.span.span(),
                    None => last.then_brace.span.span(),
                }
            }
            BodyNode::SyntheticBoundary(_) => node.span(),
        }
    }

    fn total_span_of_attr(&self, attr: &Attribute) -> Span {
        match &attr.value {
            AttributeValue::Shorthand(s) => s.span(),
            AttributeValue::AttrLiteral(l) => l.span(),
            AttributeValue::EventTokens(closure) => closure.span(),
            AttributeValue::AttrExpr(exp) => exp.span(),
            AttributeValue::IfExpr(ex) => ex.span(),
        }
    }

    fn brace_has_trailing_comments(&self, delimiters: BodyDelimiters) -> bool {
        !self.opening_comments(delimiters).is_empty()
    }

    /// The comments that go after the opening delimiter of a body: the one that is already there,
    /// and any between the name and the delimiter, which can't stay where they are
    fn opening_comments(&self, delimiters: BodyDelimiters) -> Vec<&'a str> {
        let mut comments: Vec<&str> = match delimiters.name_end {
            // The attributes of a tag come between its name and the delimiter
            Some(name_end) if !delimiters.is_tag => self
                .comments_between(name_end, delimiters.open)
                .into_iter()
                .map(|(_, comment)| comment)
                .collect(),
            _ => Vec::new(),
        };
        comments.extend(self.inline_comment(delimiters.open, 1));
        comments
    }

    /// Writes the comments that go after an opening delimiter. Only the first fits on its line,
    /// so the rest go on the first lines of the body.
    fn write_opening_comments(&mut self, delimiters: BodyDelimiters) -> Result {
        let mut comments = self.opening_comments(delimiters).into_iter();
        if let Some(first) = comments.next() {
            write!(self.out, " {first}")?;
        }
        for comment in comments {
            self.out.new_line()?;
            self.out.indented_tab()?;
            write!(self.out, "{comment}")?;
        }
        Ok(())
    }

    /// The comments between the name of an attribute and its value, with the lines they are on
    fn attr_value_comments(&self, attr: &Attribute) -> Vec<(usize, &'a str)> {
        self.comments_between(attr.name.span().end(), attr.value.span().start())
    }

    /// The comments in a gap between two tokens, with the lines they are on. There must not be
    /// anything in the gap that could contain a `//` without it being a comment, like a string.
    fn comments_between(&self, start: LineColumn, end: LineColumn) -> Vec<(usize, &'a str)> {
        self.lines_between(start, end)
            .filter_map(|(line, text)| {
                let comment = text.get(text.find("//")?..)?.trim_end();
                Some((line, comment))
            })
            .collect()
    }

    /// Whether there are comments between two locations, which can have any code between them
    fn has_comments_between(&self, start: LineColumn, end: LineColumn) -> bool {
        !line_comments(&self.source_between(start, end)).is_empty()
    }

    /// The source between two locations
    fn source_between(&self, start: LineColumn, end: LineColumn) -> String {
        let lines: Vec<&str> = self
            .lines_between(start, end)
            .map(|(_, text)| text)
            .collect();
        lines.join("\n")
    }

    /// The part of each source line that is between two locations, with its line number
    fn lines_between(
        &self,
        start: LineColumn,
        end: LineColumn,
    ) -> impl Iterator<Item = (usize, &'a str)> + '_ {
        let in_order = (start.line, start.column) < (end.line, end.column);
        let lines = (start.line..=end.line).filter(move |_| in_order);

        lines.filter_map(move |number| {
            let line = *self.src.get(number.checked_sub(1)?)?;
            let from = match number == start.line {
                true => Self::byte_offset(line, start.column)?,
                false => 0,
            };
            let to = match number == end.line {
                true => Self::byte_offset(line, end.column)?,
                false => line.len(),
            };
            Some((number, line.get(from..to)?))
        })
    }

    fn has_trailing_comments(&self, children: &[BodyNode], delimiters: BodyDelimiters) -> bool {
        let Some(last_node) = children.last() else {
            return false;
        };

        // Check for any comments after the last node between the last brace
        let mut location = Self::final_span_of_node(last_node).end();
        loop {
            match self.text_after(location) {
                Some(rest) if rest.trim().starts_with("//") => return true,
                Some(_) => {}
                None => return false,
            }

            // If we reached the end of the brace span, stop
            if location.line == delimiters.close.line {
                break;
            }

            location = LineColumn {
                line: location.line + 1,
                column: 0,
            };
        }

        false
    }
}

/// Finds the `//` comments in a piece of source, skipping over anything that only looks like one
/// because it is inside a string, a character or a block comment
fn line_comments(source: &str) -> Vec<std::ops::Range<usize>> {
    let bytes = source.as_bytes();
    let find = |from: usize, pattern: &str| source.get(from..)?.find(pattern).map(|at| from + at);

    let mut comments = Vec::new();
    let mut idx = 0;
    while let Some(&byte) = bytes.get(idx) {
        let next = bytes.get(idx + 1).copied();
        idx = match (byte, next) {
            (b'/', Some(b'/')) => {
                let end = find(idx, "\n").unwrap_or(source.len());
                comments.push(idx..end);
                end
            }
            (b'/', Some(b'*')) => {
                let mut depth = 1;
                let mut end = idx + 2;
                while depth > 0 && end < bytes.len() {
                    match bytes.get(end..end + 2) {
                        Some(b"/*") => (depth, end) = (depth + 1, end + 2),
                        Some(b"*/") => (depth, end) = (depth - 1, end + 2),
                        _ => end += 1,
                    }
                }
                end
            }
            (b'"', _) => {
                let hashes = bytes[..idx]
                    .iter()
                    .rev()
                    .take_while(|b| **b == b'#')
                    .count();
                let is_raw = idx.checked_sub(hashes + 1).map(|at| bytes[at]) == Some(b'r');
                if is_raw {
                    let close = format!("\"{}", "#".repeat(hashes));
                    find(idx + 1, &close).map_or(source.len(), |at| at + close.len())
                } else {
                    let mut end = idx + 1;
                    loop {
                        match bytes.get(end) {
                            Some(b'\\') => end += 2,
                            Some(b'"') | None => break end + 1,
                            Some(_) => end += 1,
                        }
                    }
                }
            }
            // An escaped character
            (b'\'', Some(b'\\')) => find(idx + 3, "'").map_or(source.len(), |at| at + 1),
            (b'\'', _) => {
                // Either a character, or a lifetime which has no closing quote
                let mut chars = source[idx + 1..].chars();
                match (chars.next(), chars.next()) {
                    (Some(ch), Some('\'')) => idx + ch.len_utf8() + 2,
                    _ => idx + 1,
                }
            }
            _ => idx + 1,
        };
    }

    comments
}

#[cfg(test)]
mod tests {
    use super::line_comments;

    fn comments(source: &str) -> Vec<&str> {
        line_comments(source)
            .into_iter()
            .map(|range| &source[range])
            .collect()
    }

    #[test]
    fn finds_line_comments() {
        assert_eq!(comments("a // one\nb // two"), ["// one", "// two"]);
        assert_eq!(comments("a /* b // c */ d"), Vec::<&str>::new());
        assert_eq!(comments("a /* /* b */ // c */ d // e"), ["// e"]);
    }

    #[test]
    fn skips_text_that_looks_like_a_comment() {
        assert_eq!(comments(r#"url("http://a") // one"#), ["// one"]);
        assert_eq!(comments(r#"x("\"//") // one"#), ["// one"]);
        assert_eq!(comments(r##"x(r#"a"//"#) // one"##), ["// one"]);
        assert_eq!(comments(r#"x('"', '\'', "//") // one"#), ["// one"]);
        assert_eq!(comments(r#"x::<'a>("//") // é"#), ["// é"]);
    }
}
