use dioxus_rsx::CallBody;
use syn::parse_quote;

macro_rules! test_case {
    (
        $path:literal
    ) => {
        works(include!($path), include_str!($path))
    };
}

/// Ensure we can write RSX blocks without a source file
///
/// Useful in code generation use cases where we still want formatted code.
#[test]
fn write_block_out() {
    test_case!("./srcless/basic_expr.rsx");
    test_case!("./srcless/asset.rsx");
}

fn works(parsed: CallBody, src: &str) {
    let block = dioxus_autofmt::write_block_out(&parsed).unwrap();
    let src = src
        .trim()
        .trim_start_matches("parse_quote! {")
        .trim_end_matches('}');

    // normalize line endings for windows tests to pass
    pretty_assertions::assert_eq!(
        block.trim().lines().collect::<Vec<_>>().join("\n"),
        src.trim().lines().collect::<Vec<_>>().join("\n")
    );
}
