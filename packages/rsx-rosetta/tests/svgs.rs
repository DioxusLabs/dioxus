use html_parser::Dom;

#[test]
fn svgs() {
    let viewbox = dioxus_html::map_html_attribute_to_rsx("viewBox");
    assert_eq!(viewbox, Some("view_box"));

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

    let html = r###"
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" id="flag-icons-cn" viewBox="0 0 640 480">
  <defs>
    <path id="cn-a" fill="#ff0" d="M-.6.8 0-1 .6.8-1-.3h2z"/>
  </defs>
  <path fill="#ee1c25" d="M0 0h640v480H0z"/>
  <use xlink:href="#cn-a" width="30" height="20" transform="matrix(71.9991 0 0 72 120 120)"/>
  <use xlink:href="#cn-a" width="30" height="20" transform="matrix(-12.33562 -20.5871 20.58684 -12.33577 240.3 48)"/>
  <use xlink:href="#cn-a" width="30" height="20" transform="matrix(-3.38573 -23.75998 23.75968 -3.38578 288 95.8)"/>
  <use xlink:href="#cn-a" width="30" height="20" transform="matrix(6.5991 -23.0749 23.0746 6.59919 288 168)"/>
  <use xlink:href="#cn-a" width="30" height="20" transform="matrix(14.9991 -18.73557 18.73533 14.99929 240 216)"/>
</svg>
    "###;

    let dom = Dom::parse(html).unwrap();

    let body = dioxus_rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(&body).unwrap();
    pretty_assertions::assert_eq!(
        &out,
        r##"
    svg {
        id: "flag-icons-cn",
        view_box: "0 0 640 480",
        "xlink": "http://www.w3.org/1999/xlink",
        xmlns: "http://www.w3.org/2000/svg",
        defs {
            path { d: "M-.6.8 0-1 .6.8-1-.3h2z", fill: "#ff0", id: "cn-a" }
        }
        path { d: "M0 0h640v480H0z", fill: "#ee1c25" }
        use {
            height: "20",
            href: "#cn-a",
            transform: "matrix(71.9991 0 0 72 120 120)",
            width: "30",
        }
        use {
            height: "20",
            href: "#cn-a",
            transform: "matrix(-12.33562 -20.5871 20.58684 -12.33577 240.3 48)",
            width: "30",
        }
        use {
            height: "20",
            href: "#cn-a",
            transform: "matrix(-3.38573 -23.75998 23.75968 -3.38578 288 95.8)",
            width: "30",
        }
        use {
            height: "20",
            href: "#cn-a",
            transform: "matrix(6.5991 -23.0749 23.0746 6.59919 288 168)",
            width: "30",
        }
        use {
            height: "20",
            href: "#cn-a",
            transform: "matrix(14.9991 -18.73557 18.73533 14.99929 240 216)",
            width: "30",
        }
    }"##
    );
}
