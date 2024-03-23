use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};

pub struct TemplateRenderer<'a> {
    pub roots: &'a [BodyNode],
    location: Option<String>,
}

impl<'a> TemplateRenderer<'a> {
    /// Render the contents of the callbody out with a specific location
    pub fn as_tokens(roots: &'a [BodyNode], location: Option<String>) -> TokenStream2 {
        let _self = Self::new(roots, location);
        _self.render()
    }

    /// Create a new template renderer, filling in the templates
    fn new(roots: &'a [BodyNode], location: Option<String>) -> Self {
        Self { roots, location }
    }

    fn render(mut self) -> TokenStream2 {
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
        let dynamic_nodes = self.render_dynamic_nodes(context.dynamic_nodes.as_slice());
        let dyn_attr_printer = context
            .dynamic_attributes
            .iter()
            .map(|attrs| AttributeType::merge_quote(attrs));

        let node_paths = context.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        let attr_paths = context.attr_paths.iter().map(|it| quote!(&[#(#it),*]));

        quote! {
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
                    #dynamic_nodes,
                    Box::new([ #(#dyn_attr_printer),* ]),
                );
                __vnodes
            }
        }
    }

    fn render_dynamic_nodes(&mut self, nodes: &[&BodyNode]) -> TokenStream2 {
        // Box::new([ #( #dynamic_nodes ),* ])
        todo!()
    }
}

// impl<'a> ToTokens for TemplateRenderer<'a> {
//     fn to_tokens(&self, out_tokens: &mut TokenStream2) {

//     }
// }

// impl ToTokens for BodyNode {
//     fn to_tokens(&self, tokens: &mut TokenStream2) {
//         match &self {
//             BodyNode::Element(_) => {
//                 unimplemented!("Elements are statically created in the template")
//             }
//             BodyNode::Component(comp) => comp.to_tokens(tokens),
//             BodyNode::Text(txt) => tokens.append_all(quote! {
//                 dioxus_core::DynamicNode::Text(dioxus_core::VText::new(#txt.to_string()))
//             }),
//             BodyNode::RawExpr(exp) => tokens.append_all(quote! {
//                 {
//                     let ___nodes = (#exp).into_dyn_node();
//                     ___nodes
//                 }
//             }),
//             BodyNode::ForLoop(exp) => {
//                 let ForLoop {
//                     pat, expr, body, ..
//                 } = exp;

//                 let renderer = TemplateRenderer::new(body, None);

//                 // Signals expose an issue with temporary lifetimes
//                 // We need to directly render out the nodes first to collapse their lifetime to <'a>
//                 // And then we can return them into the dyn loop
//                 tokens.append_all(quote! {
//                     {
//                         let ___nodes = (#expr).into_iter().map(|#pat| { #renderer }).into_dyn_node();
//                         ___nodes
//                     }
//                 })
//             }
//             BodyNode::IfChain(chain) => {
//                 let mut body = TokenStream2::new();
//                 let mut terminated = false;

//                 let mut elif = Some(chain);

//                 while let Some(chain) = elif {
//                     let IfChain {
//                         if_token,
//                         cond,
//                         then_branch,
//                         else_if_branch,
//                         else_branch,
//                     } = chain;

//                     let mut renderer = TemplateRenderer::new(&then_branch, None);

//                     body.append_all(quote! { #if_token #cond { Some({#renderer}) } });

//                     if let Some(next) = else_if_branch {
//                         body.append_all(quote! { else });
//                         elif = Some(next);
//                     } else if let Some(else_branch) = else_branch {
//                         let mut renderer = TemplateRenderer::new(&else_branch, None);
//                         body.append_all(quote! { else { Some({#renderer}) } });
//                         terminated = true;
//                         break;
//                     } else {
//                         elif = None;
//                     }
//                 }

//                 if !terminated {
//                     body.append_all(quote! {
//                         else { None }
//                     });
//                 }

//                 tokens.append_all(quote! {
//                     {
//                         let ___nodes = (#body).into_dyn_node();
//                         ___nodes
//                     }
//                 });
//             }
//         }
//     }
// }
