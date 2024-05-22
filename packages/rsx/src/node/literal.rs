use proc_macro2::Span;
use quote::ToTokens;
use quote::{quote, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Lit, LitBool, LitFloat, LitInt, LitStr,
};

use crate::{location::CallerLocation, IfmtInput, Segment};

/// A literal value in the rsx! macro
///
/// These get hotreloading super powers, making them a bit more complex than a normal literal.
/// In debug mode we need to generate a bunch of extra code to support hotreloading.
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct RsxLiteral {
    pub raw: Lit,
    pub value: HotLiteral,
    pub hr_idx: CallerLocation,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum HotLiteral {
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

impl Parse for RsxLiteral {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // If it's a string lit we need to parse it as an ifmt input
        if input.peek(LitStr) {
            let ifmt_input: IfmtInput = input.parse()?;
            let raw = ifmt_input.source.clone().unwrap();
            let value = HotLiteral::Fmted(ifmt_input);

            return Ok(RsxLiteral {
                raw: Lit::Str(raw),
                value,
                hr_idx: CallerLocation::default(),
            });
        } else {
            let raw = input.parse::<Lit>()?;
            let value = match raw.clone() {
                Lit::Int(a) => HotLiteral::Int(a),
                Lit::Float(a) => HotLiteral::Float(a),
                Lit::Bool(a) => HotLiteral::Bool(a),
                _ => {
                    return Err(syn::Error::new(
                        raw.span(),
                        "Only string, int, float, and bool literals are supported",
                    ))
                }
            };

            Ok(RsxLiteral {
                raw,
                value,
                hr_idx: CallerLocation::default(),
            })
        }
    }
}

impl ToTokens for RsxLiteral {
    fn to_tokens(&self, out: &mut proc_macro2::TokenStream) {
        let ty = match &self.value {
            HotLiteral::Fmted(f) if f.is_static() => quote! { &'static str  },
            HotLiteral::Fmted(_) => quote! { FmtedSegments },
            HotLiteral::Float(_) => quote! { f64 },
            HotLiteral::Int(_) => quote! { i64 },
            HotLiteral::Bool(_) => quote! { bool },
        };

        let val = match &self.value {
            HotLiteral::Fmted(fmt) => {
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
            HotLiteral::Float(a) => quote! { #a },
            HotLiteral::Int(a) => quote! { #a },
            HotLiteral::Bool(a) => quote! { #a },
        };

        let mapped = match &self.value {
            HotLiteral::Fmted(f) if f.is_static() => quote! { .into() },

            HotLiteral::Fmted(segments) => {
                let rendered_segments = segments.segments.iter().filter_map(|s| match s {
                    Segment::Literal(_lit) => None,
                    Segment::Formatted(fmt) => {
                        // just render as a format_args! call
                        Some(quote! {
                            #fmt.to_string()
                        })
                    }
                });

                quote! {
                    .render_with(vec![ #(#rendered_segments),* ])
                }
            }
            HotLiteral::Float(_) => quote! { .into() },
            HotLiteral::Int(_) => quote! { .into() },
            HotLiteral::Bool(_) => quote! { .into() },
        };

        let as_lit = match &self.value {
            HotLiteral::Fmted(f) => f.to_token_stream(),
            HotLiteral::Float(f) => f.to_token_stream(),
            HotLiteral::Int(f) => f.to_token_stream(),
            HotLiteral::Bool(f) => f.to_token_stream(),
        };

        let map_lit = match &self.value {
            HotLiteral::Fmted(f) if f.is_static() => quote! { .into() },
            HotLiteral::Fmted(_) => quote! { .to_string() },
            HotLiteral::Float(_) => quote! { .into() },
            HotLiteral::Int(_) => quote! { .into() },
            HotLiteral::Bool(_) => quote! { .into() },
        };

        let hr_idx = self.hr_idx.get().to_string();

        out.append_all(quote! {
            {
                #[cfg(debug_assertions)]
                {
                    static __SIGNAL: GlobalSignal<#ty> = GlobalSignal::with_key(|| #val, {
                        concat!(
                            file!(),
                            ":",
                            line!(),
                            ":",
                            column!(),
                            ":",
                            #hr_idx
                        )
                    });

                    // render the signal and subscribe the component to its changes
                    __SIGNAL.with(|s|  s #mapped)
                }

                // just render the literal directly
                #[cfg(not(debug_assertions))]
                { #as_lit #map_lit }
            }
        })
    }
}

impl RsxLiteral {
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
            HotLiteral::Fmted(fmt) => fmt.is_static(),
            _ => false,
        }
    }

    pub fn span(&self) -> Span {
        self.raw.span()
    }
}

#[cfg(test)]
use crate::PrettyUnparse;

#[test]
fn parses_lits() {
    let _ = syn::parse2::<RsxLiteral>(quote! { "hello" }).unwrap();
    let _ = syn::parse2::<RsxLiteral>(quote! { "hello {world}" }).unwrap();
    let _ = syn::parse2::<RsxLiteral>(quote! { 1 }).unwrap();
    let _ = syn::parse2::<RsxLiteral>(quote! { 1.0 }).unwrap();
    let _ = syn::parse2::<RsxLiteral>(quote! { false }).unwrap();
    let _ = syn::parse2::<RsxLiteral>(quote! { true }).unwrap();

    // Refuses the other unsupported types - we could add them if we wanted to
    assert!(syn::parse2::<RsxLiteral>(quote! { b"123" }).is_err());
    assert!(syn::parse2::<RsxLiteral>(quote! { 'a' }).is_err());

    let lit = syn::parse2::<RsxLiteral>(quote! { "hello" }).unwrap();
    assert!(matches!(lit.value, HotLiteral::Fmted(_)));

    let lit = syn::parse2::<RsxLiteral>(quote! { "hello {world}" }).unwrap();
    assert!(matches!(lit.value, HotLiteral::Fmted(_)));
}

#[test]
fn outputs_a_signal() {
    // Should output a type of f64 which we convert into whatever the expected type is via "into"
    // todo: hmmmmmmmmmmmm might not always work
    let lit = syn::parse2::<RsxLiteral>(quote! { 1.0 }).unwrap();
    println!("{}", lit.to_token_stream().pretty_unparse());

    let lit = syn::parse2::<RsxLiteral>(quote! { "hi" }).unwrap();
    println!("{}", lit.to_token_stream().pretty_unparse());

    let lit = syn::parse2::<RsxLiteral>(quote! { "hi {world}" }).unwrap();
    println!("{}", lit.to_token_stream().pretty_unparse());
}
