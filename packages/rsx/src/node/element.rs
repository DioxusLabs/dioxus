use std::fmt::{Display, Formatter};

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::ParseBuffer, punctuated::Punctuated, spanned::Spanned, token::Brace, Expr, Ident,
    LitStr, Token,
};

/// Parse the VNode::Element type
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Element {
    pub name: ElementName,
    pub raw_attributes: Vec<Attribute>,
    pub merged_attributes: Vec<Attribute>,
    pub spreads: Vec<Spread>,
    pub children: Vec<BodyNode>,
    pub brace: syn::token::Brace,
    pub diagnostics: Diagnostics,
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name = stream.parse::<ElementName>()?;

        let RsxBlock {
            fields,
            children,
            brace,
            spreads,
            diagnostics,
        } = stream.parse::<RsxBlock>()?;

        let mut element = Element {
            name,
            raw_attributes: fields,
            children,
            brace,
            spreads,
            diagnostics,
            merged_attributes: Vec::new(),
        };

        element.merge_attributes();
        element.validate();

        Ok(element)
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let el = self;

        let el_name = &el.name;
        let ns = |name| match el_name {
            ElementName::Ident(i) => quote! { dioxus_elements::#i::#name },
            ElementName::Custom(_) => quote! { None },
        };

        let static_attrs = el
            .merged_attributes
            .iter()
            .map(|attr| {
                // Rendering static attributes requires a bit more work than just a dynamic attrs
                match attr.as_static_str_literal() {
                    // If it's static, we'll take this little optimization
                    Some((name, value)) => {
                        let value = value.to_static().unwrap();

                        let ns = match name {
                            AttributeName::BuiltIn(name) => ns(quote!(#name.1)),
                            AttributeName::Custom(_) => quote!(None),
                        };

                        let name = match (el_name, name) {
                            (ElementName::Ident(_), AttributeName::BuiltIn(_)) => {
                                quote! { #el_name::#name.0 }
                            }
                            //hmmmm I think we could just totokens this, but the to_string might be inserting quotes
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

                    // Otherwise, we'll just render it as a dynamic attribute
                    // This will also insert the attribute into the dynamic_attributes list to assemble the final template
                    _ => {
                        let id = attr.dyn_idx.get();
                        quote! {
                            dioxus_core::TemplateAttribute::Dynamic { id: #id  },
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        // Render either the child
        let children = el.children.iter().map(|c| match c {
            BodyNode::Element(el) => quote! { #el },
            BodyNode::Text(text) if text.is_static() => {
                let text = text.input.to_static().unwrap();
                quote! { dioxus_core::TemplateNode::Text { text: #text } }
            }
            BodyNode::Text(text) => {
                let id = text.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::DynamicText { id: #id } }
            }
            BodyNode::ForLoop(floop) => {
                let id = floop.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::RawExpr(exp) => {
                let id = exp.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::Component(exp) => {
                let id = exp.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::IfChain(exp) => {
                let id = exp.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
        });

        let ns = ns(quote!(NAME_SPACE));
        let el_name = el_name.tag_name();
        let diagnostics = &el.diagnostics;

        // todo: generate less code if there's no diagnostics by not including the curlies
        tokens.append_all(quote! {
            {
                #diagnostics

                dioxus_core::TemplateNode::Element {
                    tag: #el_name,
                    namespace: #ns,
                    attrs: &[ #(#static_attrs)* ],
                    children: &[ #(#children),* ],
                }
            }
        })
    }
}

impl Element {
    /// Throw warnings if there are any issues with the element
    /// - invalid names
    /// - issues merging attributes
    /// - reserved keywords
    /// idk what else
    fn validate(&mut self) {}

    /// Collapses ifmt attributes into a single dynamic attribute using a space as a delimiter
    ///
    /// div {
    ///     class: "abc-def",
    ///     class: if some_expr { "abc" },
    /// }
    ///
    fn merge_attributes(&mut self) {
        // Now merge the attributes into the cache
        for new_attr in &self.raw_attributes {
            let attr_index = self
                .merged_attributes
                .iter()
                .position(|a| a.name == new_attr.name);

            if let Some(old_attr_index) = attr_index {
                let old_attr = &mut self.merged_attributes[old_attr_index];

                todo!("Merge attributes properly!");

                continue;
            }

            self.merged_attributes.push(new_attr.clone());
        }
    }

    pub(crate) fn key(&self) -> Option<&IfmtInput> {
        for attr in &self.raw_attributes {
            if let AttributeName::BuiltIn(name) = &attr.name {
                if name == "key" {
                    return attr.ifmt();
                }
            }
        }

        None
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementName {
    Ident(Ident),
    Custom(LitStr),
}

impl ToTokens for ElementName {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ElementName::Ident(i) => i.to_tokens(tokens),
            ElementName::Custom(s) => s.to_tokens(tokens),
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

impl ElementName {
    pub(crate) fn tag_name(&self) -> TokenStream2 {
        match self {
            ElementName::Ident(i) => quote! { dioxus_elements::#i::TAG_NAME },
            ElementName::Custom(s) => quote! { #s },
        }
    }

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

#[test]
fn parses_name() {
    let parsed: ElementName = syn::parse2(quote::quote! { div }).unwrap();
    let parsed: ElementName = syn::parse2(quote::quote! { some-cool-element }).unwrap();

    let parsed: Element = syn::parse2(quote::quote! { div {} }).unwrap();
    let parsed: Element = syn::parse2(quote::quote! { some-cool-element {} }).unwrap();

    let parsed: Element = syn::parse2(quote::quote! {
        some-cool-div {
            id: "hi",
            id: "hi {abc}",
            id: "hi {def}",
            class: 123,
            something: bool,
            data_attr: "data",
            data_attr: "data2",
            data_attr: "data3",
            exp: { some_expr },
            something: {cool},
            something: bool,
            something: 123,
            onclick: move |_| {
                println!("hello world");
            },
            "some-attr": "hello world",
            onclick: move |_| {},
            class: "hello world",
            id: "my-id",
            data_attr: "data",
            data_attr: "data2",
            data_attr: "data3",
            "somte_attr3": "hello world",
            something: {cool},
            something: bool,
            something: 123,
            onclick: move |_| {
                println!("hello world");
            },
            ..attrs1,
            ..attrs2,
            ..attrs3
        }
    })
    .unwrap();

    dbg!(parsed);
}

#[test]
fn parses_variety() {
    let input = quote::quote! {
        div {
            class: "hello world",
            id: "my-id",
            data_attr: "data",
            data_attr: "data2",
            data_attr: "data3",
            "somte_attr3": "hello world",
            something: {cool},
            something: bool,
            something: 123,
            onclick: move |_| {
                println!("hello world");
            },
            ..attrs,
            ..attrs2,
            ..attrs3
        }
    };

    let parsed: Element = syn::parse2(input).unwrap();
    dbg!(parsed);
}

#[test]
fn to_tokens_properly() {
    let input = quote::quote! {
        div {
            class: "hello world",
            class2: "hello {world}",
            class3: "goodbye {world}",
            class4: "goodbye world",
            "something": "cool {blah}",
            "something2": "cooler",
            div {
                div {
                    h1 { class: "h1 col" }
                    h2 { class: "h2 col" }
                    h3 { class: "h3 col" }
                    div {}
                }
            }
        }
    };

    let parsed: Element = syn::parse2(input).unwrap();
    println!("{}", parsed.to_token_stream().pretty_unparse());
}

#[test]
fn to_tokens_with_diagnostic() {
    let input = quote::quote! {
        div {
            class: "hello world",
            id: "my-id",
            ..attrs,
            div {
                ..attrs,
                class: "hello world",
                id: "my-id",
            }
        }
    };

    let parsed: Element = syn::parse2(input).unwrap();
    println!("{}", parsed.to_token_stream().pretty_unparse());
}

#[test]
fn merges_attributes() {
    let input = quote::quote! {
        div {
            class: "hello world",
            class: if some_expr { "abc" }
        }
    };

    let parsed: Element = syn::parse2(input).unwrap();
    assert!(parsed.merged_attributes.len() == 1);
}
