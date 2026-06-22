//! Fill-order regression for the runtime dynamic-literal pool.
//!
//! The hot-reload differ resolves a formatted value's `FmtSegment::Dynamic { id }` against the
//! canonical fill order (attributes, then key, then children) recorded in the last build. The
//! runtime `DynamicLiteralPool` that those ids index into is produced separately by the rsx
//! `ViewBuilder`. If the two disagree, a hot reload feeds one slot's text into another slot.
//!
//! Specifically, an element with both a formatted `key` and a formatted child text node must pool
//! the key's segments BEFORE the children's. Visiting children first would pool `[child, key]`
//! while the differ expects `[key, child]`, transposing the two after a hot reload.

use dioxus_rsx::CallBody;
use quote::ToTokens;

/// The runtime pool literal `vec!` for `div { key: "{key_val}", "child-{text_val}" }` must list the
/// key's dynamic segment before the child text's, matching the canonical attrs -> key -> children
/// fill order the hot-reload differ assumes.
#[test]
fn key_segments_pooled_before_child_text() {
    let body: CallBody =
        syn::parse_str(r#"div { key: "{key_val}", "child-{text_val}" }"#).expect("parse rsx");
    let tokens = body.to_token_stream().to_string();

    // The pool is only emitted under debug_assertions (tests build in debug mode).
    let pool_start = tokens
        .find("DynamicLiteralPool")
        .expect("debug build emits a DynamicLiteralPool");
    let pool = &tokens[pool_start..];

    let key_pos = pool.find("key_val").expect("key segment is pooled");
    let text_pos = pool.find("text_val").expect("child text segment is pooled");

    assert!(
        key_pos < text_pos,
        "key segment must be pooled before child text to stay in lockstep with the hot-reload \
         fill order; got pool fragment: {}",
        &pool[..text_pos.max(key_pos) + "text_val".len()]
    );
}
