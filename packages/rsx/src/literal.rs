use proc_macro2::Span;
use quote::ToTokens;
use quote::{quote, TokenStreamExt};
use std::fmt::Display;
use std::ops::Deref;
use syn::{
    parse::{Parse, ParseStream},
    Lit, LitBool, LitFloat, LitInt, LitStr,
};

use crate::{location::DynIdx, IfmtInput, Segment};
use proc_macro2::TokenStream as TokenStream2;

/// A literal value in the rsx! macro
///
/// These get hotreloading super powers, making them a bit more complex than a normal literal.
/// In debug mode we need to generate a bunch of extra code to support hotreloading.
///
/// Eventually we want to remove this notion of hot literals since we're generating different code
/// in debug than in release, which is harder to maintain and can lead to bugs.
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum HotLiteral {
    /// A *formatted* string literal
    /// We know this will generate a String, not an &'static str
    ///
    /// The raw str type will generate a &'static str, but we need to distinguish the two for component props
    ///
    /// "hello {world}"
    Fmted(HotReloadFormattedSegment),

    /// A float literal
    ///
    /// 1.0
    Float(LitFloat),

    /// An int literal
    ///
    /// 1
    Int(LitInt),

    /// A bool literal
    ///
    /// true
    Bool(LitBool),
}

impl Parse for HotLiteral {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let raw = input.parse::<Lit>()?;

        let value = match raw.clone() {
            Lit::Int(a) => HotLiteral::Int(a),
            Lit::Bool(a) => HotLiteral::Bool(a),
            Lit::Float(a) => HotLiteral::Float(a),
            Lit::Str(a) => HotLiteral::Fmted(IfmtInput::new_litstr(a).into()),
            _ => {
                return Err(syn::Error::new(
                    raw.span(),
                    "Only string, int, float, and bool literals are supported",
                ))
            }
        };

        Ok(value)
    }
}

impl ToTokens for HotLiteral {
    fn to_tokens(&self, out: &mut proc_macro2::TokenStream) {
        let val = match &self {
            HotLiteral::Fmted(fmt) if fmt.is_static() => {
                let o = fmt.to_static().unwrap().to_token_stream();
                quote! { #o }
            }

            HotLiteral::Fmted(fmt) => {
                let mut idx = 0_usize;
                let segments = fmt.segments.iter().map(|s| match s {
                    Segment::Literal(lit) => quote! {
                        dioxus_core::internal::FmtSegment::Literal { value: #lit }
                    },
                    Segment::Formatted(_fmt) => {
                        // increment idx for the dynamic segment so we maintain the mapping
                        let _idx = idx;
                        idx += 1;
                        quote! {
                           dioxus_core::internal::FmtSegment::Dynamic { id: #_idx }
                        }
                    }
                });

                // The static segments with idxs for locations
                quote! {
                    dioxus_core::internal::FmtedSegments::new( vec![ #(#segments),* ], )
                }
            }
            HotLiteral::Float(a) => quote! { #a },
            HotLiteral::Int(a) => quote! { #a },
            HotLiteral::Bool(a) => quote! { #a },
        };

        let mapped = match &self {
            HotLiteral::Fmted(f) if f.is_static() => quote! { .clone() as &'static str},

            HotLiteral::Fmted(segments) => {
                let rendered_segments = segments.segments.iter().filter_map(|s| match s {
                    Segment::Literal(_lit) => None,
                    Segment::Formatted(fmt) => {
                        // just render as a format_args! call
                        Some(quote! { #fmt.to_string() })
                    }
                });

                quote! {
                    .render_with(vec![ #(#rendered_segments),* ])
                }
            }
            HotLiteral::Float(_) => quote! { .clone() },
            HotLiteral::Int(_) => quote! { .clone() },
            HotLiteral::Bool(_) => quote! { .clone() },
        };

        let as_lit = match &self {
            HotLiteral::Fmted(f) if f.is_static() => {
                let r = f.to_static().unwrap();
                quote! { #r }
            }
            HotLiteral::Fmted(f) => f.to_token_stream(),
            HotLiteral::Float(f) => f.to_token_stream(),
            HotLiteral::Int(f) => f.to_token_stream(),
            HotLiteral::Bool(f) => f.to_token_stream(),
        };

        let map_lit = match &self {
            HotLiteral::Fmted(f) if f.is_static() => quote! { .clone() },
            HotLiteral::Fmted(_) => quote! { .to_string() },
            HotLiteral::Float(_) => quote! { .clone() },
            HotLiteral::Int(_) => quote! { .clone() },
            HotLiteral::Bool(_) => quote! { .clone() },
        };

        let hr_idx = self.hr_idx.get().to_string();

        out.append_all(quote! {
            {
                #[cfg(debug_assertions)]
                {
                    // in debug we still want these tokens to turn into fmt args such that RA can line
                    // them up, giving us rename powers
                    _ = #as_lit;

                    // The key is important here - we're creating a new GlobalSignal each call to this/
                    // But the key is what's keeping it stable
                    GlobalSignal::with_key(
                        || #val, {
                        {
                            const PATH: &str = dioxus_core::const_format::str_replace!(file!(), "\\\\", "/");
                            const NORMAL: &str = dioxus_core::const_format::str_replace!(PATH, '\\', "/");
                            dioxus_core::const_format::concatcp!(NORMAL, ':', line!(), ':', column!(), ':', #hr_idx)
                        }
                    })
                    .maybe_with_rt(|s| s #mapped)
                }

                // just render the literal directly
                #[cfg(not(debug_assertions))]
                { #as_lit #map_lit }
            }
        })
    }
}

impl HotLiteral {
    pub fn span(&self) -> Span {
        match self {
            HotLiteral::Fmted(f) => f.span(),
            HotLiteral::Float(f) => f.span(),
            HotLiteral::Int(f) => f.span(),
            HotLiteral::Bool(f) => f.span(),
        }
    }
}

impl HotLiteral {
    // We can only handle a few types of literals - the rest need to be expressions
    // todo on adding more of course - they're not hard to support, just work
    pub fn peek(input: ParseStream) -> bool {
        if input.peek(Lit) {
            let lit = input.fork().parse::<Lit>().unwrap();

            matches!(
                lit,
                Lit::Str(_) | Lit::Int(_) | Lit::Float(_) | Lit::Bool(_)
            )
        } else {
            false
        }
    }

    pub fn is_static(&self) -> bool {
        match &self {
            HotLiteral::Fmted(fmt) => fmt.is_static(),
            _ => false,
        }
    }

    pub fn from_raw_text(text: &str) -> Self {
        HotLiteral::Fmted(HotReloadFormattedSegment::from(IfmtInput {
            source: LitStr::new(text, Span::call_site()),
            segments: vec![],
        }))
    }
}

impl Display for HotLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            HotLiteral::Fmted(l) => l.to_string_with_quotes().fmt(f),
            HotLiteral::Float(l) => l.fmt(f),
            HotLiteral::Int(l) => l.fmt(f),
            HotLiteral::Bool(l) => l.value().fmt(f),
        }
    }
}

/// A formatted segment that can be hot reloaded
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct HotReloadFormattedSegment {
    pub formatted_input: IfmtInput,
    pub dynamic_node_indexes: Vec<DynIdx>,
}

impl Deref for HotReloadFormattedSegment {
    type Target = IfmtInput;

    fn deref(&self) -> &Self::Target {
        &self.formatted_input
    }
}

impl From<IfmtInput> for HotReloadFormattedSegment {
    fn from(input: IfmtInput) -> Self {
        let mut dynamic_node_indexes = Vec::new();
        for segment in &input.segments {
            if let Segment::Formatted { .. } = segment {
                dynamic_node_indexes.push(DynIdx::default());
            }
        }
        Self {
            formatted_input: input,
            dynamic_node_indexes,
        }
    }
}

impl Parse for HotReloadFormattedSegment {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ifmt: IfmtInput = input.parse()?;
        Ok(Self::from(ifmt))
    }
}

impl ToTokens for HotReloadFormattedSegment {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let mut idx = 0_usize;
        let segments = self.segments.iter().map(|s| match s {
            Segment::Literal(lit) => quote! {
                dioxus_core::internal::FmtSegment::Literal { value: #lit }
            },
            Segment::Formatted(_fmt) => {
                // increment idx for the dynamic segment so we maintain the mapping
                let _idx = self.dynamic_node_indexes[idx].get();
                idx += 1;
                quote! {
                   dioxus_core::internal::FmtSegment::Dynamic { id: #_idx }
                }
            }
        });

        // The static segments with idxs for locations
        tokens.extend(quote! {
            dioxus_core::internal::FmtedSegments::new( vec![ #(#segments),* ], )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrettyUnparse;

    #[test]
    fn parses_lits() {
        let _ = syn::parse2::<HotLiteral>(quote! { "hello" }).unwrap();
        let _ = syn::parse2::<HotLiteral>(quote! { "hello {world}" }).unwrap();
        let _ = syn::parse2::<HotLiteral>(quote! { 1 }).unwrap();
        let _ = syn::parse2::<HotLiteral>(quote! { 1.0 }).unwrap();
        let _ = syn::parse2::<HotLiteral>(quote! { false }).unwrap();
        let _ = syn::parse2::<HotLiteral>(quote! { true }).unwrap();

        // Refuses the other unsupported types - we could add them if we wanted to
        assert!(syn::parse2::<HotLiteral>(quote! { b"123" }).is_err());
        assert!(syn::parse2::<HotLiteral>(quote! { 'a' }).is_err());

        let lit = syn::parse2::<HotLiteral>(quote! { "hello" }).unwrap();
        assert!(matches!(lit.value, HotLiteral::Fmted(_)));

        let lit = syn::parse2::<HotLiteral>(quote! { "hello {world}" }).unwrap();
        assert!(matches!(lit.value, HotLiteral::Fmted(_)));
    }

    #[test]
    fn outputs_a_signal() {
        // Should output a type of f64 which we convert into whatever the expected type is via "into"
        // todo: hmmmmmmmmmmmm might not always work
        let lit = syn::parse2::<HotLiteral>(quote! { 1.0 }).unwrap();
        println!("{}", lit.to_token_stream().pretty_unparse());

        let lit = syn::parse2::<HotLiteral>(quote! { "hi" }).unwrap();
        println!("{}", lit.to_token_stream().pretty_unparse());

        let lit = syn::parse2::<HotLiteral>(quote! { "hi {world}" }).unwrap();
        println!("{}", lit.to_token_stream().pretty_unparse());
    }

    #[test]
    fn static_str_becomes_str() {
        let lit = syn::parse2::<HotLiteral>(quote! { "hello" }).unwrap();
        let HotLiteral::Fmted(segments) = &lit.value else {
            panic!("expected a formatted string");
        };
        assert!(segments.is_static());
        assert_eq!(r##""hello""##, segments.to_string_with_quotes());
        println!("{}", lit.to_token_stream().pretty_unparse());
    }

    #[test]
    fn formatted_prints_as_formatted() {
        let lit = syn::parse2::<HotLiteral>(quote! { "hello {world}" }).unwrap();
        let HotLiteral::Fmted(segments) = &lit.value else {
            panic!("expected a formatted string");
        };
        assert!(!segments.is_static());
        assert_eq!(r##""hello {world}""##, segments.to_string_with_quotes());
        println!("{}", lit.to_token_stream().pretty_unparse());
    }
}
