//! I'm so sorry this is so complicated. Here's my best to simplify and explain it:
//!
//! The `Callbody` is the contents of the rsx! macro - this contains all the information about every
//! node that rsx! directly knows about. For loops, if statements, etc.
//!
//! However, there are multiple *templates* inside a callbody - due to how core clones templates and
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
//!         ..props,                 <--- spreads are not reloadable
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

use self::location::DynIdx;
use crate::innerlude::Attribute;
use crate::*;
use proc_macro2::TokenStream as TokenStream2;

#[cfg(feature = "hot_reload")]
use dioxus_core::prelude::Template;

type NodePath = Vec<u8>;
type AttributePath = Vec<u8>;

/// A set of nodes in a template position
///
/// this could be:
/// - The root of a callbody
/// - The children of a component
/// - The children of a for loop
/// - The children of an if chain
///
/// The TemplateBody when needs to be parsed into a surrounding `Body` to be correctly re-indexed
/// By default every body has a `0` default index
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct TemplateBody {
    pub roots: Vec<BodyNode>,
    pub template_idx: DynIdx,
    pub node_paths: Vec<NodePath>,
    pub attr_paths: Vec<(AttributePath, usize)>,
    pub diagnostics: Diagnostics,
    current_path: Vec<u8>,
}

impl Parse for TemplateBody {
    /// Parse the nodes of the callbody as `Body`.
    fn parse(input: ParseStream) -> Result<Self> {
        let children = RsxBlock::parse_children(input)?;
        let mut myself = Self::new(children.children);
        myself
            .diagnostics
            .extend(children.diagnostics.into_diagnostics());
        Ok(myself)
    }
}

/// Our ToTokens impl here just defers to rendering a template out like any other `Body`.
/// This is because the parsing phase filled in all the additional metadata we need
impl ToTokens for TemplateBody {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // If there are no roots, this is an empty template, so just return None
        if self.roots.is_empty() {
            return tokens.append_all(quote! { dioxus_core::VNode::empty() });
        }

        // If we have an implicit key, then we need to write its tokens
        let key_tokens = match self.implicit_key() {
            Some(tok) => quote! { Some( #tok.to_string() ) },
            None => quote! { None },
        };

        let TemplateBody { roots, .. } = self;
        let roots = roots.iter().map(|node| match node {
            BodyNode::Element(el) => quote! { #el },
            BodyNode::Text(text) if text.is_static() => {
                let text = text.input.to_static().unwrap();
                quote! { dioxus_core::TemplateNode::Text { text: #text } }
            }
            BodyNode::Text(text) => {
                let id = text.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::ForLoop(floop) => {
                let id = floop.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::RawExpr(exp) => {
                let id = exp.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::Component(exp) => {
                let id = exp.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::IfChain(exp) => {
                let id = exp.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
        });

        // Print paths is easy - just print the paths
        let node_paths = self.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        let attr_paths = self.attr_paths.iter().map(|(it, _)| quote!(&[#(#it),*]));

        // For printing dynamic nodes, we rely on the ToTokens impl
        // Elements have a weird ToTokens - they actually are the entrypoint for Template creation
        let dynamic_nodes = self.node_paths.iter().map(|path| {
            let node = self.get_dyn_node(path);
            quote::quote! { #node }
        });

        // We could add a ToTokens for Attribute but since we use that for both components and elements
        // They actually need to be different, so we just localize that here
        let dyn_attr_printer = self
            .attr_paths
            .iter()
            .map(|(path, idx)| self.get_dyn_attr(path, *idx).rendered_as_dynamic_attr());

        let index = self.template_idx.get();

        let diagnostics = &self.diagnostics;

        tokens.append_all(quote! {
            dioxus_core::Element::Ok({
                #[doc(hidden)] // vscode please stop showing these in symbol search
                static ___TEMPLATE: dioxus_core::Template = dioxus_core::Template {
                    name: {
                        const PATH: &str = dioxus_core::const_format::str_replace!(file!(), "\\\\", "/");
                        const NORMAL: &str = dioxus_core::const_format::str_replace!(PATH, '\\', "/");
                        dioxus_core::const_format::concatcp!(NORMAL, ':', line!(), ':', column!(), ':', #index)
                    },
                    roots: &[ #( #roots ),* ],
                    node_paths: &[ #( #node_paths ),* ],
                    attr_paths: &[ #( #attr_paths ),* ],
                };

                #diagnostics

                {
                    // NOTE: Allocating a temporary is important to make reads within rsx drop before the value is returned
                    #[allow(clippy::let_and_return)]
                    let __vnodes = dioxus_core::VNode::new(
                        #key_tokens,
                        ___TEMPLATE,
                        Box::new([ #( #dynamic_nodes),* ]),
                        Box::new([ #( #dyn_attr_printer ),* ]),
                    );
                    __vnodes
                }

            })
        });
    }
}

impl TemplateBody {
    /// Create a new TemplateBody from a set of nodes
    ///
    /// This will fill in all the necessary path information for the nodes in the template and will
    /// overwrite data like dynamic indexes.
    pub fn new(nodes: Vec<BodyNode>) -> Self {
        let mut body = Self {
            roots: vec![],
            template_idx: DynIdx::default(),
            node_paths: Vec::new(),
            attr_paths: Vec::new(),
            current_path: Vec::new(),
            diagnostics: Diagnostics::new(),
        };

        // Assign paths to all nodes in the template
        body.assign_paths_inner(&nodes);

        // And then save the roots
        body.roots = nodes;

        body
    }

    /// Cascade down path information into the children of this template
    ///
    /// This provides the necessary path and index information for the children of this template
    /// so that they can render out their dynamic nodes correctly. Also does plumbing for things like
    /// hotreloaded literals which need to be tracked on a per-template basis.
    ///
    /// This can only operate with knowledge of this template, not the surrounding callbody. Things like
    /// wiring of ifmt literals need to be done at the callbody level since those final IDs need to
    /// be unique to the entire app.
    fn assign_paths_inner(&mut self, nodes: &[BodyNode]) {
        for (idx, node) in nodes.iter().enumerate() {
            self.current_path.push(idx as u8);
            match node {
                // Just descend into elements - they're not dynamic
                BodyNode::Element(el) => {
                    for (idx, attr) in el.merged_attributes.iter().enumerate() {
                        if !attr.is_static_str_literal() {
                            attr.dyn_idx.set(self.attr_paths.len());
                            self.attr_paths.push((self.current_path.clone(), idx));
                        }
                    }

                    self.assign_paths_inner(&el.children)
                }

                // Text nodes are dynamic if they contain dynamic segments
                BodyNode::Text(txt) => {
                    if !txt.is_static() {
                        self.assign_path_to(node);
                    }
                }

                // Raw exprs are always dynamic
                BodyNode::RawExpr(_)
                | BodyNode::ForLoop(_)
                | BodyNode::Component(_)
                | BodyNode::IfChain(_) => self.assign_path_to(node),
            };
            self.current_path.pop();
        }
    }

    /// Assign a path to a node and give it its dynamic index
    /// This simplifies the ToTokens implementation for the macro to be a little less centralized
    fn assign_path_to(&mut self, node: &BodyNode) {
        // Assign the TemplateNode::Dynamic index to the node
        node.set_dyn_idx(self.node_paths.len());

        // And then save the current path as the corresponding path
        self.node_paths.push(self.current_path.clone());
    }

    /// Create a new template from this TemplateBody
    ///
    /// Note that this will leak memory! We explicitly call `leak` on the vecs to match the format of
    /// the `Template` struct.
    #[cfg(feature = "hot_reload")]
    pub fn to_template<Ctx: HotReloadingContext>(&self) -> Template {
        self.to_template_with_custom_paths::<Ctx>(
            "placeholder",
            self.node_paths.clone(),
            self.attr_paths.clone().into_iter().map(|v| v.0).collect(),
        )
    }

    #[cfg(feature = "hot_reload")]
    pub fn to_template_with_custom_paths<Ctx: HotReloadingContext>(
        &self,
        location: &'static str,
        node_paths: Vec<NodePath>,
        attr_paths: Vec<AttributePath>,
    ) -> Template {
        let roots = self
            .roots
            .iter()
            .map(|node| node.to_template_node::<Ctx>())
            .collect::<Vec<_>>();

        Template {
            name: location,
            roots: intern(roots.as_slice()),
            node_paths: intern(
                node_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            attr_paths: intern(
                attr_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    fn implicit_key(&self) -> Option<IfmtInput> {
        match self.roots.first() {
            Some(BodyNode::Element(el)) if self.roots.len() == 1 => el.key().cloned(),
            Some(BodyNode::Component(comp)) if self.roots.len() == 1 => {
                comp.get_key().and_then(|f| f.ifmt().cloned())
            }
            _ => None,
        }
    }

    pub fn get_dyn_node(&self, path: &[u8]) -> &BodyNode {
        let mut node = self.roots.get(path[0] as usize).unwrap();
        for idx in path.iter().skip(1) {
            node = node.element_children().get(*idx as usize).unwrap();
        }
        node
    }

    pub fn get_dyn_attr(&self, path: &AttributePath, idx: usize) -> &Attribute {
        match self.get_dyn_node(path) {
            BodyNode::Element(el) => &el.merged_attributes[idx],
            _ => unreachable!(),
        }
    }

    pub fn dynamic_attributes(&self) -> impl DoubleEndedIterator<Item = &Attribute> {
        self.attr_paths
            .iter()
            .map(|(path, idx)| self.get_dyn_attr(path, *idx))
    }

    pub fn dynamic_nodes(&self) -> impl DoubleEndedIterator<Item = &BodyNode> {
        self.node_paths.iter().map(|path| self.get_dyn_node(path))
    }
}
