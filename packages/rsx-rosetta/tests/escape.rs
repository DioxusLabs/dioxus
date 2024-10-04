use html_parser::Dom;

// Regression test for https://github.com/DioxusLabs/dioxus/issues/3037
// We need to escape html entities as we translate html because rsx doesn't support them
#[test]
fn escaped_text() {
    let html = r#"<div>&lt;div&gt;&#x231b;&#x231b;&#x231b;&#x231b;</div>"#.trim();

    let dom = Dom::parse(html).unwrap();

    let body = dioxus_rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(&body).unwrap();

    let expected = r#"
    div { "<div>⌛⌛⌛⌛" }"#;
    pretty_assertions::assert_eq!(&out, &expected);
}
