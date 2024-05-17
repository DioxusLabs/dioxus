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
//! - The id of the template itself so we can find it and apply it to the dom.
//!   This is challenging since all calls to file/line/col/id are relative to the macro invocation,
//!   so they will have to share the same base ID and we need to give each template a new ID.
//!   The id of the template will be something like file!():line!():col!():ID where ID increases for
//!   each nested template.
//!
//! - The IDs of dynamic nodes relative to the template they live in. This is somewhat easy to track
//!   but needs to happen on a per-template basis.
//!
//! - The unique ID of a hotreloadable literal (like ifmt or integers or strings, etc). This ID is
//!   unique to the Callbody, not necessarily the template it lives in. This is similar to the
//!   template ID
//!
//! We solve this by parsing the structure completely and then doing a second pass that fills in IDs
//! by walking the structure.
//!
//! This means you can't query the ID of any node "in a vacuum" - these are assigned once - but at
//! least they're stable enough for the purposes of hotreloading
//!
//! The plumbing for hotreloadable literals could be template relative... ie "file:line:col:template:idx"
//! That would be ideal if we could determine the the idx only relative to the template
//!
//! ```rust, ignore
//! rsx! {
//!     div {
//!         class: "hello",
//!         id: "node-{node_id}",    <--- hotreloadable with ID 0
//!
//!         "Hello, world!           <--- not tracked but reloadable since it's just a string
//!
//!         for item in 0..10 {      <--- both 0 and 10 are technically reloadable...
//!             div { "cool-{item}" }     <--- the ifmt here is also reloadable
//!         }
//!
//!         Link {
//!             to: "/home", <-- hotreloadable since its a component prop
//!             class: "link {is_ready}", <-- hotreloadable since its a formatted string as a prop
//!             "Home" <-- hotreloadable since its a component child (via template)
//!         }
//!     }
//! }
//! ```

use crate::*;
use proc_macro2::TokenStream as TokenStream2;

pub struct Callbody2 {
    nodes: Vec<BodyNode>,
}

impl Parse for Callbody2 {
    /// Parse the nodes of the callbody as `Body`.
    fn parse(input: ParseStream) -> Result<Self> {
        let mut nodes = Vec::new();

        while !input.is_empty() {
            let node = input.parse::<BodyNode>()?;

            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>();
            }

            nodes.push(node);
        }

        // Now walk the body with this visitor struct thing so we can fill in the IDs
        // We do this here since we have &mut access to the body, making it easier/cheaper to represent
        // location data.
        //
        // Having location data on nodes also makes the ToTokens easier to implement since it stays
        // roughly localized
        LocationVisitor::fill(&mut nodes, 0);

        Ok(Self { nodes })
    }
}

/// Our ToTokens impl here just defers to rendering a template out like any other `Body`.
/// This is because the parsing phase filled in all the additional metadata we need
impl ToTokens for Callbody2 {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // DynamicContext::from_body(&self.nodes);
        todo!()
        // self.body.to_tokens(tokens)
    }
}

/// A traverser to install location data for a parsed body
#[derive(Default)]
struct LocationVisitor {
    cur_path: Vec<usize>,
    template_idx: usize,
    attr_idx: usize,
    dyn_node_idx: usize,
}

impl LocationVisitor {
    fn fill(roots: &mut [BodyNode], template_idx: usize) {
        let mut s = Self {
            cur_path: vec![],
            attr_idx: 0,
            dyn_node_idx: 0,
            template_idx,
        };

        for node in roots.iter_mut() {
            s.fill_node(node);
        }
    }

    fn fill_node(&mut self, node: &mut BodyNode) {
        match node {
            // Fills with simple tracking
            BodyNode::Element(_) => todo!(),
            BodyNode::RawExpr(exp) => {}
            BodyNode::Text(f) if !f.is_static() => {}
            BodyNode::Text(f) => {}

            BodyNode::Component(_) => todo!(),
            BodyNode::ForLoop(_) => todo!(),
            BodyNode::IfChain(_) => todo!(),
        }
    }

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
