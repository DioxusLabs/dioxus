use html_parser::Dom;

#[test]
fn svgs() {
    let viewbox = dioxus_html::map_html_attribute_to_rsx("viewBox");
    println!("viewbox: {viewbox:?}");

    let html = r###"
<svg xmlns="http://www.w3.org/2000/svg" id="flag-icons-fr" viewBox="0 0 640 480">
  <path fill="#fff" d="M0 0h640v480H0z"/>
  <path fill="#000091" d="M0 0h213.3v480H0z"/>
  <path fill="#e1000f" d="M426.7 0H640v480H426.7z"/>
</svg>
"###;

    let dom = Dom::parse(html).unwrap();

    let body = dioxus_rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(&body).unwrap();
    pretty_assertions::assert_eq!(
        &out,
        r##"
    svg {
        id: "flag-icons-fr",
        view_box: "0 0 640 480",
        xmlns: "http://www.w3.org/2000/svg",
        path { d: "M0 0h640v480H0z", fill: "#fff" }
        path { d: "M0 0h213.3v480H0z", fill: "#000091" }
        path { d: "M426.7 0H640v480H426.7z", fill: "#e1000f" }
    }"##
    );
}
