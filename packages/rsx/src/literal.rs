use proc_macro2::Span;
use quote::ToTokens;
use quote::{quote, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Lit, LitBool, LitFloat, LitInt,
};

use crate::{location::DynIdx, IfmtInput, Segment};

/// A literal value in the rsx! macro
///
/// These get hotreloading super powers, making them a bit more complex than a normal literal.
/// In debug mode we need to generate a bunch of extra code to support hotreloading.
///
/// Eventually we want to remove this notion of hot literals since we're generating different code
/// in debug than in release, which is harder to maintain and can lead to bugs.
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct HotLiteral {
    pub value: HotLiteralType,
    pub hr_idx: DynIdx,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum HotLiteralType {
    /// A *formatted* string literal
    /// We know this will generate a String, not an &'static str
    ///
    /// The raw str type will generate a &'static str, but we need to distinguish the two for component props
    ///
    /// "hello {world}"
    Fmted(IfmtInput),

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
            Lit::Int(a) => HotLiteralType::Int(a),
            Lit::Bool(a) => HotLiteralType::Bool(a),
            Lit::Float(a) => HotLiteralType::Float(a),
            Lit::Str(a) => HotLiteralType::Fmted(IfmtInput::new_litstr(a)),
            _ => {
                return Err(syn::Error::new(
                    raw.span(),
                    "Only string, int, float, and bool literals are supported",
                ))
            }
        };

        Ok(HotLiteral {
            value,
            hr_idx: DynIdx::default(),
        })
    }
}

impl ToTokens for HotLiteral {
    fn to_tokens(&self, out: &mut proc_macro2::TokenStream) {
        // let ty = match &self.value {
        //     HotLiteralType::Fmted(f) if f.is_static() => quote! { &'static str  },
        //     HotLiteralType::Fmted(_) => quote! { FmtedSegments },
        //     HotLiteralType::Float(_) => quote! { f64 },
        //     HotLiteralType::Int(_) => quote! { i128 },
        //     HotLiteralType::Bool(_) => quote! { bool },
        // };

        let val = match &self.value {
            HotLiteralType::Fmted(fmt) if fmt.is_static() => {
                let o = fmt.to_static().unwrap().to_token_stream();
                quote! { #o }
            }
            HotLiteralType::Fmted(fmt) => {
                let mut idx = 0_usize;
                let segments = fmt.segments.iter().map(|s| match s {
                    Segment::Literal(lit) => quote! {
                        FmtSegment::Literal { value: #lit }
                    },
                    Segment::Formatted(_fmt) => {
                        // increment idx for the dynamic segment so we maintain the mapping
                        let _idx = idx;
                        idx += 1;
                        quote! {
                            FmtSegment::Dynamic { id: #_idx }
                        }
                    }
                });

                quote! {
                    FmtedSegments::new(
                        // The static segments with idxs for locations
                        vec![ #(#segments),* ],
                    )
                }
            }
            HotLiteralType::Float(a) => quote! { #a },
            HotLiteralType::Int(a) => quote! { #a },
            HotLiteralType::Bool(a) => quote! { #a },
            // HotLiteralType::Float(a) => quote! { #a as f64 },
            // HotLiteralType::Int(a) => quote! { #a as i128 },
            // HotLiteralType::Bool(a) => quote! { #a as bool },
        };

        let mapped = match &self.value {
            HotLiteralType::Fmted(f) if f.is_static() => quote! { .clone() as &'static str},

            HotLiteralType::Fmted(segments) => {
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
            HotLiteralType::Float(_) => quote! { .clone() },
            HotLiteralType::Int(_) => quote! { .clone() },
            HotLiteralType::Bool(_) => quote! { .clone() },
        };

        let as_lit = match &self.value {
            HotLiteralType::Fmted(f) => f.to_token_stream(),
            HotLiteralType::Float(f) => f.to_token_stream(),
            HotLiteralType::Int(f) => f.to_token_stream(),
            HotLiteralType::Bool(f) => f.to_token_stream(),
        };

        let map_lit = match &self.value {
            HotLiteralType::Fmted(f) if f.is_static() => quote! { .clone() },
            HotLiteralType::Fmted(_) => quote! { .to_string() },
            HotLiteralType::Float(_) => quote! { .clone() },
            HotLiteralType::Int(_) => quote! { .clone() },
            HotLiteralType::Bool(_) => quote! { .clone() },
        };

        let hr_idx = self.hr_idx.get().to_string();

        out.append_all(quote! {
            {
                #[cfg(debug_assertions)]
                {

                    // in debug we still want these tokens to turn into fmt args such that RA can line
                    // them up, giving us rename powersa
                    _ = #as_lit;
                    GlobalSignal::with_key(
                        || #val, {
                        concat!(
                            file!(),
                            ":",
                            line!(),
                            ":",
                            column!(),
                            ":",
                            #hr_idx
                        )
                    })
                    .with(|s| s #mapped)
                }

                // just render the literal directly
                #[cfg(not(debug_assertions))]
                { #as_lit #map_lit }
            }
        })
    }
}

impl HotLiteralType {
    fn span(&self) -> Span {
        match self {
            HotLiteralType::Fmted(f) => f.span(),
            HotLiteralType::Float(f) => f.span(),
            HotLiteralType::Int(f) => f.span(),
            HotLiteralType::Bool(f) => f.span(),
        }
    }
}

impl HotLiteral {
    // We can only handle a few types of literals - the rest need to be expressions
    // todo on adding more of course - they're not hard to support, just work
    pub fn peek(input: ParseStream) -> bool {
        if input.peek(Lit) {
            let lit = input.fork().parse::<Lit>().unwrap();

            match lit {
                Lit::Str(_) | Lit::Int(_) | Lit::Float(_) | Lit::Bool(_) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn is_static(&self) -> bool {
        match &self.value {
            HotLiteralType::Fmted(fmt) => fmt.is_static(),
            _ => false,
        }
    }

    pub fn span(&self) -> Span {
        self.value.span()
    }

    pub fn to_string(&self) -> String {
        match &self.value {
            HotLiteralType::Fmted(f) => f.to_quoted_string_from_parts(),
            HotLiteralType::Float(f) => f.to_string(),
            HotLiteralType::Int(f) => f.to_string(),
            HotLiteralType::Bool(f) => f.value().to_string(),
        }
    }
}

#[cfg(test)]
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
    assert!(matches!(lit.value, HotLiteralType::Fmted(_)));

    let lit = syn::parse2::<HotLiteral>(quote! { "hello {world}" }).unwrap();
    assert!(matches!(lit.value, HotLiteralType::Fmted(_)));
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
    let HotLiteralType::Fmted(segments) = &lit.value else {
        panic!("expected a formatted string");
    };
    assert!(segments.is_static());
    println!("{}", lit.to_token_stream().pretty_unparse());
}

#[test]
fn formatted_prints_as_formatted() {
    let lit = syn::parse2::<HotLiteral>(quote! { "hello {world}" }).unwrap();
    let HotLiteralType::Fmted(segments) = &lit.value else {
        panic!("expected a formatted string");
    };
    assert!(!segments.is_static());
    println!("{}", lit.to_token_stream().pretty_unparse());

    println!("{}", segments.to_quoted_string_from_parts());
}
