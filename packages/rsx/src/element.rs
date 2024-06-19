use std::fmt::{Display, Formatter};

use crate::errors::missing_trailing_comma;

use self::util::try_parse_braces;

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    ext::IdentExt, punctuated::Punctuated, spanned::Spanned, token::Brace, Expr, Ident, LitStr,
    Token,
};

// =======================================
// Parse the VNode::Element type
// =======================================
#[derive(Clone, Debug)]
pub struct Element {
    pub name: ElementName,
    pub key: Option<IfmtInput>,
    pub attributes: Vec<AttributeType>,
    pub merged_attributes: Vec<AttributeType>,
    pub children: Vec<BodyNode>,
    pub brace: Option<syn::token::Brace>,
    // Non-fatal errors that occurred during parsing
    errors: Vec<syn::Error>,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.key == other.key
            && self.attributes == other.attributes
            && self.merged_attributes == other.merged_attributes
            && self.children == other.children
            && self.brace == other.brace
    }
}

impl Eq for Element {}

impl Hash for Element {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.key.hash(state);
        self.attributes.hash(state);
        self.merged_attributes.hash(state);
        self.children.hash(state);
        self.brace.hash(state);
    }
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
            brace: Some(brace),
            errors: Vec::new(),
        }
    }

    /// Create a new incomplete element that has not been fully typed yet
    fn incomplete(name: ElementName) -> Self {
        Self {
            errors: vec![syn::Error::new(
                name.span(),
                format!("Missing braces after element name `{}`", name),
            )],
            name,
            key: None,
            attributes: Vec::new(),
            merged_attributes: Vec::new(),
            children: Vec::new(),
            brace: None,
        }
    }

    pub(crate) fn parse_with_options(
        stream: ParseStream,
        partial_completions: bool,
    ) -> Result<Self> {
        fn peek_any_ident(input: ParseStream) -> bool {
            input.peek(Ident::peek_any)
                && !input.peek(Token![for])
                && !input.peek(Token![if])
                && !input.peek(Token![match])
        }

        let el_name = ElementName::parse(stream)?;

        // parse the guts
        let Ok((brace, content)) = try_parse_braces(stream) else {
            // If there are no braces, this is an incomplete element. We still parse it so that we can autocomplete it, but we don't need to parse the children
            return Ok(Self::incomplete(el_name));
        };

        let mut attributes: Vec<AttributeType> = vec![];
        let mut children: Vec<BodyNode> = vec![];
        let mut key = None;
        let mut errors = Vec::new();

        macro_rules! accumulate_or_return_error {
            ($error:expr) => {
                let error = $error;
                if partial_completions {
                    errors.push(error);
                } else {
                    return Err(error);
                }
            };
        }

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
                    accumulate_or_return_error!(missing_trailing_comma(span));
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
                let followed_by_comma = content.parse::<Token![,]>().is_ok();
                attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                    el_name: el_name.clone(),
                    attr: ElementAttr {
                        name: ElementAttrName::Custom(name),
                        value,
                    },
                    followed_by_comma,
                }));

                if content.is_empty() {
                    break;
                }

                if !followed_by_comma {
                    accumulate_or_return_error!(missing_trailing_comma(ident.span()));
                }
                continue;
            }

            // Parse
            // abc: 123,
            if peek_any_ident(&content) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                let name = Ident::parse_any(&content)?;

                let name_str = name.to_string();
                content.parse::<Token![:]>()?;

                // The span of the content to be parsed,
                // for example the `hi` part of `class: "hi"`.
                let span = content.span();

                if name_str == "key" {
                    let _key: IfmtInput = content.parse()?;

                    if _key.is_static() {
                        invalid_key!(_key);
                    }

                    key = Some(_key);
                } else {
                    let value = if name_str.starts_with("on") {
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
                        ElementAttrValue::EventTokens(content.parse()?)
                    } else {
                        content.parse::<ElementAttrValue>()?
                    };
                    attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                        el_name: el_name.clone(),
                        attr: ElementAttr {
                            name: ElementAttrName::built_in(&name),
                            value,
                        },
                        followed_by_comma: content.peek(Token![,]),
                    }));
                }

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    accumulate_or_return_error!(missing_trailing_comma(span));
                }
                continue;
            }

            // Parse shorthand fields
            if peek_any_ident(&content)
                && !content.peek2(Brace)
                && !content.peek2(Token![:])
                && !content.peek2(Token![-])
            {
                let name = Ident::parse_any(&content)?;
                let name_ = name.clone();

                // If the shorthand field is children, these are actually children!
                if name == "children" {
                    return Err(syn::Error::new(
                        name.span(),
                        r#"Shorthand element children are not supported.
To pass children into elements, wrap them in curly braces.
Like so:
    div { {children} }

"#,
                    ));
                };

                let followed_by_comma = content.parse::<Token![,]>().is_ok();

                // If the shorthand field starts with a capital letter and it isn't followed by a comma, it's actually the start of typing a component
                let starts_with_capital = match name.to_string().chars().next() {
                    Some(c) => c.is_uppercase(),
                    None => false,
                };

                if starts_with_capital && !followed_by_comma {
                    children.push(BodyNode::Component(Component::incomplete(name.into())));
                    continue;
                }

                // Otherwise, it is really a shorthand field
                let value = ElementAttrValue::shorthand(&name);

                attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                    el_name: el_name.clone(),
                    attr: ElementAttr {
                        name: ElementAttrName::built_in(&name),
                        value,
                    },
                    followed_by_comma,
                }));

                if content.is_empty() {
                    break;
                }

                if !followed_by_comma {
                    accumulate_or_return_error!(missing_trailing_comma(name_.span()));
                }
                continue;
            }

            break;
        }

        while !content.is_empty() {
            if ((content.peek(Ident) || content.peek(LitStr)) && content.peek2(Token![:]))
                && !content.peek3(Token![:])
            {
                attr_after_element!(content.span());
            }

            children.push(BodyNode::parse_with_options(&content, partial_completions)?);
            // consume comma if it exists
            // we don't actually care if there *are* commas after elements/text
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        let mut myself = Self::new(key, el_name, attributes, children, brace);

        myself.errors = errors;

        Ok(myself)
    }

    /// If this element doesn't include braces, the user is probably still typing the element name.
    /// We can add hints for rust analyzer to complete the element name better.
    pub(crate) fn completion_hints(&self) -> TokenStream2 {
        let Element { name, brace, .. } = self;

        // If there are braces, this is a complete element and we don't need to add any hints
        if brace.is_some() {
            return quote! {};
        }

        // Only complete the element name if it's a built in element
        let ElementName::Ident(name) = name else {
            return quote! {};
        };

        quote! {
            #[allow(dead_code)]
            {
                // Autocomplete as an element
                dioxus_elements::elements::completions::CompleteWithBraces::#name;
            }
        }
    }

    /// If this element is only partially complete, return the errors that occurred during parsing
    pub(crate) fn errors(&self) -> TokenStream2 {
        let Element { errors, .. } = self;

        let mut tokens = quote! {};
        for error in errors {
            tokens.append_all(error.to_compile_error());
        }

        tokens
    }
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        Self::parse_with_options(stream, true)
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
            ElementName::Ident(i) => quote! { dioxus_elements::elements::#i::TAG_NAME },
            ElementName::Custom(s) => quote! { #s },
        }
    }

    pub(crate) fn namespace(&self) -> TokenStream2 {
        match self {
            ElementName::Ident(i) => quote! { dioxus_elements::elements::#i::NAME_SPACE },
            ElementName::Custom(_) => quote! { None },
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
            ElementName::Ident(i) => tokens.append_all(quote! { dioxus_elements::elements::#i }),
            ElementName::Custom(s) => tokens.append_all(quote! { #s }),
        }
    }
}
