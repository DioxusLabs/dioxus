//! An example where the dioxus vdom is running in a native thread, interacting with webview

fn main() {
    let html_content = "<div>Hello world!</div>";

    web_view::builder()
        .title("My Project")
        .content(web_view::Content::Html(html_content))
        .size(320, 480)
        .resizable(true)
        .debug(true)
        .user_data(())
        .invoke_handler(|view, arg| {
            //

            Ok(())
        })
        .run()
        .unwrap();
}
