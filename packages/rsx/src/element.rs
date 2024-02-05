use std::fmt::{Display, Formatter};

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseBuffer, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Brace,
    Expr, Ident, LitStr, Result, Token,
};

// =======================================
// Parse the VNode::Element type
// =======================================
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Element {
    pub name: ElementName,
    pub key: Option<IfmtInput>,
    pub attributes: Vec<AttributeType>,
    pub merged_attributes: Vec<AttributeType>,
    pub children: Vec<BodyNode>,
    pub brace: syn::token::Brace,
}

impl Element {
    /// Create a new element with the given name, attributes and children
    pub fn new(
        key: Option<IfmtInput>,
        name: ElementName,
        attributes: Vec<AttributeType>,
        children: Vec<BodyNode>,
        brace: syn::token::Brace,
    ) -> Self {
        // Deduplicate any attributes that can be combined
        // For example, if there are two `class` attributes, combine them into one
        let mut merged_attributes: Vec<AttributeType> = Vec::new();
        for attr in &attributes {
            let attr_index = merged_attributes
                .iter()
                .position(|a| a.matches_attr_name(attr));

            if let Some(old_attr_index) = attr_index {
                let old_attr = &mut merged_attributes[old_attr_index];

                if let Some(combined) = old_attr.try_combine(attr) {
                    *old_attr = combined;
                }

                continue;
            }

            merged_attributes.push(attr.clone());
        }

        Self {
            name,
            key,
            attributes,
            merged_attributes,
            children,
            brace,
        }
    }
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        let el_name = ElementName::parse(stream)?;

        // parse the guts
        let content: ParseBuffer;
        let brace = syn::braced!(content in stream);

        let mut attributes: Vec<AttributeType> = vec![];
        let mut children: Vec<BodyNode> = vec![];
        let mut key = None;

        // parse fields with commas
        // break when we don't get this pattern anymore
        // start parsing bodynodes
        // "def": 456,
        // abc: 123,
        loop {
            if content.peek(Token![..]) {
                content.parse::<Token![..]>()?;
                let expr = content.parse::<Expr>()?;
                let span = expr.span();
                attributes.push(attribute::AttributeType::Spread(expr));

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(span);
                }
                continue;
            }

            // Parse the raw literal fields
            // "def": 456,
            if content.peek(LitStr) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                let name = content.parse::<LitStr>()?;
                let ident = name.clone();

                content.parse::<Token![:]>()?;

                let value = content.parse::<ElementAttrValue>()?;
                attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                    el_name: el_name.clone(),
                    attr: ElementAttr {
                        name: ElementAttrName::Custom(name),
                        value,
                    },
                }));

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(ident.span());
                }
                continue;
            }

            // Parse
            // abc: 123,
            if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                let name = content.parse::<Ident>()?;

                let name_str = name.to_string();
                content.parse::<Token![:]>()?;

                // The span of the content to be parsed,
                // for example the `hi` part of `class: "hi"`.
                let span = content.span();

                if name_str.starts_with("on") {
                    // check for any duplicate event listeners
                    if attributes.iter().any(|f| {
                        if let AttributeType::Named(ElementAttrNamed {
                            attr:
                                ElementAttr {
                                    name: ElementAttrName::BuiltIn(n),
                                    value: ElementAttrValue::EventTokens(_),
                                },
                            ..
                        }) = f
                        {
                            n == &name_str
                        } else {
                            false
                        }
                    }) {
                        return Err(syn::Error::new(
                            name.span(),
                            format!("Duplicate event listener `{}`", name),
                        ));
                    }
                    attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                        el_name: el_name.clone(),
                        attr: ElementAttr {
                            name: ElementAttrName::BuiltIn(name),
                            value: ElementAttrValue::EventTokens(content.parse()?),
                        },
                    }));
                } else if name_str == "key" {
                    key = Some(content.parse()?);
                } else {
                    let value = content.parse::<ElementAttrValue>()?;
                    attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                        el_name: el_name.clone(),
                        attr: ElementAttr {
                            name: ElementAttrName::BuiltIn(name),
                            value,
                        },
                    }));
                }

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(span);
                }
                continue;
            }

            // Parse shorthand fields
            if content.peek(Ident)
                && !content.peek2(Brace)
                && !content.peek2(Token![:])
                && !content.peek2(Token![-])
            {
                let name = content.parse::<Ident>()?;
                let name_ = name.clone();

                // If the shorthand field is children, these are actually children!
                if name == "children" {
                    return Err(syn::Error::new(
                        name.span(),
                        r#"Shorthand element children are not supported.
To pass children into elements, wrap them in curly braces.
Like so:
    div {{ {{children}} }}

"#,
                    ));
                };

                let value = ElementAttrValue::Shorthand(name.clone());
                attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                    el_name: el_name.clone(),
                    attr: ElementAttr {
                        name: ElementAttrName::BuiltIn(name),
                        value,
                    },
                }));

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(name_.span());
                }
                continue;
            }

            break;
        }

        while !content.is_empty() {
            if (content.peek(LitStr) && content.peek2(Token![:])) && !content.peek3(Token![:]) {
                attr_after_element!(content.span());
            }

            if (content.peek(Ident) && content.peek2(Token![:])) && !content.peek3(Token![:]) {
                attr_after_element!(content.span());
            }

            children.push(content.parse::<BodyNode>()?);
            // consume comma if it exists
            // we don't actually care if there *are* commas after elements/text
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        Ok(Self::new(key, el_name, attributes, children, brace))
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementName {
    Ident(Ident),
    Custom(LitStr),
}

impl ElementName {
    pub(crate) fn tag_name(&self) -> TokenStream2 {
        match self {
            ElementName::Ident(i) => quote! { dioxus_elements::#i::TAG_NAME },
            ElementName::Custom(s) => quote! { #s },
        }
    }
}

impl ElementName {
    pub fn span(&self) -> Span {
        match self {
            ElementName::Ident(i) => i.span(),
            ElementName::Custom(s) => s.span(),
        }
    }
}

impl PartialEq<&str> for ElementName {
    fn eq(&self, other: &&str) -> bool {
        match self {
            ElementName::Ident(i) => i == *other,
            ElementName::Custom(s) => s.value() == *other,
        }
    }
}

impl Display for ElementName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementName::Ident(i) => write!(f, "{}", i),
            ElementName::Custom(s) => write!(f, "{}", s.value()),
        }
    }
}

impl Parse for ElementName {
    fn parse(stream: ParseStream) -> Result<Self> {
        let raw = Punctuated::<Ident, Token![-]>::parse_separated_nonempty(stream)?;
        if raw.len() == 1 {
            Ok(ElementName::Ident(raw.into_iter().next().unwrap()))
        } else {
            let span = raw.span();
            let tag = raw
                .into_iter()
                .map(|ident| ident.to_string())
                .collect::<Vec<_>>()
                .join("-");
            let tag = LitStr::new(&tag, span);
            Ok(ElementName::Custom(tag))
        }
    }
}

impl ToTokens for ElementName {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ElementName::Ident(i) => tokens.append_all(quote! { dioxus_elements::#i }),
            ElementName::Custom(s) => tokens.append_all(quote! { #s }),
        }
    }
}
