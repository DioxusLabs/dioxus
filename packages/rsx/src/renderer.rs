use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};

pub struct TemplateRenderer<'a> {
    pub roots: &'a [BodyNode],
    location: Option<String>,
}

impl<'a> TemplateRenderer<'a> {
    /// Create a new template renderer, filling in the templates
    pub fn new(roots: &'a [BodyNode], location: Option<String>) -> Self {
        Self { roots, location }
    }
}

impl<'a> ToTokens for TemplateRenderer<'a> {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let mut context = DynamicContext::default();

        let key = match self.roots.first() {
            Some(BodyNode::Element(el)) if self.roots.len() == 1 => el.key.clone(),
            Some(BodyNode::Component(comp)) if self.roots.len() == 1 => comp.key().cloned(),
            _ => None,
        };

        let key_tokens = match key {
            Some(tok) => quote! { Some( #tok.to_string() ) },
            None => quote! { None },
        };

        let root_col = match self.roots.first() {
            Some(first_root) => {
                let first_root_span = format!("{:?}", first_root.span());
                first_root_span
                    .rsplit_once("..")
                    .and_then(|(_, after)| after.split_once(')').map(|(before, _)| before))
                    .unwrap_or_default()
                    .to_string()
            }
            _ => "0".to_string(),
        };

        let root_printer = self.roots.iter().enumerate().map(|(idx, root)| {
            context.current_path.push(idx as u8);
            let out = context.render_static_node(root);
            context.current_path.pop();
            out
        });

        let name = match self.location {
            Some(ref loc) => quote! { #loc },
            None => quote! {
                concat!(
                    file!(),
                    ":",
                    line!(),
                    ":",
                    column!(),
                    ":",
                    #root_col
                )
            },
        };

        // Render and release the mutable borrow on context
        let roots = quote! { #( #root_printer ),* };
        let node_printer = &context.dynamic_nodes;
        let dyn_attr_printer = context
            .dynamic_attributes
            .iter()
            .map(|attrs| AttributeType::merge_quote(attrs));

        let node_paths = context.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        let attr_paths = context.attr_paths.iter().map(|it| quote!(&[#(#it),*]));

        out_tokens.append_all(quote! {
            static TEMPLATE: dioxus_core::Template = dioxus_core::Template {
                name: #name,
                roots: &[ #roots ],
                node_paths: &[ #(#node_paths),* ],
                attr_paths: &[ #(#attr_paths),* ],
            };

            // spit out all the sub-templates
            {
                // NOTE: Allocating a temporary is important to make reads within rsx drop before the value is returned
                let __vnodes = dioxus_core::VNode::new(
                    #key_tokens,
                    TEMPLATE,
                    Box::new([ #( #node_printer),* ]),
                    Box::new([ #(#dyn_attr_printer),* ]),
                );
                __vnodes
            }
        });
    }
}
