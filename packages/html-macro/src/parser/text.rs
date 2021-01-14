use crate::parser::HtmlParser;
use crate::tag::{Tag, TagKind};
use proc_macro2::{Ident, Span};
use quote::quote;

impl HtmlParser {
    /// Parse an incoming Tag::Text text node
    pub(crate) fn parse_text(
        &mut self,
        text: &str,
        text_start: Span,
        text_end: Span,
        next_tag: Option<&Tag>,
    ) {
        let mut text = text.to_string();

        if self.should_insert_space_before_text(&text_start) {
            text = " ".to_string() + &text;
        }

        let should_insert_space_after_text = match next_tag {
            Some(Tag::Close {
                first_angle_bracket_span,
                ..
            }) => self.separated_by_whitespace(&text_end, first_angle_bracket_span),
            Some(Tag::Braced { brace_span, .. }) => {
                self.separated_by_whitespace(&text_end, brace_span)
            }
            Some(Tag::Open {
                open_bracket_span, ..
            }) => self.separated_by_whitespace(&text_end, open_bracket_span),
            _ => false,
        };
        if should_insert_space_after_text {
            text += " ";
        }

        let idx = &mut self.current_node_idx;
        let parent_to_children = &mut self.parent_to_children;
        let parent_stack = &mut self.parent_stack;
        let tokens = &mut self.tokens;
        let node_order = &mut self.node_order;

        if *idx == 0 {
            node_order.push(0);
            // TODO: This is just a consequence of bad code. We're pushing this to make
            // things work but in reality a text node isn't a parent ever.
            // Just need to make the code DRY / refactor so that we can make things make
            // sense vs. just bolting things together.
            parent_stack.push((0, Ident::new("unused", Span::call_site())));
        }

        let var_name = Ident::new(format!("node_{}", idx).as_str(), Span::call_site());

        let text_node = quote! {
            let mut #var_name = VirtualNode::text(#text);
        };

        tokens.push(text_node);

        if *idx == 0 {
            *idx += 1;
            return;
        }

        let parent_idx = &parent_stack[parent_stack.len() - 1];

        node_order.push(*idx);

        parent_to_children
            .get_mut(&parent_idx.0)
            .expect("Parent of this text node")
            .push(*idx);

        *idx += 1;
    }

    /// If the last TagKind was a block or an open tag we check to see if there is space
    /// between this text and that tag. If so we insert some space before this text.
    fn should_insert_space_before_text(&self, text_start: &Span) -> bool {
        if self.last_tag_kind == Some(TagKind::Braced) {
            let most_recent_block_start = self.recent_span_locations.most_recent_block_start;
            let most_recent_block_start = most_recent_block_start.as_ref().unwrap();

            self.separated_by_whitespace(most_recent_block_start, text_start)
        } else if self.last_tag_kind == Some(TagKind::Open) {
            let most_recent_open_tag_end =
                self.recent_span_locations.most_recent_open_tag_end.as_ref();
            let most_recent_open_tag_end = most_recent_open_tag_end.unwrap();

            self.separated_by_whitespace(most_recent_open_tag_end, text_start)
        } else {
            false
        }
    }
}
