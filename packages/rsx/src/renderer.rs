use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub struct TemplateRenderer<'a> {
    pub roots: &'a [BodyNode],
}

impl<'a> TemplateRenderer<'a> {
    /// Render the contents of the callbody out with a specific location
    ///
    /// This will cascade location information down the tree if it already hasn't been set
    pub fn as_tokens(roots: &'a [BodyNode], location: Option<String>) -> TokenStream2 {
        let location = location.map(|loc| quote! { #loc });

        TemplateRenderer::render(Self {
            roots,
            // location,
            // parent_idx: 0,
        })
    }

    fn render(mut self) -> TokenStream2 {
        // If there are no roots, this is an empty template, so just return None
        if self.roots.is_empty() {
            return quote! { Option::<dioxus_core::VNode>::None };
        }

        // // Create a new dynamic context that tracks the state of all the dynamic nodes
        // // We have no old template, to seed it with, so it'll just be used for rendering
        // let mut context = DynamicContext::default();

        // // If we have an implicit key, then we need to write its tokens
        // let key_tokens = match self.implicit_key() {
        //     Some(tok) => quote! { Some( #tok.to_string() ) },
        //     None => quote! { None },
        // };

        // // Get the tokens we'll use as the ID of the template
        // // This follows the file:line:column:id format
        // let name = self.get_template_id_tokens();

        // // Render the static nodes, generating the mapping of dynamic
        // // This will modify the bodynodes, filling in location information for any sub templates
        // let roots = self.render_body_nodes(&mut context);

        // // run through the dynamic nodes and set their location based on the idx of that node
        // for (idx, node) in context.dynamic_nodes.iter_mut().enumerate() {
        //     // We use +1 since :0 is the base of the template
        //     node.set_location_idx(idx + 1);
        // }

        todo!()

        // let dynamic_nodes = &context.dynamic_nodes;

        // let dyn_attr_printer = context
        //     .dynamic_attributes
        //     .iter()
        //     .map(|attrs| AttributeType::merge_quote(attrs));

        // let node_paths = context.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        // let attr_paths = context.attr_paths.iter().map(|it| quote!(&[#(#it),*]));

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

    fn get_template_id_tokens(&self) -> TokenStream2 {
        todo!()
        // match self.location {
        //     Some(ref loc) => loc.clone(),
        //     None => {
        //         // // Get the root:column:id tag we'll use as the ID of the template
        //         // let root_col = self.get_root_col_id();

        //         quote! {
        //             concat!(
        //                 file!(),
        //                 ":",
        //                 line!(),
        //                 ":",
        //                 column!(),
        //                 ":",
        //                 "0"
        //             )
        //         }
        //     }
        // }
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
        // let root_printer = self
        //     .roots
        //     .iter()
        //     .enumerate()
        //     .map(|(idx, root)| context.render_children_nodes(idx, root));

        // // Render the static nodes, generating the mapping of dynamic
        // quote! {
        //     &[ #( #root_printer ),* ]
        // }
        todo!()
    }

    /// Render a dynamic node
    ///
    /// This is a method on template renderer since we need to cascade down location data into the
    /// child templates.
    fn render_dynamic_node(&mut self, node: &BodyNode, tokens: &mut TokenStream2) {
        match node {
            BodyNode::Element(_) => {
                unreachable!("Elements are never dynamic and should never be queued to be rendered")
            }

            // Raw exprs don't require anything too crazy, just render them out
            BodyNode::RawExpr(exp) => {
                let exp = &exp.expr;
                tokens.append_all(quote! {
                    {
                        let ___nodes = (#exp).into_dyn_node();
                        ___nodes
                    }
                })
            }

            // Dynamic text nodes need ID propagation
            BodyNode::Text(txt) => {
                let txt = &txt.input;
                if txt.is_static() {
                    tokens.append_all(quote! {
                        dioxus_core::DynamicNode::Text(dioxus_core::VText::new(#txt.to_string()))
                    })
                } else {
                    // If the text is dynamic, we actually create a signal of the formatted segments
                    // Crazy, right?
                    let segments = txt.as_htotreloaded();
                    let idx = txt.location.idx.get() + 1;

                    let rendered_segments = txt.segments.iter().filter_map(|s| match s {
                        Segment::Literal(lit) => None,
                        Segment::Formatted(fmt) => {
                            // just render as a format_args! call
                            Some(quote! {
                                format!("{}", #fmt)
                            })
                        }
                    });

                    tokens.append_all(quote! {
                        dioxus_core::DynamicNode::Text(dioxus_core::VText::new({
                            // Create a signal of the formatted segments
                            // htotreloading will find this via its location and then update the signal
                            static __SIGNAL: GlobalSignal<FmtedSegments> = GlobalSignal::with_key(|| #segments, {
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

                            // render the signal and subscribe the component to its changes
                            __SIGNAL.with(|s| s.render_with(
                                vec![ #(#rendered_segments),* ]
                            ))
                        }))
                    })
                }
            }

            // make sure we propgate our template renderer state into the template renderer state we
            // end up creating for the forloop context
            BodyNode::ForLoop(floop) => {
                let ForLoop {
                    pat, expr, body, ..
                } = floop;

                let renderer = TemplateRenderer::as_tokens_with_idx(body, 0);

                // the temporary is important so we create a lifetime binding
                tokens.append_all(quote! {
                    {
                        // let ___nodes = (#expr).into_iter().map(|#pat| { #body }).into_dyn_node();
                        let ___nodes = (#expr).into_iter().map(|#pat| { #renderer }).into_dyn_node();
                        ___nodes
                    }
                })
            }

            // make sure we propgate our template renderer state into the template renderer state we
            // end up creating for component children
            BodyNode::Component(_) => todo!(),

            BodyNode::IfChain(_ifchain) => {
                let mut body = TokenStream2::new();
                let mut terminated = false;

                let mut elif = Some(_ifchain);

                let base_idx = 1123123;
                // let base_idx = self.location.idx.get() * 1000;
                let mut cur_idx = base_idx + 1;

                while let Some(chain) = elif {
                    let IfChain {
                        if_token,
                        cond,
                        then_branch,
                        else_if_branch,
                        else_branch,
                        ..
                    } = chain;

                    let renderer = TemplateRenderer::as_tokens_with_idx(then_branch, cur_idx);
                    body.append_all(quote! { #if_token #cond { {#renderer} } });

                    cur_idx += 1;

                    if let Some(next) = else_if_branch {
                        body.append_all(quote! { else });
                        elif = Some(next);
                    } else if let Some(else_branch) = else_branch {
                        let renderer = TemplateRenderer::as_tokens_with_idx(else_branch, cur_idx);
                        body.append_all(quote! { else { {#renderer} } });
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

                tokens.append_all(quote! {
                    {
                        let ___nodes = (#body).into_dyn_node();
                        ___nodes
                    }
                })
            }
        }
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
            // location,
            // parent_idx: idx,
        })
    }
}
