use html_parser::Dom;

#[test]
fn h_tags_translate() {
    let html = r#"
    <div>
        <h1>hello world!</h1>
        <h2>hello world!</h2>
        <h3>hello world!</h3>
        <h4>hello world!</h4>
        <h5>hello world!</h5>
        <h6>hello world!</h6>
    </div>
    "#
    .trim();

    let dom = Dom::parse(html).unwrap();

    let body = rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(body).unwrap();

    let expected = r#"
    div {
        h1 { "hello world!" }
        h2 { "hello world!" }
        h3 { "hello world!" }
        h4 { "hello world!" }
        h5 { "hello world!" }
        h6 { "hello world!" }
    }"#;
    pretty_assertions::assert_eq!(&out, &expected);
}
