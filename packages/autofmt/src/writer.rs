//! The rsx writer, built on top of the Mesa pretty-printer algorithm.
//!
//! The writer walks the rsx body and emits a stream of `Begin`/`End` box
//! tokens, `Break` tokens and `String` tokens into a [`Printer`]. The printer
//! then decides where lines actually break based on what fits within the
//! margin:
//!
//! - An element/component body lives in a consistent outer box. If everything
//!   fits on one line we get `div { class: "x", "hi" }`, otherwise every break
//!   in the box fires.
//! - Attributes live in a nested consistent box of their own. When the outer
//!   box breaks but the attributes still fit, they stay on the opening line
//!   (`div { class: "x",` followed by indented children). When the attribute
//!   box itself overflows, each attribute gets its own line.
//! - Comments are emitted as plain words followed by hard breaks, which forces
//!   every enclosing box to break around them.
//!
//! Comments and verbatim multi-line expressions are recovered from the
//! original source by span, since the rsx AST does not preserve them.

use crate::{IndentOptions, lexstate::LexState, mesa};
use dioxus_rsx::*;
use mesa::{BreakToken, Printer};
use proc_macro2::{LineColumn, Span};
use quote::ToTokens;
use regex::Regex;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Result,
};
use syn::{Expr, spanned::Spanned, token::Brace};

/// Whether a trimmed source line is a comment (`//` line comment, or the start
/// or end of a `/* */` block comment)
fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.ends_with("*/")
}

/// Whether a trimmed source line is a standalone comment that can be spliced
/// verbatim into pretty-printed expression output: a `//` line comment or a
/// single-line `/* */` block comment
fn is_standalone_comment(trimmed: &str) -> bool {
    trimmed.starts_with("//") || (trimmed.starts_with("/*") && trimmed.ends_with("*/"))
}

pub struct Writer<'a> {
    pub raw_src: &'a str,
    pub src: Vec<&'a str>,
    pub cached_formats: HashMap<LineColumn, String>,
    pub indent: IndentOptions,
    pub invalid_exprs: Vec<Span>,
    out: Printer,
    /// The last token emitted was an inline `// ...` comment, so the next
    /// separator must be a hard break to avoid swallowing what follows it
    comment_pending: bool,
    /// Current rsx nesting depth in units of one indent, used to estimate the
    /// line budget for the legacy one-liner and if-chain fitting heuristics
    depth: usize,
}

impl<'a> Writer<'a> {
    pub fn new(raw_src: &'a str, indent: IndentOptions) -> Self {
        Self {
            src: raw_src.lines().collect(),
            raw_src,
            indent,
            cached_formats: HashMap::new(),
            invalid_exprs: Vec::new(),
            out: Printer::new(),
            comment_pending: false,
            depth: 0,
        }
    }

    fn w(&self) -> isize {
        mesa::INDENT
    }

    /// Finish printing and take the formatted output, converting the printer's
    /// space-based indentation to the configured indent string
    pub fn take_output(&mut self) -> String {
        let printer = std::mem::replace(&mut self.out, Printer::new());
        self.comment_pending = false;
        let out = printer.eof();

        let four = " ".repeat(mesa::INDENT as usize);
        if self.indent.indent_str() == four {
            return out;
        }

        // Convert leading runs of the printer's 4-space indents into the
        // configured indent string, skipping lines inside multiline strings
        let mut state = LexState::default();
        let mut result = String::with_capacity(out.len());
        for (i, line) in out.split('\n').enumerate() {
            if i > 0 {
                result.push('\n');
            }
            if state.is_in_string() {
                result.push_str(line);
            } else {
                let spaces = line.chars().take_while(|&c| c == ' ').count();
                let indents = spaces / four.len();
                let remainder = spaces % four.len();
                for _ in 0..indents {
                    result.push_str(self.indent.indent_str());
                }
                result.push_str(&" ".repeat(remainder));
                result.push_str(&line[spaces..]);
            }
            state.advance(line);
        }
        result
    }

    pub fn consume(mut self) -> Option<String> {
        Some(self.take_output())
    }

    // ---- token emission helpers ----

    fn word(&mut self, s: impl Into<Cow<'static, str>>) {
        self.out.word(s);
    }

    /// A single-space break that turns into a newline when the enclosing box
    /// breaks. Promoted to a hard break if an inline comment was just written.
    fn space_break(&mut self) {
        if std::mem::take(&mut self.comment_pending) {
            self.out.hardbreak();
        } else {
            self.out.space();
        }
    }

    fn hard_break(&mut self) {
        self.comment_pending = false;
        self.out.hardbreak();
    }

    /// A single-space separator that never turns into a newline, used when the
    /// legacy heuristics have already decided a block fits on one line
    fn inline_space(&mut self) {
        if std::mem::take(&mut self.comment_pending) {
            self.out.hardbreak();
        } else {
            self.out.scan_break(BreakToken {
                blank_space: 1,
                never_break: true,
                ..BreakToken::default()
            });
        }
    }

    pub fn write_rsx_call(&mut self, body: &CallBody, indent_level: usize) -> Result {
        self.out = Printer::new();
        self.comment_pending = false;
        self.depth = indent_level + 1;
        self.out
            .set_base_indent(indent_level * self.w() as usize);

        let roots = &body.body.roots;
        if roots.is_empty() {
            return Ok(());
        }

        // Only a lone text node may share a line with the rsx! call itself
        let inlineable = matches!(roots.as_slice(), [BodyNode::Text(_)])
            && !self.children_have_comments(roots)
            && !self.body_has_trailing_comments(body);

        self.out.cbox(self.w());
        if inlineable {
            self.space_break();
        } else {
            self.hard_break();
        }
        self.write_body_node_seq(roots, !inlineable)?;
        self.write_trailing_body_comments(body)?;
        self.out.end();

        Ok(())
    }

    /// Whether the rsx call is short enough to be inlined
    pub(crate) fn is_short_rsx_call(roots: &[BodyNode]) -> bool {
        matches!(roots, [] | [BodyNode::Text(_)])
    }

    /// Write just the nodes of a body (no rsx! wrapper or padding), starting
    /// at column 0 with each node on its own line. Used when formatting rsx!
    /// macros nested inside other expressions.
    pub fn write_body_nodes(&mut self, roots: &[BodyNode]) -> Result {
        self.out = Printer::new();
        self.comment_pending = false;
        self.depth = 0;

        if roots.is_empty() {
            return Ok(());
        }

        self.out.cbox(0);
        self.write_body_node_seq(roots, !Self::is_short_rsx_call(roots))?;
        self.out.end();
        Ok(())
    }

    /// Full-line comments between the last node and the body's closing brace.
    /// Only lines inside the body count - the backwards walk from the closing
    /// brace must not escape above the body's opening line.
    fn trailing_body_comments(&mut self, body: &CallBody) -> VecDeque<usize> {
        let Some(span) = body.span else {
            return VecDeque::new();
        };
        let mut comments = self.accumulate_full_line_comments(span.span().end());
        // The backwards walk from the closing brace must not escape above the
        // body's own contents and pick up comments preceding the rsx! call
        if let Some(last) = body.body.roots.last() {
            let last_line = Self::final_span_of_node(last).end().line;
            comments.retain(|&id| id >= last_line);
        }
        comments
    }

    fn body_has_trailing_comments(&mut self, body: &CallBody) -> bool {
        let comments = self.trailing_body_comments(body);
        comments
            .iter()
            .any(|&id| self.src.get(id).is_some_and(|l| is_comment_line(l.trim())))
    }

    fn write_trailing_body_comments(&mut self, body: &CallBody) -> Result {
        let comments = self.trailing_body_comments(body);
        let has_real_comment = comments
            .iter()
            .any(|&id| self.src.get(id).is_some_and(|l| is_comment_line(l.trim())));
        if has_real_comment {
            self.emit_line_comments(comments)?;
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

    fn write_element(&mut self, el: &Element) -> Result {
        let Element {
            name,
            raw_attributes: attributes,
            children,
            spreads,
            brace,
            ..
        } = el;

        self.word(format!("{name} "));
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

        // Same idea with generics, write those via the to_tokens method and then remove all whitespace
        if let Some(generics) = generics {
            let mut written = generics.to_token_stream().to_string();
            written.retain(|c| !c.is_whitespace());
            name.push_str(&written);
        }

        self.word(format!("{name} "));
        self.write_rsx_block(fields, spreads, &children.roots, &brace.unwrap_or_default())?;

        Ok(())
    }

    fn write_text_node(&mut self, text: &TextNode) -> Result {
        // Multiline string literals keep their raw newlines; the printer
        // passes embedded newlines through verbatim with no indentation
        self.word(text.input.to_string_with_quotes());
        Ok(())
    }

    fn write_expr_node(&mut self, expr: &ExprNode) -> Result {
        self.write_partial_expr(expr.expr.as_expr(), expr.span())
    }

    fn write_for_loop(&mut self, forloop: &ForLoop) -> Result {
        self.word(format!("for {} in ", self.unparse_pat(&forloop.pat)));

        self.write_inline_expr(&forloop.expr)?;

        if forloop.body.is_empty() {
            self.word("}");
            return Ok(());
        }

        self.write_block_body(&forloop.body.roots)?;

        Ok(())
    }

    fn write_if_chain(&mut self, ifchain: &IfChain) -> Result {
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

            self.word(format!("{} ", if_token.to_token_stream()));

            self.write_inline_expr(cond)?;

            self.write_block_body_open(&then_branch.roots)?;

            if let Some(else_if_branch) = else_if_branch {
                self.word("} else ");
                branch = Some(else_if_branch);
            } else if let Some(else_branch) = else_branch {
                self.word("} else {");
                self.write_block_body_open(&else_branch.roots)?;
                self.word("}");
                branch = None;
            } else {
                self.word("}");
                branch = None;
            }
        }

        Ok(())
    }

    /// Write the always-broken body of a for loop or if chain, followed by the
    /// closing brace
    fn write_block_body(&mut self, children: &[BodyNode]) -> Result {
        self.write_block_body_open(children)?;
        self.word("}");
        Ok(())
    }

    /// Write the always-broken body of a block, leaving the cursor at the
    /// start of the line that should hold the closing brace
    fn write_block_body_open(&mut self, children: &[BodyNode]) -> Result {
        self.out.cbox(self.w());
        self.hard_break();
        self.depth += 1;
        self.write_body_node_seq(children, true)?;
        self.depth -= 1;
        self.out.scan_break(BreakToken {
            blank_space: mesa::SIZE_INFINITY_SPACE,
            offset: -self.w(),
            ..BreakToken::default()
        });
        self.comment_pending = false;
        self.out.end();
        Ok(())
    }

    /// An expression within a for or if block that might need to be spread out
    /// across several lines, followed by the opening brace
    fn write_inline_expr(&mut self, expr: &Expr) -> Result {
        let unparsed = self.unparse_expr(expr);
        let mut lines = unparsed.lines();
        let first_line = lines.next().ok_or(std::fmt::Error)?;

        self.word(first_line.to_string());

        let mut was_multiline = false;

        for line in lines {
            was_multiline = true;
            self.hard_break();
            self.word(line.to_string());
        }

        if was_multiline {
            self.hard_break();
            self.word("{");
        } else {
            self.word(" {");
        }

        Ok(())
    }

    /// Write a sequence of body nodes with their preceding comments. `hard`
    /// forces every node onto its own line; otherwise the enclosing box
    /// decides.
    fn write_body_node_seq(&mut self, children: &[BodyNode], hard: bool) -> Result {
        let mut is_first = true;

        for child in children {
            if !is_first {
                if hard {
                    self.hard_break();
                } else {
                    self.space_break();
                }
            }

            if self.current_span_is_primary(child.span().start()) {
                let comments = self.accumulate_full_line_comments(child.span().start());
                let has_real_comment = comments
                    .iter()
                    .any(|&id| self.src.get(id).is_some_and(|l| is_comment_line(l.trim())));
                if has_real_comment || !is_first {
                    self.emit_line_comments_before_item(comments)?;
                }
            }
            is_first = false;

            self.write_ident(child)?;
        }

        Ok(())
    }

    /// Basically elements and components are the same thing
    ///
    /// This writes the contents out for both in one function, centralizing the
    /// annoying logic like key handling, breaks, closures, etc
    fn write_rsx_block(
        &mut self,
        attributes: &[Attribute],
        spreads: &[Spread],
        children: &[BodyNode],
        brace: &Brace,
    ) -> Result {
        self.word("{");

        let has_attrs = !attributes.is_empty() || !spreads.is_empty();
        let has_children = !children.is_empty();

        let trailing_comments = if self.leading_row_is_empty(brace.span.span().end()) {
            let comments = self.accumulate_full_line_comments(brace.span.span().end());
            let has_real_comment = comments
                .iter()
                .any(|&id| self.src.get(id).is_some_and(|l| is_comment_line(l.trim())));
            has_real_comment.then_some(comments)
        } else {
            None
        };

        // Empty blocks print as `div {}`, but comments inside them survive
        if !has_attrs && !has_children && trailing_comments.is_none() {
            self.write_todo_body(brace)?;
            self.word("}");
            return Ok(());
        }

        self.depth += 1;

        // A lone child that's a text node, expression, or empty component may
        // share a line with its parent
        let children_inline = trailing_comments.is_none()
            && !self.children_have_comments(children)
            && match children {
                [] | [BodyNode::Text(_)] | [BodyNode::RawExpr(_)] => true,
                [BodyNode::Component(comp)] => {
                    comp.fields.is_empty() && comp.children.is_empty() && comp.spreads.is_empty()
                }
                _ => false,
            };

        let attrs_have_comments = self.attrs_have_comments(attributes, spreads, brace);

        // Estimated width-based fitting, mirroring the original formatter's
        // heuristics: the attribute list may share the opening line only when
        // its estimated width fits in 80 columns, and the whole block becomes a
        // one-liner when attributes plus a short lone child fit in 100 columns
        let attr_len = self.is_short_attrs(attributes, spreads);
        let attr_indent = (self.depth - 1) * 4;
        let attrs_fit = attr_len + attr_indent < 80;
        let force_inline = children_inline
            && attrs_fit
            && !attrs_have_comments
            && !self.comment_pending
            && trailing_comments.is_none()
            && !self.indent.split_line_attributes()
            && !self.has_inline_comment(brace.span.span().start(), 1)
            && children.last().is_none_or(|child| {
                !self.has_inline_comment(Self::final_span_of_node(child).end(), 0)
            })
            && match self.children_inline_len(children) {
                Some(children_len) => attr_len + children_len + attr_indent < 100,
                None => false,
            };

        self.out.cbox(self.w());

        // A comment on the same line as the opening brace
        self.write_inline_comments(brace.span.span().start(), 1)?;

        // A single comment-free attribute is glued onto the opening line no
        // matter how long it is; readability suffers more from breaking it.
        // But only if its value won't span multiple lines.
        let single_attr_multiline = attributes.len() == 1
            && spreads.is_empty()
            && self.attr_value_is_multiline(&attributes[0]);
        let glue_single_attr = attributes.len() + spreads.len() == 1
            && !attrs_have_comments
            && !self.comment_pending
            && trailing_comments.is_none()
            && !single_attr_multiline;

        // Attributes get their own consistent box. When the outer box is
        // broken but this one still fits, the attributes stay on the opening
        // line with the children spread below them.
        let mut wrote_explicit_comma = false;
        if has_attrs {
            enum AttrType<'b> {
                Attr(&'b Attribute),
                Spread(&'b Spread),
            }

            let items: Vec<AttrType> = attributes
                .iter()
                .map(AttrType::Attr)
                .chain(spreads.iter().map(AttrType::Spread))
                .collect();
            let last = items.len() - 1;

            // Attributes whose estimated width doesn't fit each go on their
            // own line (this also covers more than 3 attributes)
            let force_split = self.indent.split_line_attributes()
                || attrs_have_comments
                || !attrs_fit;

            self.out.cbox(0);
            for (i, attr) in items.iter().enumerate() {
                if glue_single_attr {
                    self.out.nbsp();
                } else if force_split {
                    self.hard_break();
                } else if force_inline {
                    self.inline_space();
                } else {
                    self.space_break();
                }

                let attr_span = match attr {
                    AttrType::Attr(attr) => attr.span(),
                    AttrType::Spread(attr) => attr.expr.span(),
                };
                self.write_attr_comments(brace, attr_span)?;

                match attr {
                    AttrType::Attr(attr) => self.write_attribute(attr)?,
                    AttrType::Spread(attr) => self.write_spread_attribute(&attr.expr)?,
                }

                let comma_span = match attr {
                    AttrType::Attr(attr) => attr
                        .comma
                        .as_ref()
                        .map(|c| c.span())
                        .unwrap_or_else(|| self.total_span_of_attr(attr)),
                    AttrType::Spread(attr) => attr.span(),
                };

                let is_last = i == last;
                if !is_last || has_children {
                    self.word(",");
                } else if trailing_comments.is_some() || self.has_inline_comment(comma_span.end(), 0)
                {
                    // The closing break can't carry the trailing comma when a
                    // comment sits between the attribute and the closing brace
                    self.word(",");
                    wrote_explicit_comma = true;
                }

                self.write_inline_comments(comma_span.end(), 0)?;
            }
            self.out.end();
        }

        if has_children {
            for child in children {
                if force_inline {
                    self.inline_space();
                } else if children_inline {
                    self.space_break();
                } else {
                    self.hard_break();
                }
                if self.current_span_is_primary(child.span().start()) {
                    let comments = self.accumulate_full_line_comments(child.span().start());
                    self.emit_line_comments_before_item(comments)?;
                }
                self.write_ident(child)?;
            }
        }

        // Comments between the last node and the closing brace
        if let Some(comments) = trailing_comments {
            self.emit_line_comments(comments)?;
        }

        // The closing break carries the trailing comma when the attribute list
        // is the last thing in the block and ends up broken
        let trailing_comma =
            has_attrs && !has_children && !wrote_explicit_comma && !glue_single_attr;
        if self.comment_pending || (has_children && !children_inline) {
            self.out.scan_break(BreakToken {
                blank_space: mesa::SIZE_INFINITY_SPACE,
                offset: -self.w(),
                ..BreakToken::default()
            });
        } else if (glue_single_attr && !has_children) || force_inline {
            // The glued attribute stays put, so the closing brace does too
            self.out.scan_break(BreakToken {
                blank_space: 1,
                offset: -self.w(),
                never_break: true,
                ..BreakToken::default()
            });
        } else {
            self.out.scan_break(BreakToken {
                blank_space: 1,
                offset: -self.w(),
                pre_break: trailing_comma.then_some(','),
                ..BreakToken::default()
            });
        }
        self.comment_pending = false;
        self.out.end();
        self.word("}");
        self.depth -= 1;

        Ok(())
    }

    /// Whether any attribute or spread has full-line comments above it
    fn attrs_have_comments(
        &self,
        attributes: &[Attribute],
        spreads: &[Spread],
        brace: &Brace,
    ) -> bool {
        let brace_line = brace.span.span().start().line;
        let spans = attributes
            .iter()
            .map(|a| a.span())
            .chain(spreads.iter().map(|s| s.expr.span()));

        for span in spans {
            if span.start().line == brace_line || !self.current_span_is_primary(span.start()) {
                continue;
            }
            'line: for line in self.src[..span.start().line - 1].iter().rev() {
                match (is_comment_line(line.trim()), line.is_empty()) {
                    (true, _) => return true,
                    (_, true) => continue 'line,
                    _ => break 'line,
                }
            }
        }

        false
    }

    /// Whether a single attribute's value will definitely span multiple lines
    fn attr_value_is_multiline(&mut self, attr: &Attribute) -> bool {
        if attr.can_be_shorthand() {
            return false;
        }
        let expr = match &attr.value {
            AttributeValue::EventTokens(closure) => closure.as_expr(),
            AttributeValue::AttrExpr(value) => value.as_expr(),
            _ => return false,
        };
        let Ok(expr) = expr else {
            return true;
        };
        if self.retrieve_formatted_expr(&expr).contains('\n') {
            return true;
        }
        // Source comments get merged back into the value, making it multiline
        // even when the comment-free pretty output fits on one line
        let span = expr.span();
        (span.start().line..span.end().line)
            .filter_map(|idx| self.src.get(idx))
            .any(|line| line.trim().starts_with("//"))
    }

    fn write_attribute(&mut self, attr: &Attribute) -> Result {
        self.write_attribute_name(&attr.name)?;

        if !attr.can_be_shorthand() {
            if let AttributeValue::IfExpr(if_chain) = &attr.value {
                let inline_len = self.attr_value_len(&attr.value);
                let line_budget = 80usize.saturating_sub(self.depth * 4);
                if inline_len > line_budget {
                    self.word(":");
                    self.out.cbox(0);
                    self.hard_break();
                    self.write_attribute_if_chain_multiline(if_chain)?;
                    self.out.end();
                    return Ok(());
                }
            }
            self.word(": ");
            self.write_attribute_value(&attr.value)?;
        }

        Ok(())
    }

    /// Estimated single-line width of an attribute value; very large when the
    /// value can never be rendered on one line
    fn attr_value_len(&mut self, value: &AttributeValue) -> usize {
        match value {
            AttributeValue::IfExpr(if_chain) => {
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

    fn span_has_line_comments(&self, span: Span) -> bool {
        span.source_text().is_some_and(|source| {
            source
                .lines()
                .any(|line| line.trim_start().starts_with("//"))
        })
    }

    /// Estimated total single-line width of an attribute list; very large when
    /// it can never be rendered on one line
    fn is_short_attrs(&mut self, attributes: &[Attribute], spreads: &[Spread]) -> usize {
        let mut total = 0;

        // No more than 3 attributes before breaking the line
        if attributes.len() > 3 {
            return 100000;
        }

        for attr in attributes {
            total += match &attr.name {
                AttributeName::BuiltIn(name) => name.to_string().len(),
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

    /// Single-line width of an inline-able lone child, or None when the child
    /// must be spread across lines
    fn children_inline_len(&mut self, children: &[BodyNode]) -> Option<usize> {
        match children {
            [] => Some(0),
            [BodyNode::Text(text)] => Some(text.input.to_string_with_quotes().len()),
            [BodyNode::RawExpr(expr)] => {
                let expr = expr.expr.as_expr().ok()?;
                if self.span_has_line_comments(expr.span()) {
                    return None;
                }
                let pretty = self.retrieve_formatted_expr(&expr);
                if pretty.contains('\n') {
                    None
                } else {
                    Some(pretty.len() + 2)
                }
            }
            [BodyNode::Component(comp)]
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
            _ => None,
        }
    }

    fn write_attribute_name(&mut self, attr: &AttributeName) -> Result {
        match attr {
            AttributeName::BuiltIn(name) => self.word(name.to_string()),
            AttributeName::Custom(name) => self.word(name.to_token_stream().to_string()),
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
                self.word(value.to_string());
            }
            AttributeValue::Shorthand(value) => {
                self.word(value.to_string());
            }
            AttributeValue::EventTokens(closure) => {
                self.write_partial_expr(closure.as_expr(), closure.span())?;
            }
            AttributeValue::AttrExpr(value) => {
                self.write_partial_expr(value.as_expr(), value.span())?;
            }
        }

        Ok(())
    }

    /// An attribute if chain that fits within the line budget, written inline
    /// as unbreakable words
    fn write_attribute_if_chain(&mut self, if_chain: &IfAttributeValue) -> Result {
        let mut chain = Some(if_chain);
        while let Some(current) = chain {
            let cond = self.unparse_expr(&current.if_expr.cond);
            self.word(format!("if {cond} {{ "));
            self.write_attribute_value(&current.then_value)?;
            self.word(" }");

            match current.else_value.as_deref() {
                Some(AttributeValue::IfExpr(else_if_chain)) => {
                    self.word(" else ");
                    chain = Some(else_if_chain);
                }
                Some(other) => {
                    self.word(" else { ");
                    self.write_attribute_value(other)?;
                    self.word(" }");
                    chain = None;
                }
                None => chain = None,
            }
        }
        Ok(())
    }

    /// An attribute if chain too long for one line: every branch body sits on
    /// its own indented line
    fn write_attribute_if_chain_multiline(&mut self, if_chain: &IfAttributeValue) -> Result {
        let mut chain = Some(if_chain);
        while let Some(current) = chain {
            let cond = self.unparse_expr(&current.if_expr.cond);
            self.word(format!("if {cond} {{"));
            self.out.cbox(self.w());
            self.hard_break();
            self.write_attribute_value(&current.then_value)?;
            self.out.scan_break(BreakToken {
                blank_space: mesa::SIZE_INFINITY_SPACE,
                offset: -self.w(),
                ..BreakToken::default()
            });
            self.out.end();
            self.word("}");

            match current.else_value.as_deref() {
                Some(AttributeValue::IfExpr(else_if_chain)) => {
                    self.word(" else ");
                    chain = Some(else_if_chain);
                }
                Some(other) => {
                    self.word(" else {");
                    self.out.cbox(self.w());
                    self.hard_break();
                    self.write_attribute_value(other)?;
                    self.out.scan_break(BreakToken {
                        blank_space: mesa::SIZE_INFINITY_SPACE,
                        offset: -self.w(),
                        ..BreakToken::default()
                    });
                    self.out.end();
                    self.word("}");
                    chain = None;
                }
                None => chain = None,
            }
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
            // Get the raw line of the attribute
            let line = self.src.get(attr_line - 1).unwrap_or(&"");

            // Only write comments if the line is empty before the attribute start
            let row_start = line.get(..attr_span.start().column - 1).unwrap_or("");
            if !row_start.trim().is_empty() {
                return Ok(());
            }

            let comments = self.accumulate_full_line_comments(attr_span.start());
            self.emit_line_comments_before_item(comments)?;
        }

        Ok(())
    }

    /// Whether an inline `// ...` comment follows the given location on the
    /// same source line
    fn has_inline_comment(&self, final_span: LineColumn, offset: usize) -> bool {
        if final_span.line == 1 && final_span.column == 0 {
            return false;
        }
        let Some(src_line) = self.src.get(final_span.line - 1) else {
            return false;
        };
        let Some(whitespace) = src_line.get(final_span.column..).map(|s| s.trim()) else {
            return false;
        };
        if whitespace.is_empty() || whitespace.len() < offset {
            return false;
        }
        whitespace[offset..].trim().starts_with("//")
    }

    fn write_inline_comments(&mut self, final_span: LineColumn, offset: usize) -> Result {
        if !self.has_inline_comment(final_span, offset) {
            return Ok(());
        }
        let src_line = self.src[final_span.line - 1];
        let comment = src_line[final_span.column..].trim()[offset..].trim().to_string();
        // Zero-width so the comment doesn't push the line past the margin and
        // force the enclosing box to break
        self.out.scan_string_zero_width(format!(" {comment}").into());
        self.comment_pending = true;
        Ok(())
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
        // the rest should be `//` comments or `/* */` block comments
        let mut last_line_was_empty = false;
        let mut in_block_comment = false;
        for (id, line) in lines.iter().enumerate().rev() {
            let trimmed = line.trim();
            if in_block_comment {
                comments.push_front(id);
                last_line_was_empty = false;
                if trimmed.starts_with("/*") {
                    in_block_comment = false;
                }
            } else if trimmed.starts_with("//") {
                comments.push_front(id);
                last_line_was_empty = false;
            } else if trimmed.ends_with("*/") {
                comments.push_front(id);
                last_line_was_empty = false;
                in_block_comment = !trimmed.starts_with("/*");
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

    /// Emit full-line comments that precede an item. The cursor is at the
    /// start of the item's line; each comment is followed by a hard break so
    /// the item lands on its own line below them.
    fn emit_line_comments_before_item(&mut self, mut comments: VecDeque<usize>) -> Result {
        while let Some(comment_line) = comments.pop_front() {
            let Some(line) = self.src.get(comment_line) else {
                continue;
            };

            let line = line.trim();

            if line.is_empty() {
                self.hard_break();
            } else {
                self.word(line.to_string());
                self.hard_break();
            }
        }
        Ok(())
    }

    /// Emit trailing full-line comments. The cursor is at the end of the last
    /// item; each comment goes on a fresh line and the output ends at the end
    /// of the final comment.
    fn emit_line_comments(&mut self, mut comments: VecDeque<usize>) -> Result {
        while let Some(comment_line) = comments.pop_front() {
            let Some(line) = self.src.get(comment_line) else {
                continue;
            };

            let line = line.trim();

            self.hard_break();
            if !line.is_empty() {
                self.word(line.to_string());
                self.comment_pending = true;
            }
        }
        Ok(())
    }

    /// Write the comments inside an empty set of braces, e.g.
    /// `div { // TODO }` spread over multiple lines. Returns whether anything
    /// was written.
    fn write_todo_body(&mut self, brace: &Brace) -> std::result::Result<bool, std::fmt::Error> {
        let span = brace.span.span();
        let start = span.start();
        let end = span.end();

        if start.line == end.line {
            return Ok(false);
        }

        let comments: Vec<String> = (start.line..end.line)
            .filter_map(|idx| {
                let line = self.src.get(idx)?;
                line.trim()
                    .starts_with("//")
                    .then_some(line.trim().to_string())
            })
            .collect();

        if comments.is_empty() {
            return Ok(false);
        }

        self.out.cbox(self.w());
        for comment in comments {
            self.hard_break();
            self.word(comment);
        }
        self.out.scan_break(BreakToken {
            blank_space: mesa::SIZE_INFINITY_SPACE,
            offset: -self.w(),
            ..BreakToken::default()
        });
        self.out.end();

        Ok(true)
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
            .any(|line| is_standalone_comment(line.trim_start()));
        let mut src_lines = source.lines().peekable();

        // Comments already in pretty output (from nested rsx!) - skip these from source
        let pretty_comments: HashSet<_> = pretty
            .lines()
            .filter(|l| is_standalone_comment(l.trim()))
            .map(|l| l.trim())
            .collect();

        let mut out = String::new();

        if src_lines.peek().is_none() {
            out = pretty;
        } else {
            // When one source line expands into several pretty lines (e.g. a
            // one-line nested rsx! that gets broken up), this holds the not yet
            // consumed compacted remainder of that source line so the iterators
            // stay in sync instead of drifting and dropping comments
            let mut src_carry = String::new();
            for line in pretty.lines() {
                let trimmed = line.trim();
                let compacted = line.replace(" ", "").replace(",", "");

                if !src_carry.is_empty() {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(line);
                    src_carry = src_carry
                        .strip_prefix(&compacted)
                        .unwrap_or_default()
                        .to_string();
                    continue;
                }

                // Pretty comments: consume matching source lines, preserve preceding empty lines
                if is_standalone_comment(trimmed) {
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
                let mut multiline: Option<(Vec<&str>, bool)> = None;

                while let Some(src) = src_lines.peek() {
                    let src_trimmed = src.trim();

                    if src_trimmed.is_empty() || is_standalone_comment(src_trimmed) {
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

                    // Multi-line method chain (e.g., foo\n  .bar()\n  .baz()),
                    // or - when the source has comments that could otherwise be
                    // lost to iterator drift - any multi-line construct
                    if !src_compacted.is_empty() && compacted.starts_with(&src_compacted) {
                        let is_call = src_trimmed.ends_with('(')
                            || src_trimmed.ends_with(',')
                            || src_trimmed.ends_with('{');
                        if source_has_line_comments || !is_call {
                            // Splices entered only because the source has
                            // comments must actually contain one to be emitted
                            multiline = Some((vec![*src], is_call));
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
                        out.push_str(self.indent.indent_str());
                    }
                    out.push_str(comment);
                    out.push('\n');
                }

                // Handle multi-line method chains
                if let Some((mut ml, requires_comments)) = multiline {
                    // Compact a source line, ignoring any trailing line comment
                    let compact_src = |s: &str| {
                        let code = COMMENT_REGEX.with(|r| {
                            match r.captures(s).and_then(|c| c.get(1)) {
                                Some(m) => s[..m.start()].to_string(),
                                None => s.to_string(),
                            }
                        });
                        code.replace(" ", "").replace(",", "")
                    };

                    src_lines.next();
                    let mut acc = compact_src(ml[0]);
                    let mut matched = acc == compacted;

                    while let Some(src) = src_lines.peek() {
                        let t = src.trim();
                        if is_standalone_comment(t) {
                            ml.push(src);
                            src_lines.next();
                            continue;
                        }
                        if t.is_empty() {
                            src_lines.next();
                            continue;
                        }

                        acc.push_str(&compact_src(src));
                        ml.push(src);

                        if acc == compacted {
                            matched = true;
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

                    // The source lines must compact to exactly the pretty line, otherwise
                    // emitting them verbatim would corrupt the expression. Fall back to the
                    // pretty line if the match failed.
                    if !matched {
                        out.push_str(line);
                        continue;
                    }

                    // Splicing such a block verbatim only exists to preserve the
                    // comments inside it; without any, the pretty line is
                    // canonical and re-emitting the source would not be
                    // idempotent. Blocks containing nested rsx! are also
                    // canonical in pretty form, since the macro gets reformatted
                    let is_chain = ml.iter().skip(1).any(|l| {
                        let t = l.trim();
                        t.starts_with('.')
                            || t.starts_with("&&")
                            || t.starts_with("||")
                            || matches!(t.chars().next(), Some('+' | '-' | '*' | '/' | '?'))
                    });
                    // A gather whose last line opens a block continues into the
                    // following pretty lines, so splicing it verbatim would mix
                    // source and pretty layouts
                    let opens_block = ml
                        .last()
                        .is_some_and(|l| matches!(l.trim_end().chars().last(), Some('{' | '(' | '[')));
                    let pretty_is_canonical = requires_comments
                        || !is_chain
                        || opens_block
                        || ml.iter().any(|l| l.contains("rsx!") || l.contains("render!"));
                    if pretty_is_canonical
                        && !ml
                            .iter()
                            .any(|l| is_standalone_comment(l.trim()) || l.contains("//"))
                    {
                        out.push_str(line);
                        continue;
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
                    if let Some(src_line) = src_lines.next() {
                        let src_code = match COMMENT_REGEX
                            .with(|r| r.captures(src_line).and_then(|cap| cap.get(1)))
                        {
                            Some(c) => &src_line[..c.start()],
                            None => src_line,
                        };
                        let src_compacted = src_code.replace(" ", "").replace(",", "");

                        // Only attach the comment if the source line's code actually matches
                        // the pretty line, otherwise the iterators have drifted apart and the
                        // comment belongs somewhere else
                        if let Some(cap) = COMMENT_REGEX.with(|r| r.captures(src_line))
                            && let Some(c) = cap.get(1)
                            && src_compacted == compacted
                        {
                            out.push_str(" // ");
                            out.push_str(c.as_str().replace("//", "").trim());
                        }

                        // The source line continues past this pretty line; the
                        // remainder corresponds to the following pretty lines
                        if let Some(rest) = src_compacted.strip_prefix(&compacted)
                            && !rest.is_empty()
                        {
                            src_carry = rest.to_string();
                        }
                    }
                }
            }
        }

        self.write_verbatim_multiline(out)?;
        Ok(())
    }

    /// Emit a pre-formatted, potentially multi-line chunk of Rust code. The
    /// first line continues the current line; subsequent lines keep their own
    /// relative indentation on top of the current indent. Lines that start
    /// inside a multiline string literal are passed through verbatim with no
    /// indentation at all.
    fn write_verbatim_multiline(&mut self, text: impl Into<String>) -> Result {
        let text = text.into();
        let mut lines = text.split('\n').peekable();
        let first = lines.next().unwrap_or_default();

        self.word(first.to_string());

        let mut state = LexState::default();
        state.advance(first);

        let mut last_line: &str = first;
        while let Some(line) = lines.next() {
            if state.is_in_string() {
                // Embedded newline inside a word bypasses the printer's
                // indentation, keeping the string contents verbatim
                self.word(format!("\n{line}"));
            } else if line.trim().is_empty() {
                // Blank line — a zero-width "\n" glued to the previous line's
                // content; the next line's hard break supplies the second
                // newline, producing a visual blank with no trailing spaces
                self.out.scan_string_zero_width("\n".into());
            } else {
                self.hard_break();
                self.word(line.to_string());
            }
            state.advance(line);
            if lines.peek().is_none() {
                last_line = line;
            }
        }

        // If the chunk ends in a line comment, the next break must be hard
        thread_local! {
            static COMMENT_REGEX: Regex = Regex::new("\"[^\"]*\"|(//.*)").unwrap();
        }
        if COMMENT_REGEX
            .with(|r| r.captures(last_line).and_then(|c| c.get(1)).is_some())
        {
            self.comment_pending = true;
        }

        Ok(())
    }

    fn write_spread_attribute(&mut self, attr: &Expr) -> Result {
        let formatted = self.unparse_expr(attr);
        self.word("..");
        self.write_verbatim_multiline(formatted)?;
        Ok(())
    }

    fn children_have_comments(&self, children: &[BodyNode]) -> bool {
        for child in children {
            if self.current_span_is_primary(child.span().start()) {
                'line: for line in self.src[..child.span().start().line - 1].iter().rev() {
                    match (is_comment_line(line.trim()), line.is_empty()) {
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
            BodyNode::IfChain(i) => {
                let mut chain = i;
                while let Some(next) = &chain.else_if_branch {
                    chain = next;
                }
                match chain.else_brace {
                    Some(b) => b.span.span(),
                    None => chain.then_brace.span.span(),
                }
            }
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
}
