use crate::mapping::DynamicMapping;
use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

// As we create the dynamic nodes, we want to keep track of them in a linear fashion
// We'll use the size of the vecs to determine the index of the dynamic node in the final output
#[derive(Default, Debug)]
pub struct DynamicContext<'a> {
    pub dynamic_nodes: Vec<&'a BodyNode>,
    pub dynamic_attributes: Vec<Vec<&'a AttributeType>>,
    pub current_path: Vec<u8>,
    pub node_paths: Vec<Vec<u8>>,
    pub attr_paths: Vec<Vec<u8>>,
}

impl<'a> DynamicContext<'a> {
    /// Render a portion of an rsx callbody to a token stream
    pub fn render_static_node(&mut self, root: &'a BodyNode) -> TokenStream2 {
        match root {
            BodyNode::Element(el) => {
                let el_name = &el.name;
                let ns = |name| match el_name {
                    ElementName::Ident(i) => quote! { dioxus_elements::#i::#name },
                    ElementName::Custom(_) => quote! { None },
                };
                let static_attrs = el.merged_attributes.iter().map(|attr| match attr {
                    AttributeType::Named(ElementAttrNamed {
                        attr:
                            ElementAttr {
                                value: ElementAttrValue::AttrLiteral(value),
                                name,
                            },
                        ..
                    }) if value.is_static() => {
                        let value = value.to_static().unwrap();
                        let ns = {
                            match name {
                                ElementAttrName::BuiltIn(name) => ns(quote!(#name.1)),
                                ElementAttrName::Custom(_) => quote!(None),
                            }
                        };
                        let name = match (el_name, name) {
                            (ElementName::Ident(_), ElementAttrName::BuiltIn(_)) => {
                                quote! { #el_name::#name.0 }
                            }
                            _ => {
                                let as_string = name.to_string();
                                quote! { #as_string }
                            }
                        };
                        quote! {
                            dioxus_core::TemplateAttribute::Static {
                                name: #name,
                                namespace: #ns,
                                value: #value,

                                // todo: we don't diff these so we never apply the volatile flag
                                // volatile: dioxus_elements::#el_name::#name.2,
                            },
                        }
                    }

                    _ => {
                        let ct = self.dynamic_attributes.len();
                        self.dynamic_attributes.push(vec![attr]);
                        self.attr_paths.push(self.current_path.clone());
                        quote! { dioxus_core::TemplateAttribute::Dynamic { id: #ct }, }
                    }
                });

                let attrs = quote! { #(#static_attrs)* };

                let children = el.children.iter().enumerate().map(|(idx, root)| {
                    self.current_path.push(idx as u8);
                    let out = self.render_static_node(root);
                    self.current_path.pop();
                    out
                });

                let children = quote! { #(#children),* };

                let ns = ns(quote!(NAME_SPACE));
                let el_name = el_name.tag_name();

                quote! {
                    dioxus_core::TemplateNode::Element {
                        tag: #el_name,
                        namespace: #ns,
                        attrs: &[ #attrs ],
                        children: &[ #children ],
                    }
                }
            }

            BodyNode::Text(text) if text.is_static() => {
                let text = text.to_static().unwrap();
                quote! { dioxus_core::TemplateNode::Text{ text: #text } }
            }

            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => {
                let ct = self.dynamic_nodes.len();
                self.dynamic_nodes.push(root);
                self.node_paths.push(self.current_path.clone());

                match root {
                    BodyNode::Text(_) => {
                        quote! { dioxus_core::TemplateNode::DynamicText { id: #ct } }
                    }
                    _ => quote! { dioxus_core::TemplateNode::Dynamic { id: #ct } },
                }
            }
        }
    }

    #[cfg(feature = "hot_reload")]
    pub fn update_node<Ctx: HotReloadingContext>(
        &mut self,
        root: &'a BodyNode,
        mapping: &mut Option<DynamicMapping>,
    ) -> Option<TemplateNode> {
        match root {
            BodyNode::Element(el) => {
                let element_name_rust = el.name.to_string();

                let mut static_attrs = Vec::new();
                for attr in &el.merged_attributes {
                    match &attr {
                        AttributeType::Named(ElementAttrNamed {
                            attr:
                                ElementAttr {
                                    value: ElementAttrValue::AttrLiteral(value),
                                    name,
                                },
                            ..
                        }) if value.is_static() => {
                            let value = value.source.as_ref().unwrap();
                            let attribute_name_rust = name.to_string();
                            let (name, namespace) =
                                Ctx::map_attribute(&element_name_rust, &attribute_name_rust)
                                    .unwrap_or((intern(attribute_name_rust.as_str()), None));
                            static_attrs.push(TemplateAttribute::Static {
                                name,
                                namespace,
                                value: intern(value.value().as_str()),
                            })
                        }

                        _ => {
                            let idx = match mapping {
                                Some(mapping) => mapping.get_attribute_idx(attr)?,
                                None => self.dynamic_attributes.len(),
                            };
                            self.dynamic_attributes.push(vec![attr]);

                            if self.attr_paths.len() <= idx {
                                self.attr_paths.resize_with(idx + 1, Vec::new);
                            }
                            self.attr_paths[idx] = self.current_path.clone();
                            static_attrs.push(TemplateAttribute::Dynamic { id: idx })
                        }
                    }
                }

                let mut children = Vec::new();
                for (idx, root) in el.children.iter().enumerate() {
                    self.current_path.push(idx as u8);
                    children.push(self.update_node::<Ctx>(root, mapping)?);
                    self.current_path.pop();
                }

                let (tag, namespace) = Ctx::map_element(&element_name_rust)
                    .unwrap_or((intern(element_name_rust.as_str()), None));
                Some(TemplateNode::Element {
                    tag,
                    namespace,
                    attrs: intern(static_attrs.into_boxed_slice()),
                    children: intern(children.as_slice()),
                })
            }

            BodyNode::Text(text) if text.is_static() => {
                let text = text.source.as_ref().unwrap();
                Some(TemplateNode::Text {
                    text: intern(text.value().as_str()),
                })
            }

            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => {
                let idx = match mapping {
                    Some(mapping) => mapping.get_node_idx(root)?,
                    None => self.dynamic_nodes.len(),
                };
                self.dynamic_nodes.push(root);

                if self.node_paths.len() <= idx {
                    self.node_paths.resize_with(idx + 1, Vec::new);
                }
                self.node_paths[idx] = self.current_path.clone();

                Some(match root {
                    BodyNode::Text(_) => TemplateNode::DynamicText { id: idx },
                    _ => TemplateNode::Dynamic { id: idx },
                })
            }
        }
    }
}
