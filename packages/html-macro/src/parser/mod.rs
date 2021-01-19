use crate::tag::TagKind;
use crate::Tag;
use quote::{quote, quote_spanned};
use std::collections::HashMap;
use syn::export::Span;
use syn::spanned::Spanned;
use syn::{Ident, Stmt};

mod braced;
mod close_tag;
mod open_tag;
mod statement;
mod text;

pub enum NodesToPush<'a> {
    Stmt(&'a Stmt),
    TokenStream(&'a Stmt, proc_macro2::TokenStream),
}

/// Used to parse [`Tag`]s that we've parsed and build a tree of `VirtualNode`s
///
/// [`Tag`]: enum.Tag.html
pub struct HtmlParser {
    /// As we parse our macro tokens we'll generate new tokens to return back into the compiler
    /// when we're done.
    tokens: Vec<proc_macro2::TokenStream>,
    /// Everytime we encounter a new node we'll use the current_node_idx to name it.
    /// Then we'll increment the current_idx by one.
    /// This gives every node that we encounter a unique name that we can use to find
    /// it later when we want to push child nodes into parent nodes
    current_node_idx: usize,
    /// The order that we encountered nodes while parsing.
    node_order: Vec<usize>,
    /// Each time we encounter a new node that could possible be a parent node
    /// we push it's node index onto the stack.
    ///
    /// Text nodes cannot be parent nodes.
    parent_stack: Vec<(usize, Ident)>,
    /// Key -> index of the parent node within the HTML tree
    /// Value -> vector of child node indices
    parent_to_children: HashMap<usize, Vec<usize>>,
    /// The locations of the most recent spans that we parsed.
    /// Used to determine whether or not to put space around text nodes.
    recent_span_locations: RecentSpanLocations,
    /// The last kind of tag that we parsed.
    /// Used to determine whether or not to put space around text nodes.
    last_tag_kind: Option<TagKind>,
}

/// TODO: I've hit a good stopping point... but we can clean these methods up / split them up
/// a bit...
impl HtmlParser {
    /// Create a new HtmlParser
    pub fn new() -> HtmlParser {
        let mut parent_to_children: HashMap<usize, Vec<usize>> = HashMap::new();
        parent_to_children.insert(0, vec![]);

        HtmlParser {
            tokens: vec![],
            current_node_idx: 0,
            node_order: vec![],
            parent_stack: vec![],
            parent_to_children,
            recent_span_locations: RecentSpanLocations::default(),
            last_tag_kind: None,
        }
    }

    /// Generate the tokens for the incoming Tag and update our parser's heuristics that keep
    /// track of information about what we've parsed.
    pub fn push_tag(&mut self, tag: &Tag, next_tag: Option<&Tag>) {
        match tag {
            Tag::Open {
                name,
                attrs,
                closing_bracket_span,
                is_self_closing,
                ..
            } => {
                self.parse_open_tag(name, closing_bracket_span, attrs, *is_self_closing);
                self.last_tag_kind = Some(TagKind::Open);
            }
            Tag::Close { name, .. } => {
                self.parse_close_tag(name);
                self.last_tag_kind = Some(TagKind::Close);
            }
            Tag::Text {
                text,
                start_span,
                end_span,
            } => {
                self.parse_text(text, start_span.unwrap(), end_span.unwrap(), next_tag);
                self.last_tag_kind = Some(TagKind::Text);
            }
            Tag::Braced { block, brace_span } => {
                self.parse_braced(block, brace_span, next_tag);
                self.last_tag_kind = Some(TagKind::Braced);
            }
        };
    }

    ///  1. Pop a node off the stack
    ///  2. Look up all of it's children in parent_to_children
    ///  3. Append the children to this node
    ///  4. Move on to the next node (as in, go back to step 1)
    pub fn finish(&mut self) -> proc_macro2::TokenStream {
        let node_order = &mut self.node_order;
        let parent_to_children = &mut self.parent_to_children;
        let tokens = &mut self.tokens;

        if node_order.len() > 1 {
            for _ in 0..(node_order.len()) {
                let parent_idx = node_order.pop().unwrap();

                // TODO: Figure out how to really use spans
                let parent_name =
                    Ident::new(format!("node_{}", parent_idx).as_str(), Span::call_site());

                let parent_to_children_indices = match parent_to_children.get(&parent_idx) {
                    Some(children) => children,
                    None => continue,
                };

                if parent_to_children_indices.len() > 0 {
                    for child_idx in parent_to_children_indices.iter() {
                        let children =
                            Ident::new(format!("node_{}", child_idx).as_str(), Span::call_site());

                        let unreachable = quote_spanned!(Span::call_site() => {
                            unreachable!("Non-elements cannot have children");
                        });

                        let push_children = quote! {
                            if let Some(ref mut element_node) = #parent_name.as_velement_mut() {
                                element_node.children.extend(#children.into_iter());
                            } else {
                                #unreachable;
                            }
                        };

                        tokens.push(push_children);
                    }
                }
            }
        }

        // Create a virtual node tree
        let node = quote! {
            {
                #(#tokens)*
                // Root node is always named node_0
                node_0
            }
        };
        node
    }

    /// Add more tokens to our tokens that we'll eventually return to the compiler.
    fn push_tokens(&mut self, tokens: proc_macro2::TokenStream) {
        self.tokens.push(tokens);
    }

    /// Set the location of the most recent start tag's ending LineColumn
    fn set_most_recent_open_tag_end(&mut self, span: Span) {
        self.recent_span_locations.most_recent_open_tag_end = Some(span);
    }

    /// Set the location of the most recent start tag's ending LineColumn
    fn set_most_recent_block_start(&mut self, span: Span) {
        self.recent_span_locations.most_recent_block_start = Some(span);
    }

    /// Determine whether or not there is any space between the end of the first
    /// span and the beginning of the second span.
    ///
    /// There is space if they are on separate lines or if they have different columns.
    ///
    /// html! { <div>Hello</div> } <--- no space between end of div and Hello
    ///
    /// html! { <div> Hello</div> } <--- space between end of div and Hello
    fn separated_by_whitespace(&self, first_span: &Span, second_span: &Span) -> bool {
        if first_span.end().line != second_span.end().line {
            return true;
        }

        second_span.start().column - first_span.end().column > 0
    }

    /// Create a new identifier for a VirtualNode and increment our node_idx so that next
    /// time we call this our node will get a different name.
    fn new_virtual_node_ident(&mut self, span: Span) -> Ident {
        let node_name = format!("node_{}", self.current_node_idx);

        let node_ident = Ident::new(node_name.as_str(), span);

        // TODO: Increment before creating the new node, not after.
        // This way the current virtual node ident won't need to do strange subtraction
        self.current_node_idx += 1;

        node_ident
    }

    /// Get the Ident for the current (last created) virtual node, without incrementing
    /// the node index.
    fn current_virtual_node_ident(&self, span: Span) -> Ident {
        // TODO: Increment before creating the new node, not after.
        // This way the current virtual node ident won't need to do strange subtraction
        let node_name = format!("node_{}", self.current_node_idx - 1);

        Ident::new(node_name.as_str(), span)
    }

    /// Generate virtual node tokens for a statement that came from in between braces
    ///
    /// examples:
    ///
    /// html! { <div> { some_var_in_braces } </div>
    /// html! { <div> { some_other_variable } </div>
    fn push_iterable_nodes(&mut self, nodes: NodesToPush) {
        let node_idx = self.current_node_idx;

        match nodes {
            NodesToPush::Stmt(stmt) => {
                let node_ident = self.new_virtual_node_ident(stmt.span());

                self.push_tokens(quote! {
                    let mut #node_ident: IterableNodes = (#stmt).into();
                });
            }
            NodesToPush::TokenStream(stmt, tokens) => {
                let node_ident = self.new_virtual_node_ident(stmt.span());

                self.push_tokens(quote! {
                    let mut #node_ident: IterableNodes = #tokens.into();
                });
            }
        }

        let parent_idx = *&self.parent_stack[self.parent_stack.len() - 1].0;

        self.parent_to_children
            .get_mut(&parent_idx)
            .expect("Parent of these iterable nodes")
            .push(node_idx);
        self.node_order.push(node_idx);
    }
}

/// Keep track of the locations of different kinds of tokens that we encounter.
///
/// This helps us determine whether or not to insert space before or after text tokens
/// in cases such as:
///
/// ```ignore
/// html! { <div> { Hello World } </div>
/// html! { <div>{Hello World}</div>
/// ```
#[derive(Default)]
struct RecentSpanLocations {
    most_recent_open_tag_end: Option<Span>,
    most_recent_block_start: Option<Span>,
}

fn is_self_closing(tag: &str) -> bool {
    crate::validation::self_closing::is_self_closing(tag)
}

fn is_valid_tag(tag: &str) -> bool {
    crate::validation::valid_tags::is_valid_tag(tag)
}
