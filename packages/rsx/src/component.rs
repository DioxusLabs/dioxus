//! Parse components into the VNode::Component variant
//! ==========================================
//!
//! We can be reasonably sure that whatever enters this parsing path is in the right format.
//! This feature must support
//! - [x] Namespaced components
//! - [x] Fields
//! - [x] Componentbuilder synax
//! - [x] Optional commas
//! - [ ] Children
//! - [ ] Keys
//! - [ ] Properties spreading with with `..` syntax

use self::{location::CallerLocation, util::try_parse_braces};

use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{
    ext::IdentExt, spanned::Spanned, token::Brace, AngleBracketedGenericArguments, Error, Expr,
    Ident, LitStr, PathArguments, Token,
};
use tracing::span;

#[derive(Clone, Debug)]
pub struct Component {
    pub name: syn::Path,
    pub prop_gen_args: Option<AngleBracketedGenericArguments>,
    pub key: Option<IfmtInput>,
    pub fields: Vec<ComponentField>,
    pub children: Vec<BodyNode>,
    pub manual_props: Option<Expr>,
    pub brace: Option<syn::token::Brace>,
    pub location: CallerLocation,
    errors: Vec<syn::Error>,
}

impl PartialEq for Component {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.prop_gen_args == other.prop_gen_args
            && self.key == other.key
            && self.fields == other.fields
            && self.children == other.children
            && self.manual_props == other.manual_props
            && self.brace == other.brace
    }
}

impl Eq for Component {}

impl Hash for Component {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.prop_gen_args.hash(state);
        self.key.hash(state);
        self.fields.hash(state);
        self.children.hash(state);
        self.manual_props.hash(state);
        self.brace.hash(state);
    }
}

impl Parse for Component {
    fn parse(stream: ParseStream) -> Result<Self> {
        let mut name = stream.parse::<syn::Path>()?;
        Component::validate_component_path(&name)?;

        // extract the path arguments from the path into prop_gen_args
        let prop_gen_args = normalize_path(&mut name);

        let Ok((brace, content)) = try_parse_braces(stream) else {
            // If there are no braces, this is an incomplete component. We still parse it so that we can autocomplete it, but we don't need to parse the children
            return Ok(Self::incomplete(name));
        };

        let mut fields = Vec::new();
        let mut children = Vec::new();
        let mut manual_props = None;
        let mut key = None;

        while !content.is_empty() {
            // if we splat into a component then we're merging properties
            if content.peek(Token![..]) {
                content.parse::<Token![..]>()?;
                manual_props = Some(content.parse()?);
            } else if
            // Named fields
            (content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]))
                // shorthand struct initialization
                // Not a div {}, mod::Component {}, or web-component {}
                || (content.peek(Ident)
                    && !content.peek2(Brace)
                    && !content.peek2(Token![:])
                    && !content.peek2(Token![-]))
            {
                // If it is a key, make sure it isn't static and then add it to the component
                if content.fork().parse::<Ident>()? == "key" {
                    _ = content.parse::<Ident>()?;
                    _ = content.parse::<Token![:]>()?;

                    let _key: IfmtInput = content.parse()?;
                    if _key.is_static() {
                        invalid_key!(_key);
                    }
                    key = Some(_key);
                } else {
                    fields.push(content.parse()?);
                }
            } else {
                children.push(content.parse()?);
            }

            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        Ok(Self {
            location: CallerLocation::default(),
            name,
            prop_gen_args,
            fields,
            children,
            manual_props,
            brace: Some(brace),
            key,
            errors: Vec::new(),
        })
    }
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self {
            name,
            prop_gen_args,
            ..
        } = self;

        let builder = self
            .manual_props
            .as_ref()
            .map(|props| self.collect_manual_props(props))
            .unwrap_or_else(|| self.collect_props());

        let fn_name = self.fn_name();

        let completion_hints = self.completion_hints();
        let errors = self.errors();

        let component_node = quote_spanned! { name.span() =>
            dioxus_core::DynamicNode::Component({
                #[allow(unused_imports)]
                use dioxus_core::prelude::Properties;
                (#builder).into_vcomponent(
                    #name #prop_gen_args,
                    #fn_name
                )
            })
        };

        let component = if errors.is_empty() && completion_hints.is_empty() {
            component_node
        } else {
            quote_spanned! {
                name.span() => {
                    #completion_hints
                    #errors
                    #component_node
                }
            }
        };

        tokens.append_all(component);
    }
}

impl Component {
    /// Create a new Component
    pub fn new(
        name: syn::Path,
        prop_gen_args: Option<AngleBracketedGenericArguments>,
        fields: Vec<ComponentField>,
        children: Vec<BodyNode>,
        manual_props: Option<Expr>,
        key: Option<IfmtInput>,
        brace: syn::token::Brace,
    ) -> Self {
        Self {
            errors: vec![],
            name,
            prop_gen_args,
            fields,
            children,
            manual_props,
            brace: Some(brace),
            key,
            location: CallerLocation::default(),
        }
    }

    pub(crate) fn incomplete(name: syn::Path) -> Self {
        Self {
            errors: vec![syn::Error::new(
                name.span(),
                format!(
                    "Missing braces after component name `{}`",
                    name.segments.last().unwrap().ident
                ),
            )],
            name,
            prop_gen_args: None,
            fields: Vec::new(),
            children: Vec::new(),
            manual_props: None,
            brace: None,
            key: None,
            location: CallerLocation::default(),
        }
    }

    fn validate_component_path(path: &syn::Path) -> Result<()> {
        // ensure path segments doesn't have PathArguments, only the last
        // segment is allowed to have one.
        if path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .any(|seg| seg.arguments != PathArguments::None)
        {
            component_path_cannot_have_arguments!(path.span());
        }

        // ensure last segment only have value of None or AngleBracketed
        if !matches!(
            path.segments.last().unwrap().arguments,
            PathArguments::None | PathArguments::AngleBracketed(_)
        ) {
            invalid_component_path!(path.span());
        }

        Ok(())
    }

    pub fn key(&self) -> Option<&IfmtInput> {
        self.key.as_ref()
    }

    fn collect_manual_props(&self, manual_props: &Expr) -> TokenStream2 {
        let mut toks =
            quote_spanned! { manual_props.span() => let mut __manual_props = #manual_props; };
        for field in &self.fields {
            if field.name == "key" {
                continue;
            }
            let ComponentField { name, content } = field;
            toks.append_all(
                quote_spanned! { manual_props.span() => __manual_props.#name = #content; },
            );
        }
        toks.append_all(quote_spanned! { manual_props.span() => __manual_props });
        quote! {{ #toks }}
    }

    fn collect_props(&self) -> TokenStream2 {
        let name = &self.name;

        let mut toks = match &self.prop_gen_args {
            Some(gen_args) => quote_spanned! { name.span() => fc_to_builder(#name #gen_args) },
            None => quote_spanned! { name.span() => fc_to_builder(#name) },
        };
        for field in &self.fields {
            toks.append_all(quote! {#field})
        }
        if !self.children.is_empty() {
            let renderer = TemplateRenderer::as_tokens(&self.children, None);
            toks.append_all(quote_spanned! { name.span() => .children( #renderer ) });
        }
        toks.append_all(quote_spanned! { name.span() => .build() });
        toks
    }

    fn fn_name(&self) -> String {
        self.name.segments.last().unwrap().ident.to_string()
    }

    /// We can add hints for rust analyzer to provide better hints for the component fields.
    pub(crate) fn completion_hints(&self) -> TokenStream2 {
        // If we have fields, we can't add any completion hints because completing the builder like a struct will create issues with optional fields
        if !self.fields.is_empty() {
            return quote! {};
        }

        let name = &self.name;
        let braces = self
            .brace
            .as_ref()
            .map(|b| {
                let mut tokens = TokenStream2::new();
                b.surround(&mut tokens, |_| {});
                tokens
            })
            .unwrap_or_default();

        quote! {
            #[allow(dead_code)]
            {
                #name #braces
            };
        }
    }

    /// If this element is only partially complete, return the errors that occurred during parsing
    pub(crate) fn errors(&self) -> TokenStream2 {
        let Self { errors, .. } = self;

        let mut tokens = quote! {};
        for error in errors {
            tokens.append_all(error.to_compile_error());
        }

        tokens
    }
}

// the struct's fields info
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ComponentField {
    pub name: Ident,
    pub content: ContentField,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ContentField {
    Shorthand(Ident),
    ManExpr(Expr),
    Formatted(IfmtInput),
}

impl ContentField {
    fn new(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            let forked = input.fork();
            let t: LitStr = forked.parse()?;

            // the string literal must either be the end of the input or a followed by a comma
            let res =
                match (forked.is_empty() || forked.peek(Token![,])) && is_literal_foramtted(&t) {
                    true => ContentField::Formatted(input.parse()?),
                    false => ContentField::ManExpr(input.parse()?),
                };

            return Ok(res);
        }

        Ok(ContentField::ManExpr(input.parse()?))
    }
}

impl ToTokens for ContentField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ContentField::Shorthand(i) => tokens.append_all(quote! { #i }),
            ContentField::ManExpr(e) => e.to_tokens(tokens),
            ContentField::Formatted(s) => tokens.append_all(quote! {
                #s
            }),
        }
    }
}

impl Parse for ComponentField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = Ident::parse_any(input)?;

        // if the next token is not a colon, then it's a shorthand field
        if input.parse::<Token![:]>().is_err() {
            return Ok(Self {
                content: ContentField::Shorthand(name.clone()),
                name,
            });
        };

        let content = ContentField::new(input)?;

        if input.peek(LitStr) || input.peek(Ident) {
            missing_trailing_comma!(content.span());
        }

        Ok(Self { name, content })
    }
}

impl ToTokens for ComponentField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ComponentField { name, content, .. } = self;
        tokens.append_all(quote! { .#name(#content) })
    }
}

fn is_literal_foramtted(lit: &LitStr) -> bool {
    let s = lit.value();
    let mut chars = s.chars();

    while let Some(next) = chars.next() {
        if next == '{' {
            let nen = chars.next();
            if nen != Some('{') {
                return true;
            }
        }
    }

    false
}

fn normalize_path(name: &mut syn::Path) -> Option<AngleBracketedGenericArguments> {
    let seg = name.segments.last_mut()?;
    match seg.arguments.clone() {
        PathArguments::AngleBracketed(args) => {
            seg.arguments = PathArguments::None;
            Some(args)
        }
        _ => None,
    }
}

impl ComponentField {
    pub fn can_be_shorthand(&self) -> bool {
        // If it's a shorthand...
        if matches!(self.content, ContentField::Shorthand(_)) {
            return true;
        }

        // If it's in the form of attr: attr, return true
        if let ContentField::ManExpr(Expr::Path(path)) = &self.content {
            if path.path.segments.len() == 1 && path.path.segments[0].ident == self.name {
                return true;
            }
        }

        false
    }
}
