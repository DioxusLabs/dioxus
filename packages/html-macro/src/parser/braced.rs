use crate::parser::HtmlParser;
use crate::tag::{Tag, TagKind};
use proc_macro2::Span;
use quote::quote;
use syn::spanned::Spanned;
use syn::Block;

impl HtmlParser {
    /// Parse an incoming Tag::Braced text node
    pub(crate) fn parse_braced(
        &mut self,
        block: &Box<Block>,
        brace_span: &Span,
        next_tag: Option<&Tag>,
    ) {
        // We'll check to see if there is a space between this block and the previous open
        // tag's closing brace.
        //
        // If so we'll then check if the node in this block is a text node. If it is we'll
        // insert a single white space before it.
        //
        // let some_var = "hello"
        // let another_var = "world";
        //
        // html! { <span>{some_var}</span> }  -> would not get a " " inserted
        //
        // html! { <span> {some_var}</span> } -> would get a " " inserted
        let mut insert_whitespace_before_text = false;
        if let Some(open_tag_end) = self.recent_span_locations.most_recent_open_tag_end.as_ref() {
            if self.last_tag_kind == Some(TagKind::Open)
                && self.separated_by_whitespace(open_tag_end, brace_span)
            {
                insert_whitespace_before_text = true;
            }
        }

        // If
        //   1. The next tag is a closing tag or another braced block
        //   2. There is space between this brace and that next tag / braced block
        //
        // Then
        //   We'll insert some spacing after this brace.
        //
        // This ensures that we properly maintain spacing between two neighboring braced
        // text nodes
        //
        // html! { <div>{ This Brace } { Space WILL be inserted }</div>
        //   -> <div>This Brace Space WILL be inserted</div>
        //
        // html! { <div>{ This Brace }{ Space WILL NOT be inserted }</div>
        //   -> <div>This BraceSpace WILL NOT be inserted</div>
        let insert_whitespace_after_text = match next_tag {
            Some(Tag::Close {
                first_angle_bracket_span,
                ..
            }) => self.separated_by_whitespace(brace_span, &first_angle_bracket_span),
            Some(Tag::Braced {
                brace_span: next_brace_span,
                ..
            }) => self.separated_by_whitespace(brace_span, &next_brace_span),
            _ => false,
        };

        // TODO: Only allow one statement per block. Put a quote_spanned! compiler error if
        // there is more than 1 statement. Add a UI test for this.
        block.stmts.iter().for_each(|stmt| {
            if self.current_node_idx == 0 {
                // Here we handle a block being the root node of an `html!` call
                //
                // html { { some_node }  }
                let node = quote! {
                    let node_0: VirtualNode = #stmt.into();
                };
                self.push_tokens(node);
            } else {
                self.parse_statement(stmt);

                if insert_whitespace_before_text {
                    let node = self.current_virtual_node_ident(stmt.span());

                    let insert_whitespace = quote! {
                        #node.first().insert_space_before_text();
                    };

                    self.push_tokens(insert_whitespace);
                }

                if insert_whitespace_after_text {
                    let node = self.current_virtual_node_ident(stmt.span());

                    let insert_whitespace = quote! {
                        #node.last().insert_space_after_text();
                    };

                    self.push_tokens(insert_whitespace);
                }
            }
        });

        self.set_most_recent_block_start(brace_span.clone());
    }
}
