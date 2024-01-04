use html_parser::Dom;

#[test]
fn web_components_translate() {
    let html = r#"
    <div>
       <my-component></my-component>
    </div>
    "#
    .trim();

    let dom = Dom::parse(html).unwrap();

    let body = rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(body).unwrap();

    let expected = r#"
    div { my-component {} }"#;
    pretty_assertions::assert_eq!(&out, &expected);
}
