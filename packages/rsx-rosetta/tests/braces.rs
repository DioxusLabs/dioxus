use html_parser::Dom;

// Regression test for https://github.com/DioxusLabs/dioxus/issues/3352
// Literal `{`/`}` (e.g. inside CSS) must be escaped so the generated `rsx!` doesn't try to
// interpolate them as format arguments when it is recompiled.
#[test]
fn braces_in_text_are_escaped() {
    let html = r#"<style>body { color: red; }</style>"#;

    let dom = Dom::parse(html).unwrap();

    let body = dioxus_rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(&body).unwrap();

    let expected = r#"
    style { "body {{ color: red; }}" }"#;
    pretty_assertions::assert_eq!(&out, &expected);
}
