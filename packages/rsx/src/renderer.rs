use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub struct TemplateRenderer<'a> {
    pub roots: &'a [BodyNode],
    parent_idx: usize,
    location: Option<TokenStream2>,
}

impl<'a> TemplateRenderer<'a> {
    /// Render the contents of the callbody out with a specific location
    ///
    /// This will cascade location information down the tree if it already hasn't been set
    pub fn as_tokens(roots: &'a [BodyNode], location: Option<String>) -> TokenStream2 {
        let location = location.map(|loc| quote! { #loc });

        TemplateRenderer::render(Self {
            roots,
            location,
            parent_idx: 0,
        })
    }

    pub fn as_tokens_with_idx(roots: &'a [BodyNode], idx: usize) -> TokenStream2 {
        let location = Some(quote! {
            concat!(
                file!(),
                ":",
                line!(),
                ":",
                column!(),
                ":",
                #idx
            )
        });

        TemplateRenderer::render(Self {
            roots,
            location,
            parent_idx: idx,
        })
    }

    fn render(mut self) -> TokenStream2 {
        // If there are no roots, this is an empty template, so just return None
        if self.roots.is_empty() {
            return quote! { Option::<dioxus_core::VNode>::None };
        }

        // Create a new dynamic context that tracks the state of all the dynamic nodes
        // We have no old template, to seed it with, so it'll just be used for rendering
        let mut context = DynamicContext::default();

        // If we have an implicit key, then we need to write its tokens
        let key_tokens = match self.implicit_key() {
            Some(tok) => quote! { Some( #tok.to_string() ) },
            None => quote! { None },
        };

        // Get the tokens we'll use as the ID of the template
        // This follows the file:line:column:id format
        let name = self.get_template_id_tokens();

        // Render the static nodes, generating the mapping of dynamic
        // This will modify the bodynodes, filling in location information for any sub templates
        let roots = self.render_body_nodes(&mut context);

        // run through the dynamic nodes and set their location based on the idx of that node
        for (idx, node) in context.dynamic_nodes.iter_mut().enumerate() {
            // We use +1 since :0 is the base of the template
            node.set_location_idx(idx + 1);
        }

        let dynamic_nodes = &context.dynamic_nodes;
        let dyn_attr_printer = context
            .dynamic_attributes
            .iter()
            .map(|attrs| AttributeType::merge_quote(attrs));

        let node_paths = context.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        let attr_paths = context.attr_paths.iter().map(|it| quote!(&[#(#it),*]));

        let vnode = quote! {
            static TEMPLATE: dioxus_core::Template = dioxus_core::Template {
                name: #name,
                roots: #roots,
                node_paths: &[ #(#node_paths),* ],
                attr_paths: &[ #(#attr_paths),* ],
            };

            {
                // NOTE: Allocating a temporary is important to make reads within rsx drop before the value is returned
                let __vnodes = dioxus_core::VNode::new(
                    #key_tokens,
                    TEMPLATE,
                    Box::new([ #( #dynamic_nodes),* ]),
                    Box::new([ #(#dyn_attr_printer),* ]),
                );
                __vnodes
            }
        };

        quote! { Some({ #vnode }) }
    }

    fn get_template_id_tokens(&self) -> TokenStream2 {
        match self.location {
            Some(ref loc) => loc.clone(),
            None => {
                // // Get the root:column:id tag we'll use as the ID of the template
                // let root_col = self.get_root_col_id();

                quote! {
                    concat!(
                        file!(),
                        ":",
                        line!(),
                        ":",
                        column!(),
                        ":",
                        "0"
                    )
                }
            }
        }
    }

    fn implicit_key(&self) -> Option<IfmtInput> {
        let key = match self.roots.first() {
            Some(BodyNode::Element(el)) if self.roots.len() == 1 => el.key.clone(),
            Some(BodyNode::Component(comp)) if self.roots.len() == 1 => comp.key().cloned(),
            _ => None,
        };
        key
    }

    /// Render a list of BodyNodes as a static array (&[...])
    pub fn render_body_nodes(&mut self, context: &mut DynamicContext<'a>) -> TokenStream2 {
        let root_printer = self
            .roots
            .iter()
            .enumerate()
            .map(|(idx, root)| context.render_children_nodes(idx, root));

        // Render the static nodes, generating the mapping of dynamic
        quote! {
            &[ #( #root_printer ),* ]
        }
    }
}
