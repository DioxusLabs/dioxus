use html_parser::Dom;

#[test]
fn simple_elements() {
    let html = r#"
    <div>
        <div class="asd">hello world!</div>
        <div id="asd">hello world!</div>
        <div id="asd">hello world!</div>
        <div for="asd">hello world!</div>
        <div async="asd">hello world!</div>
    </div>
    "#
    .trim();

    let dom = Dom::parse(html).unwrap();

    let body = rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(body).unwrap();

    let expected = r#"
    div {
        div { class: "asd", "hello world!" }
        div { id: "asd", "hello world!" }
        div { id: "asd", "hello world!" }
        div { r#for: "asd", "hello world!" }
        div { r#async: "asd", "hello world!" }
    }"#;
    pretty_assertions::assert_eq!(&out, &expected);
}

#[test]
fn deeply_nested() {
    let html = r#"
    <div>
        <div class="asd">
            <div class="asd">
                <div class="asd">
                    <div class="asd">
                    </div>
                </div>
            </div>
        </div>
    </div>
    "#
    .trim();

    let dom = Dom::parse(html).unwrap();

    let body = rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(body).unwrap();

    let expected = r#"
    div {
        div { class: "asd",
            div { class: "asd",
                div { class: "asd", div { class: "asd" } }
            }
        }
    }"#;
    pretty_assertions::assert_eq!(&out, &expected);
}
