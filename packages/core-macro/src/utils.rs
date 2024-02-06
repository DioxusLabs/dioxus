use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_quote, Expr, Lit, Meta, Token, Type};

const FORMATTED_TYPE_START: &str = "static TY_AFTER_HERE:";
const FORMATTED_TYPE_END: &str = "= unreachable!();";

/// Attempts to convert the given literal to a string.
/// Converts ints and floats to their base 10 counterparts.
///
/// Returns `None` if the literal is [`Lit::Verbatim`] or if the literal is [`Lit::ByteStr`]
/// and the byte string could not be converted to UTF-8.
pub fn lit_to_string(lit: Lit) -> Option<String> {
    match lit {
        Lit::Str(l) => Some(l.value()),
        Lit::ByteStr(l) => String::from_utf8(l.value()).ok(),
        Lit::Byte(l) => Some(String::from(l.value() as char)),
        Lit::Char(l) => Some(l.value().to_string()),
        Lit::Int(l) => Some(l.base10_digits().to_string()),
        Lit::Float(l) => Some(l.base10_digits().to_string()),
        Lit::Bool(l) => Some(l.value().to_string()),
        Lit::Verbatim(_) => None,
        _ => None,
    }
}

pub fn format_type_string(ty: &Type) -> String {
    let ty_unformatted = ty.into_token_stream().to_string();
    let ty_unformatted = ty_unformatted.trim();

    // This should always be valid syntax.
    // Not Rust code, but syntax, which is the only thing that `syn` cares about.
    let Ok(file_unformatted) = syn::parse_file(&format!(
        "{FORMATTED_TYPE_START}{ty_unformatted}{FORMATTED_TYPE_END}"
    )) else {
        return ty_unformatted.to_string();
    };

    let file_formatted = prettyplease::unparse(&file_unformatted);

    let file_trimmed = file_formatted.trim();
    let start_removed = file_trimmed.trim_start_matches(FORMATTED_TYPE_START);
    let end_removed = start_removed.trim_end_matches(FORMATTED_TYPE_END);
    let ty_formatted = end_removed.trim();

    ty_formatted.to_string()
}

/// Represents the `#[deprecated]` attribute.
///
/// You can use the [`DeprecatedAttribute::from_meta`] function to try to parse an attribute to this struct.
#[derive(Default)]
pub struct DeprecatedAttribute {
    pub since: Option<String>,
    pub note: Option<String>,
}

impl DeprecatedAttribute {
    /// Returns `None` if the given attribute was not a valid form of the `#[deprecated]` attribute.
    pub fn from_meta(meta: &Meta) -> syn::Result<Self> {
        if meta.path() != &parse_quote!(deprecated) {
            return Err(syn::Error::new(
                meta.span(),
                "attribute path is not `deprecated`",
            ));
        }

        match &meta {
            Meta::Path(_) => Ok(Self::default()),
            Meta::NameValue(name_value) => {
                let Expr::Lit(expr_lit) = &name_value.value else {
                    return Err(syn::Error::new(
                        name_value.span(),
                        "literal in `deprecated` value must be a string",
                    ));
                };

                Ok(Self {
                    since: None,
                    note: lit_to_string(expr_lit.lit.clone()).map(|s| s.trim().to_string()),
                })
            }
            Meta::List(list) => {
                let parsed = list.parse_args::<DeprecatedAttributeArgsParser>()?;

                Ok(Self {
                    since: parsed.since.map(|s| s.trim().to_string()),
                    note: parsed.note.map(|s| s.trim().to_string()),
                })
            }
        }
    }
}

mod kw {
    use syn::custom_keyword;
    custom_keyword!(since);
    custom_keyword!(note);
}

struct DeprecatedAttributeArgsParser {
    since: Option<String>,
    note: Option<String>,
}

impl Parse for DeprecatedAttributeArgsParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut since: Option<String> = None;
        let mut note: Option<String> = None;

        if input.peek(kw::since) {
            input.parse::<kw::since>()?;
            input.parse::<Token![=]>()?;

            since = lit_to_string(input.parse()?);
        }

        if input.peek(Token![,]) && input.peek2(kw::note) {
            input.parse::<Token![,]>()?;
            input.parse::<kw::note>()?;
            input.parse::<Token![=]>()?;

            note = lit_to_string(input.parse()?);
        }

        Ok(Self { since, note })
    }
}
