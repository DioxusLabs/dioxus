//! I'm so sorry this is so complicated. Here's my best to simplify and explain it:
//!
//! The `Callbody` is the contents of the rsx! macro - this contains all the information about every
//! node that rsx! directly knows about. For loops, if statements, etc.
//!
//! However, thre are multiple *templates* inside a callbody - due to how core clones templates and
//! just generally rationalize the concept of a template, nested bodies like for loops and if statements
//! and component childrne are all templates, contained within the same Callbody.
//!
//! This gets confusing fast since there's lots of IDs bouncing around.
//!
//! The IDs at play:
//! - The id of the template itself so we can find it and apply it to the dom.
//!   This is challenging since all calls to file/line/col/id are relative to the macro invocation,
//!   so they will have to share the same base ID and we need to give each template a new ID.
//!
//! - The IDs of dynamic nodes relative to the template they live in. This is somewhat easy to track
//!   but needs to happen on a per-template basis.
//!
//! - The unique ID of a hotreloadable literal. This ID is unique to the Callbody, not necessarily the
//!   template it lives in.
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!

use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

type NodePath = Vec<u8>;
type AttributePath = Vec<u8>;

/// A new implementation of the callbody that stores all the dynamic mapping on itself
///
/// This makes it easier to have repeatable location data between hotreloading and typical rendering
/// It also deduplicates a lot of the tracking logic.
///
/// When the callbody is built, we also generate the associated template nodes... you should have
/// *everything* you need from this struct to render and hotreload.
///
/// It's a lot of work, but it's all saved via caching.
pub struct Callbody2 {
    /// The list of all the templates found in this rsx! call in the order they were found via dfs
    /// This guarantees that the first template is the root template and this rendering order makes sense
    ///
    /// IE
    /// rsx! {
    ///     /* ...0 */
    ///    for { /* ...1  */ }
    ///    Component { /* ...2 */ }
    ///    if {
    ///       /* ...3 */
    ///       for { /* ...4 */ }
    ///    }
    /// }
    templates: Vec<TemplateContext>,
}

/// A single template with its internal representation
struct TemplateContext {
    id: usize,
    roots: Vec<BodyNode>,
    template: Vec<TemplateNode>,
    node_paths: Vec<NodePath>,
    attr_paths: Vec<AttributePath>,
}

impl Callbody2 {
    pub fn new(roots: Vec<BodyNode>) -> Option<Self> {
        let mut s = Self {
            templates: vec![], // roots,
                               // current_path: vec![],
                               // node_paths: vec![],
                               // attr_paths: vec![],
                               // template: vec![],
                               // dynamic_idx: 0,
        };

        // s.template = s.fill()?;

        Some(s)
    }

    /// Fill the dynamic context with the callbody
    ///
    /// This will cascade location information down the tree if it already hasn't been set
    ///
    fn fill(&mut self) -> Option<Vec<TemplateNode>> {
        todo!()
    }

    /// Produce a dioxus template ID for this callbody
    ///
    /// This follows the file:line:column:id format
    ///
    fn location_with_idx(&self, idx: usize) -> TokenStream2 {
        quote! {
            concat!( file!(), ":", line!(), ":", column!(), ":", #idx )
        }
    }

    // /// Get the implicit key of this set of nodes
    // ///
    // /// todo: throw some warnings or something if there's an implicit key but it's defined incorrectly
    // fn implicit_key(&self) -> Option<IfmtInput> {
    //     match self.roots.first() {
    //         Some(BodyNode::Element(el)) if self.roots.len() == 1 => el.key.clone(),
    //         Some(BodyNode::Component(comp)) if self.roots.len() == 1 => comp.key().cloned(),
    //         _ => None,
    //     }
    // }

    // /// Find the current node by traversing the children arrays
    // /// This is not great performance wise (cache locality and all that), but it's not a big deal
    // /// since rendering is not that common
    // fn find_dynamic_node(&self, idx: usize) -> &BodyNode {
    //     let path = self.node_paths[idx].clone();
    //     let mut cur_node = &self.roots[path[0] as usize];
    //     for id in path.iter().skip(1) {
    //         cur_node = &cur_node.children()[*id as usize];
    //     }
    //     cur_node
    // }

    // /// Convert the list of nodepaths into actual calls to create the dynamic nodes.
    // ///
    // /// This is stuff like converting `Component {}` to a vcomponent::new call
    // fn render_dynamic_node(&self, idx: usize, tokens: &mut TokenStream2) {
    //     let node = self.find_dynamic_node(idx);
    //     todo!()
    // }

    // /// Render dynamic attributes
    // fn render_dynamic_attributes(&self, idx: usize, tokens: &mut TokenStream2) {
    //     let node = self.find_dynamic_node(idx);
    //     todo!()
    // }

    /// Render this callbody to a tokenstream
    ///
    /// todo: change the syntax here so we're rendering *into* the tokenstream instead of returning it
    ///       this has the benefit of not having to allocate a new token stream
    fn render(&self) -> TokenStream2 {
        // If there are no roots, this is an empty template, so just return None
        if self.roots.is_empty() {
            return quote! { Option::<dioxus_core::VNode>::None };
        }

        // If we have an implicit key, then we need to write its tokens
        let key_tokens = match self.implicit_key() {
            Some(tok) => quote! { Some( #tok.to_string() ) },
            None => quote! { None },
        };

        // Get the tokens we'll use as the ID of the template
        let name = self.location_with_idx(0);

        todo!()
        // // Render the static nodes, generating the mapping of dynamic
        // // This will modify the bodynodes, filling in location information for any sub templates
        // let roots = self.render_body_nodes(&mut context);

        // // run through the dynamic nodes and set their location based on the idx of that node
        // for (idx, node) in self.dynamic_nodes.iter_mut().enumerate() {
        //     // We use +1 since :0 is the base of the template
        //     node.set_location_idx(idx + 1);
        // }

        // let dynamic_nodes = &self.dynamic_nodes;
        // let dyn_attr_printer = self
        //     .dynamic_attributes
        //     .iter()
        //     .map(|attrs| AttributeType::merge_quote(attrs));

        // let node_paths = self.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        // let attr_paths = self.attr_paths.iter().map(|it| quote!(&[#(#it),*]));

        // let vnode = quote! {
        //     static TEMPLATE: dioxus_core::Template = dioxus_core::Template {
        //         name: #name,
        //         roots: #roots,
        //         node_paths: &[ #(#node_paths),* ],
        //         attr_paths: &[ #(#attr_paths),* ],
        //     };

        //     {
        //         // NOTE: Allocating a temporary is important to make reads within rsx drop before the value is returned
        //         let __vnodes = dioxus_core::VNode::new(
        //             #key_tokens,
        //             TEMPLATE,
        //             Box::new([ #( #dynamic_nodes),* ]),
        //             Box::new([ #(#dyn_attr_printer),* ]),
        //         );
        //         __vnodes
        //     }
        // };

        // quote! { Some({ #vnode }) }
    }
}

impl Parse for Callbody2 {
    /// Parse the nodes of the callbody and then fill in all the location information we need to generate
    /// template IDs and dynamic node IDs
    fn parse(input: ParseStream) -> Result<Self> {
        let mut roots = Vec::new();

        while !input.is_empty() {
            let node = input.parse::<BodyNode>()?;

            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>();
            }

            roots.push(node);
        }

        Ok(Self::new(roots).unwrap())
    }
}

impl ToTokens for Callbody2 {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.render().to_tokens(tokens);
    }
}
