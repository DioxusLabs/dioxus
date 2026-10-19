use html_parser::Dom;

// Regression test for https://github.com/DioxusLabs/dioxus/issues/3352
// `<html>` is the document root and has no RSX equivalent (`html` is the element vocabulary root
// in Dioxus), so its children are spliced in as siblings.
#[test]
fn document_html_is_unwrapped() {
    let html = r#"<html><head><title>Hello</title></head><body><p>hi</p></body></html>"#;

    let dom = Dom::parse(html).unwrap();

    let body = dioxus_rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(&body).unwrap();

    let expected = r#"
    head {
        title { "Hello" }
    }
    body {
        p { "hi" }
    }"#;
    pretty_assertions::assert_eq!(&out, &expected);
}

// `xmlns` is a global attribute, so it must be accepted on non-SVG roots like MathML's `<math>`.
#[test]
fn mathml_xmlns_is_a_valid_attribute() {
    let html = r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi></math>"#;

    let dom = Dom::parse(html).unwrap();

    let body = dioxus_rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(&body).unwrap();

    let expected = r#"
    math { xmlns: "http://www.w3.org/1998/Math/MathML",
        mi { "x" }
    }"#;
    pretty_assertions::assert_eq!(&out, &expected);
}
