use crate::mapping::DynamicMapping;
use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

/// As we create the dynamic nodes, we want to keep track of them in a linear fashion
/// We'll use the size of the vecs to determine the index of the dynamic node in the final output
#[derive(Default, Debug)]
pub struct DynamicContext<'a> {
    pub dynamic_nodes: Vec<&'a BodyNode>,
    pub dynamic_attributes: Vec<Vec<&'a AttributeType>>,
    pub current_path: Vec<u8>,
    pub node_paths: Vec<Vec<u8>>,
    pub attr_paths: Vec<Vec<u8>>,

    // The mapping is used to map the old template to the new template
    // Not having a mapping means that we're just creating new nodes
    pub mapping: Option<DynamicMapping>,
}

impl<'a> DynamicContext<'a> {
    pub fn new_with_old(template: Option<CallBody>) -> Self {
        let mapping = template.map(|call| mapping::DynamicMapping::new(call.roots));
        Self {
            mapping,
            ..Self::default()
        }
    }

    /// Render a portion of an rsx callbody to a token stream
    pub fn render_static_node(&mut self, root: &'a BodyNode) -> TokenStream2 {
        match root {
            BodyNode::Element(el) => self.render_static_element(el),

            BodyNode::Text(text) if text.is_static() => {
                let text = text.to_static().unwrap();
                quote! { dioxus_core::TemplateNode::Text{ text: #text } }
            }

            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => self.render_dynamic_node(root),
        }
    }

    fn render_dynamic_node(&mut self, root: &'a BodyNode) -> TokenStream2 {
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

    fn render_static_element(&mut self, el: &'a Element) -> TokenStream2 {
        let el_name = &el.name;
        let ns = |name| match el_name {
            ElementName::Ident(i) => quote! { dioxus_elements::#i::#name },
            ElementName::Custom(_) => quote! { None },
        };

        let static_attrs = el
            .merged_attributes
            .iter()
            .map(|attr| self.render_merged_attributes(attr, ns, el_name))
            .collect::<Vec<_>>();

        let children = el
            .children
            .iter()
            .enumerate()
            .map(|(idx, root)| self.render_children_nodes(idx, root))
            .collect::<Vec<_>>();

        let ns = ns(quote!(NAME_SPACE));
        let el_name = el_name.tag_name();

        quote! {
            dioxus_core::TemplateNode::Element {
                tag: #el_name,
                namespace: #ns,
                attrs: &[ #(#static_attrs)* ],
                children: &[ #(#children),* ],
            }
        }
    }

    fn render_children_nodes(&mut self, idx: usize, root: &'a BodyNode) -> TokenStream2 {
        self.current_path.push(idx as u8);
        let out = self.render_static_node(root);
        self.current_path.pop();
        out
    }

    /// Render the attributes of an element
    fn render_merged_attributes(
        &mut self,
        attr: &'a AttributeType,
        ns: impl Fn(TokenStream2) -> TokenStream2,
        el_name: &ElementName,
    ) -> TokenStream2 {
        // Rendering static attributes requires a bit more work than just a dynamic attrs
        match attr.as_static_str_literal() {
            // If it's static, we'll take this little optimization
            Some((name, value)) => Self::render_static_attr(value, name, ns, el_name),

            // Otherwise, we'll just render it as a dynamic attribute
            // This will also insert the attribute into the dynamic_attributes list to assemble the final template
            _ => self.render_dynamic_attr(attr),
        }
    }

    fn render_static_attr(
        value: &IfmtInput,
        name: &ElementAttrName,
        ns: impl Fn(TokenStream2) -> TokenStream2,
        el_name: &ElementName,
    ) -> TokenStream2 {
        let value = value.to_static().unwrap();

        let ns = match name {
            ElementAttrName::BuiltIn(name) => ns(quote!(#name.1)),
            ElementAttrName::Custom(_) => quote!(None),
        };

        let name = match (el_name, name) {
            (ElementName::Ident(_), ElementAttrName::BuiltIn(_)) => quote! { #el_name::#name.0 },
            _ => {
                let as_string = name.to_string(); //hmmmm I think we could just totokens this, but the to_string might be inserting quotes
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

    /// If the attr is dynamic
    fn render_dynamic_attr(&mut self, attr: &'a AttributeType) -> TokenStream2 {
        let ct = self.dynamic_attributes.len();

        self.dynamic_attributes.push(vec![attr]);
        self.attr_paths.push(self.current_path.clone());

        quote! { dioxus_core::TemplateAttribute::Dynamic { id: #ct }, }
    }

    #[cfg(feature = "hot_reload")]
    pub fn update_node<Ctx: HotReloadingContext>(
        &mut self,
        root: &'a BodyNode,
    ) -> Option<TemplateNode> {
        match root {
            // The user is moving a static node around in the template
            // Check this is a bit more complex but we can likely handle it
            BodyNode::Element(el) => self.update_element::<Ctx>(el),

            BodyNode::Text(text) if text.is_static() => {
                let text = text.source.as_ref().unwrap();
                let text = intern(text.value().as_str());
                Some(TemplateNode::Text { text })
            }

            // The user is moving a dynamic node around in the template
            // We *might* be able to handle it, but you never really know
            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => self.update_dynamic_node(root),
        }
    }

    /// Attempt to update a dynamic node in the template
    ///
    /// If the change between the old and new template results in a mapping that doesn't exist, then we need to bail out.
    /// Basically if we *had* a mapping of `[0, 1]` and the new template is `[1, 2]`, then we need to bail out, since
    /// the new mapping doesn't exist in the original.
    fn update_dynamic_node(&mut self, root: &'a BodyNode) -> Option<TemplateNode> {
        let idx = match self.mapping {
            //    Bail out if the mapping doesn't exist
            //    The user put it new code in the template, and that code is not hotreloadable
            Some(ref mut mapping) => mapping.get_node_idx(root)?,
            None => self.dynamic_nodes.len(),
        };

        // Put the node in the dynamic nodes list
        self.dynamic_nodes.push(root);

        // Fill in as many paths as we need - might have to fill in more since the old tempate shrunk some and let some paths be empty
        if self.node_paths.len() <= idx {
            self.node_paths.resize_with(idx + 1, Vec::new);
        }

        // And then set the path of this node to the current path (which we're hitting during traversal)
        self.node_paths[idx] = self.current_path.clone();

        Some(match root {
            BodyNode::Text(_) => TemplateNode::DynamicText { id: idx },
            _ => TemplateNode::Dynamic { id: idx },
        })
    }

    fn update_element<Ctx: HotReloadingContext>(
        &mut self,
        el: &'a Element,
    ) -> Option<TemplateNode> {
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
                    let idx = match self.mapping.as_mut() {
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
            children.push(self.update_node::<Ctx>(root)?);
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
}
