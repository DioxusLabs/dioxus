pub(crate) fn try_parse_braces<'a>(
    input: &syn::parse::ParseBuffer<'a>,
) -> syn::Result<(syn::token::Brace, syn::parse::ParseBuffer<'a>)> {
    let content;
    let brace = syn::braced!(content in input);
    Ok((brace, content))
}
