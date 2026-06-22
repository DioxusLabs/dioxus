use dioxus_rsx::*;
use quote::ToTokens;

#[test]
fn dynamic_text_pool_order() {
    let body: CallBody =
        syn::parse_str(r#"div { key: "{key_val}", "child-{text_val}" }"#).unwrap();
    let ts = body.to_token_stream().to_string();
    let pos = ts.find("DynamicLiteralPool").expect("pool present");
    let key_pos = ts[pos..].find("key_val").map(|p| p + pos);
    let text_pos = ts[pos..].find("text_val").map(|p| p + pos);
    eprintln!("key_val at {key_pos:?}, text_val at {text_pos:?}");
    // After the fix, key_val must appear BEFORE text_val in the runtime pool.
    assert!(key_pos.unwrap() < text_pos.unwrap(), "key must be pooled before child text");
}
