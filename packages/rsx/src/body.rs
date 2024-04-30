//! I'm so sorry this is so complicated. Here's my best to simplify and explain it:
//!
//! The `Callbody` is the contents of the rsx! macro - this contains all the information about every
//! node that rsx! directly knows about. For loops, if statements, etc.
//!
//! However, thre are multiple *templates* inside a callbody - due to how core clones templates and
//! just generally rationalize the concept of a template, nested bodies like for loops and if statements
//! and component children are all templates, contained within the same Callbody.
//!
//! This gets confusing fast since there's lots of IDs bouncing around.
//!
//! The IDs at play:
//!
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
//! We solve this by parsing the structure completely and then doing a second pass that fills in IDs
//! by walking the structure.
//!
//! This means you can't query the ID of any node "in a vacuum" - these are assigned once - but at
//! least theyre stable enough for the purposes of hotreloading

use crate::*;
use proc_macro2::TokenStream as TokenStream2;

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
    body: Vec<BodyNode>,
}

impl Parse for Callbody2 {
    /// Parse the nodes of the callbody as `Body`.
    fn parse(input: ParseStream) -> Result<Self> {
        // let body: Body = input.parse()?;
        let mut roots = Vec::new();

        while !input.is_empty() {
            let node = input.parse::<BodyNode>()?;

            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>();
            }

            roots.push(node);
        }

        todo!()
        // Ok(Self::new(body).unwrap())
    }
}

/// Our ToTokens impl here just defers to rendering a template out like any other `Body`.
/// This is because the parsing phase filled in all the additional metadata we need
impl ToTokens for Callbody2 {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        todo!()
        // self.body.to_tokens(tokens)
    }
}

/// A traverser for the entirety of the callbody
/// We'll create new ones on the fly but pass down some fields that need to be shared among the whole
/// callbody as we go
struct Traverser {
    cur_path: Vec<usize>,
    template_idx: usize,
    attr_idx: usize,
    dyn_node_idx: usize,
}

impl Traverser {
    /// Get an ID for the next dynamic attribute
    fn next_attr(&mut self) -> usize {
        self.attr_idx += 1;
        self.attr_idx - 1
    }

    /// Get an ID for the next dynamic node
    fn next_dyn_node(&mut self) -> usize {
        self.dyn_node_idx += 1;
        self.dyn_node_idx - 1
    }
}

// fn fill(
//     body: &mut Body,
//     nodes: &mut [BodyNode],
//     state: &mut Traverser,
// ) -> Option<Vec<TemplateNode>> {
//     let mut template_nodes = vec![];

//     for (idx, node) in nodes.iter_mut().enumerate() {
//         state.cur_path.push(idx);

//         // Create a new template node for us to stitch onto
//         let template_node = match node {
//             BodyNode::Text(_) => todo!(),
//             BodyNode::RawExpr(_) => todo!(),

//             // Handle the element by walking its attributes and then descending into its children
//             BodyNode::Element(el) => {
//                 // run through the attributes
//                 for attr in el.merged_attributes.iter() {
//                     todo!();
//                     // cur_body.attr_paths.push(value)
//                 }

//                 // run through the children
//                 let child_template_roots = fill(body, &mut el.children, state)?;

//                 todo!()
//                 // Build the template node
//                 // TemplateNode::Element {
//                 //     tag,
//                 //     namespace,
//                 //     attrs: intern(static_attr_array.into_boxed_slice()),
//                 //     children: intern(child_template_roots.as_slice()),
//                 // }
//             }

//             BodyNode::Component(_) => {
//                 // create a new traverser for the component children but using some of our traverser
//                 // as a seed
//                 let id = state.next_dyn_node();

//                 // And then just return a dynamic node
//                 TemplateNode::Dynamic { id }
//             }

//             BodyNode::ForLoop(_) => {
//                 // create a new traverser for the forloop contents using our traverser as a seed
//                 // as a seed
//                 let id = state.next_dyn_node();

//                 //
//                 TemplateNode::Dynamic { id }
//             }

//             BodyNode::IfChain(_) => {
//                 // create a new traverser for each of the ifchain branches using our traverser as
//                 // a seed
//                 let id = state.next_dyn_node();

//                 TemplateNode::Dynamic { id }
//             }
//         };
//         template_nodes.push(template_node);

//         _ = state.cur_path.pop();
//     }

//     Some(template_nodes)
// }
