use dioxus_rsx::CallBody;
use proc_macro2::TokenStream as TokenStream2;

#[test]
fn write_block_out() {
    let src = include_str!("./srcless/basic_expr.rsx");

    let tokens: TokenStream2 = syn::parse_str(src).unwrap();
    let parsed: CallBody = syn::parse2(tokens).unwrap();

    let block = dioxus_autofmt::write_block_out(&parsed).unwrap();

    pretty_assertions::assert_eq!(block.trim(), src.trim());
}
