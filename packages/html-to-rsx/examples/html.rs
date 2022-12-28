use html_parser::Dom;

fn main() {
    let html = "hello world!";

    let dom = Dom::parse(html).unwrap();

    let body = rsx_rosetta::convert_from_html(dom);

    let out = dioxus_autofmt::write_block_out(body).unwrap();

    dbg!(out);
}
