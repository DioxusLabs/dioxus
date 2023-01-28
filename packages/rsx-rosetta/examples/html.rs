use html_parser::Dom;

fn main() {
    let html = r#"
    <div>
        <div class="asd">hello world!</div>
        <div id="asd">hello world!</div>
        <div id="asd">hello world!</div>
        <div for="asd">hello world!</div>
        <div async="asd">hello world!</div>
        <div LargeThing="asd">hello world!</div>
        <ai-is-awesome>hello world!</ai-is-awesome>
    </div>
    "#
    .trim();

    let dom = Dom::parse(html).unwrap();

    let body = rsx_rosetta::rsx_from_html(&dom);

    let out = dioxus_autofmt::write_block_out(body).unwrap();

    println!("{out}");
}
