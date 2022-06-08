//! Parse the root tokens in the rsx!{} macro
//! =========================================
//!
//! This parsing path emerges directly from the macro call, with `RsxRender` being the primary entrance into parsing.
//! This feature must support:
//! - [x] Optionally rendering if the `in XYZ` pattern is present
//! - [x] Fragments as top-level element (through ambiguous)
//! - [x] Components as top-level element (through ambiguous)
//! - [x] Tags as top-level elements (through ambiguous)
//! - [x] Good errors if parsing fails
//!
//! Any errors in using rsx! will likely occur when people start using it, so the first errors must be really helpful.

use std::collections::HashMap;
use std::rc::Rc;

// imports
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Ident, LitStr, Result, Token,
};

// Re-export the namespaces into each other
pub use component::*;
pub use element::*;
pub use node::*;

use crate::props::injection::{Branch, InjectedProperties, Property};

#[macro_use]
mod errors;

mod component;
mod element;
mod node;

pub mod pretty;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CustomContext {
    pub name: Ident,
    pub cx_type: Option<Ident>,
}

pub struct CallBody {
    pub custom_context: Option<CustomContext>,
    pub roots: Vec<BodyNode>,
}

impl Parse for CallBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let custom_context = if input.peek(Ident) && input.peek2(Token![:]) && input.peek3(Ident) {
            let name = input.parse::<Ident>()?;
            input.parse::<Token![:]>()?;
            let r#type = input.parse::<Ident>()?;
            input.parse::<Token![;]>()?;

            Some(CustomContext {
                name,
                cx_type: Some(r#type),
            })
        } else if input.peek(Ident) && input.peek2(Token![,]) {
            let name = input.parse::<Ident>()?;
            input.parse::<Token![,]>()?;

            Some(CustomContext {
                name,
                cx_type: None,
            })
        } else {
            None
        };

        let mut roots = Vec::new();

        while !input.is_empty() {
            let node = input.parse::<BodyNode>()?;

            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>();
            }

            roots.push(node);
        }

        if let Some(CustomContext {
            name,
            cx_type: Some(cx_type),
        }) = custom_context.as_ref()
        {
            inject_attributes(name, cx_type, &mut roots)?;
        }

        Ok(Self {
            custom_context,
            roots,
        })
    }
}

fn inject_attributes(ctx: &Ident, component: &Ident, roots: &mut [BodyNode]) -> Result<()> {
    let mut branch = Branch::new();
    let properties = InjectedProperties::component_properties(component)?;
    let component = component.to_string();
    let context = ctx.to_string();

    return inject_attributes(&context, &component, roots, &mut branch, &properties);

    // recursive impl of inject attributes
    fn inject_attributes(
        cx: &str,
        component: &str,
        nodes: &mut [BodyNode],
        branch: &mut Branch,
        properties: &[Property],
    ) -> Result<()> {
        let total = nodes.len();
        let totals = calc_type_totals(nodes);

        let mut inject_properties =
            |index: usize,
             name: &Ident,
             children: &mut Vec<BodyNode>,
             mut inject_property: Box<dyn FnMut(&Ident, &Property) -> Result<()>>|
             -> Result<()> {
                if index == 0 {
                    branch.new_child(name, total, totals.clone())
                } else {
                    branch
                        .next_sibling(name)
                        .map_err(|err| syn::Error::new(name.span(), err))?;
                }

                for property in properties {
                    let applies = InjectedProperties::check_branch(component, property, branch)
                        .map_err(|err| syn::Error::new(name.span(), err))?;

                    if applies {
                        inject_property(name, property)?
                    }
                }

                if !children.is_empty() {
                    inject_attributes(cx, component, children, branch, properties)?;
                }

                Ok(())
            };

        let mut branched = false;

        for (index, node) in nodes.iter_mut().enumerate() {
            match node {
                BodyNode::Element(Element {
                    name,
                    attributes,
                    children,
                    ..
                }) => {
                    branched = true;

                    inject_properties(
                        index,
                        name,
                        children,
                        Box::new(move |el_name, property| {
                            let attr = match property {
                                Property::Attribute {
                                    name,
                                    inject_as,
                                    optional,
                                } => ElementAttr::CustomAttrText {
                                    name: LitStr::new(inject_as, el_name.span()),
                                    value: LitStr::new(
                                        &format!(
                                            "{{{cx}.props.{name}{}}}",
                                            if *optional { ":?" } else { "" }
                                        ),
                                        el_name.span(),
                                    ),
                                },
                                Property::Handler {
                                    name,
                                    inject_as,
                                    optional,
                                } => ElementAttr::EventTokens {
                                    name: Ident::new(inject_as, el_name.span()),
                                    tokens: if *optional {
                                        syn::parse_str(&format!("|evt| if let Some({name}) = &{cx}.props.{name} {{ {name}.call(evt) }}"))?
                                    } else {
                                        syn::parse_str(&format!(
                                            "|evt| {cx}.props.{name}.call(evt)"
                                        ))?
                                    },
                                },
                            };

                            attributes.push(ElementAttrNamed {
                                el_name: el_name.clone(),
                                attr,
                            });

                            Ok(())
                        }),
                    )?;
                }
                BodyNode::Component(Component {
                    name,
                    body,
                    children,
                    ..
                }) => {
                    branched = true;

                    let name = match name.segments.last() {
                        Some(last) => &last.ident,
                        None => {
                            return Err(syn::Error::new_spanned(name, "Expected component name"))
                        }
                    };

                    inject_properties(
                        index,
                        name,
                        children,
                        Box::new(|el_name, property| {
                            let attr = match property {
                                Property::Attribute {
                                    name,
                                    inject_as,
                                    optional,
                                } => ComponentField {
                                    name: Ident::new(inject_as, el_name.span()),
                                    content: ContentField::Formatted(LitStr::new(
                                        &format!(
                                            "{{{cx}.props.{name}{}}}",
                                            if *optional { ":?" } else { "" }
                                        ),
                                        el_name.span(),
                                    )),
                                },
                                Property::Handler {
                                    name,
                                    inject_as,
                                    optional,
                                } => ComponentField {
                                    name: Ident::new(inject_as, el_name.span()),
                                    content: ContentField::OnHandlerRaw(if *optional {
                                        syn::parse_str(&format!("|evt| if let Some({name}) = &{cx}.props.{name} {{ {name}.call(evt) }}"))?
                                    } else {
                                        syn::parse_str(&format!(
                                            "|evt| {cx}.props.{name}.call(evt)"
                                        ))?
                                    }),
                                },
                            };

                            body.push(attr);

                            Ok(())
                        }),
                    )?;
                }
                _ => {}
            }
        }

        if branched {
            branch
                .finish()
                .map_err(|err| syn::Error::new(Span::call_site(), err))?;
        }

        Ok(())
    }

    fn calc_type_totals(nodes: &mut [BodyNode]) -> Rc<HashMap<String, usize>> {
        Rc::new(
            nodes
                .iter()
                .filter_map(|bn| match bn {
                    BodyNode::Element(elm) => Some(elm.name.to_token_stream().to_string()),
                    BodyNode::Component(cmp) => {
                        Some(cmp.name.segments.last().unwrap().ident.to_string())
                    }
                    BodyNode::Text(_) | BodyNode::RawExpr(_) => None,
                })
                .fold(HashMap::new(), |mut acc, next| {
                    let entry = acc.entry(next).or_insert(0_usize);

                    *entry += 1;

                    acc
                }),
        )
    }
}

/// Serialize the same way, regardless of flavor
impl ToTokens for CallBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let inner = if self.roots.len() == 1 {
            let inner = &self.roots[0];
            quote! { #inner }
        } else {
            let childs = &self.roots;
            quote! { __cx.fragment_root([ #(#childs),* ]) }
        };

        match &self.custom_context {
            // The `in cx` pattern allows directly rendering
            Some(CustomContext { name, .. }) => out_tokens.append_all(quote! {
                #name.render(LazyNodes::new(move |__cx: NodeFactory| -> VNode {
                    use dioxus_elements::{GlobalAttributes, SvgAttributes};
                    #inner
                }))
            }),

            // Otherwise we just build the LazyNode wrapper
            None => out_tokens.append_all(quote! {
                LazyNodes::new(move |__cx: NodeFactory| -> VNode {
                    use dioxus_elements::{GlobalAttributes, SvgAttributes};
                    #inner
                })
            }),
        };
    }
}
