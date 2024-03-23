use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

pub struct TemplateRenderer<'a> {
    pub roots: &'a [BodyNode],
    pub location: Option<String>,
}

impl<'a> TemplateRenderer<'a> {
    #[cfg(feature = "hot_reload")]
    pub fn update_template<Ctx: HotReloadingContext>(
        &mut self,
        previous_call: Option<CallBody>,
        location: &'static str,
    ) -> Option<Template> {
        use mapping::DynamicMapping;

        use crate::context::DynamicContext;

        let mut mapping = previous_call.map(|call| DynamicMapping::from(call.roots));

        let mut context = DynamicContext::default();

        let mut roots = Vec::new();

        for (idx, root) in self.roots.iter().enumerate() {
            context.current_path.push(idx as u8);
            roots.push(context.update_node::<Ctx>(root, &mut mapping)?);
            context.current_path.pop();
        }

        Some(Template {
            name: location,
            roots: intern(roots.as_slice()),
            node_paths: intern(
                context
                    .node_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            attr_paths: intern(
                context
                    .attr_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        })
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
