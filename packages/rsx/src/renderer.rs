use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub struct TemplateRenderer<'a> {
    pub roots: &'a [BodyNode],
    location: Option<String>,
}

impl<'a> TemplateRenderer<'a> {
    /// Render the contents of the callbody out with a specific location
    pub fn as_tokens(roots: &'a [BodyNode], location: Option<String>) -> TokenStream2 {
        TemplateRenderer::render(Self { roots, location })
    }

    fn render(mut self) -> TokenStream2 {
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
        let roots = self.render_body_nodes(&mut context);
        let dynamic_nodes = self.render_dynamic_nodes(&context);

        let dyn_attr_printer = context
            .dynamic_attributes
            .iter()
            .map(|attrs| AttributeType::merge_quote(attrs));

        let node_paths = context.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        let attr_paths = context.attr_paths.iter().map(|it| quote!(&[#(#it),*]));

        quote! {
            static TEMPLATE: dioxus_core::Template = dioxus_core::Template {
                name: #name,
                roots: #roots,
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

    fn get_template_id_tokens(&self) -> TokenStream2 {
        let name = match self.location {
            Some(ref loc) => quote! { #loc },
            None => {
                // Get the root:column:id tag we'll use as the ID of the template
                let root_col = self.get_root_col_id();

                quote! {
                    concat!(
                        file!(),
                        ":",
                        line!(),
                        ":",
                        column!(),
                        ":",
                        #root_col
                    )
                }
            }
        };
        name
    }

    fn get_root_col_id(&self) -> String {
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
        root_col
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

    /// Render the dynamic nodes of the context out to the Boxed array
    ///
    /// This is basically the allocation step of the rsx!{} macro
    ///
    /// We do it like this in a linear block rather than recursion to be able to handle the location
    /// information of the nodes after they've been processed by the dynamic context.
    fn render_dynamic_nodes(&mut self, context: &DynamicContext<'a>) -> TokenStream2 {
        let mut roots = vec![];

        for &node in context.dynamic_nodes.iter() {
            let root = match node {
                BodyNode::Element(_) => {
                    unimplemented!("Elements are statically created in the template")
                }

                // Text is simple, just write it out
                BodyNode::Text(txt) => quote! {
                    dioxus_core::DynamicNode::Text(dioxus_core::VText::new(#txt.to_string()))
                },

                // Expressons too
                BodyNode::RawExpr(exp) => quote! {
                    {
                        let ___nodes = (#exp).into_dyn_node();
                        ___nodes
                    }
                },

                // todo:
                //
                // Component children should also participate in hotreloading
                // This is a *little* hard since components might not be able to take children in the
                // first place. I'm sure there's a hacky way to allow this... but it's not quite as
                // straightforward as a for loop.
                //
                // It might involve always generating a `children` field on the component and always
                // populating it with an empty template. This might lose the typesafety of whether
                // or not a component can even accept children - essentially allowing childrne in
                // every component - so it'd be breaking - but it would/could work.
                BodyNode::Component(comp) => comp.render(None),

                BodyNode::ForLoop(exp) => {
                    let ForLoop {
                        pat, expr, body, ..
                    } = exp;

                    let renderer = TemplateRenderer::as_tokens(body, None);

                    // Signals expose an issue with temporary lifetimes
                    // We need to directly render out the nodes first to collapse their lifetime to <'a>
                    // And then we can return them into the dyn loop
                    quote! {
                        {
                            let ___nodes = (#expr).into_iter().map(|#pat| { #renderer }).into_dyn_node();
                            ___nodes
                        }
                    }
                }

                BodyNode::IfChain(chain) => {
                    let mut body = TokenStream2::new();
                    let mut terminated = false;

                    let mut elif = Some(chain);

                    while let Some(chain) = elif {
                        let IfChain {
                            if_token,
                            cond,
                            then_branch,
                            else_if_branch,
                            else_branch,
                        } = chain;

                        let renderer = TemplateRenderer::as_tokens(&then_branch, None);

                        body.append_all(quote! { #if_token #cond { Some({#renderer}) } });

                        if let Some(next) = else_if_branch {
                            body.append_all(quote! { else });
                            elif = Some(next);
                        } else if let Some(else_branch) = else_branch {
                            let renderer = TemplateRenderer::as_tokens(&else_branch, None);
                            body.append_all(quote! { else { Some({#renderer}) } });
                            terminated = true;
                            break;
                        } else {
                            elif = None;
                        }
                    }

                    if !terminated {
                        body.append_all(quote! {
                            else { None }
                        });
                    }

                    quote! {
                        {
                            let ___nodes = (#body).into_dyn_node();
                            ___nodes
                        }
                    }
                }
            };

            roots.push(root);
        }

        quote! {
            Box::new([ #( #roots ),* ])
        }
    }
}
