use dioxus::prelude::*;
use dioxus_desktop::DesktopContext;

#[path = "./utils.rs"]
mod utils;

fn main() {
    #[cfg(not(windows))]
    utils::check_app_exits(check_html_renders);
}

fn use_inner_html(id: &'static str) -> Option<String> {
    let mut value = use_signal(|| None as Option<String>);

    use_effect(move || {
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let res = document::eval(&format!(
                r#"let element = document.getElementById('{}');
                return element.innerHTML"#,
                id
            ))
            .await
            .unwrap();

            if let Some(html) = res.as_str() {
                println!("html: {}", html);
                value.set(Some(html.to_string()));
            }
        });
    });

    value()
}

const EXPECTED_HTML: &str = r#"<div style="width: 100px; height: 100px; color: rgb(0, 0, 0);" id="5"><input type="checkbox"><h1>text</h1><div><p>hello world</p></div></div>"#;

fn check_html_renders() -> Element {
    let inner_html = use_inner_html("main_div");

    let desktop_context: DesktopContext = consume_context();

    if let Some(raw_html) = inner_html {
        println!("{}", raw_html);
        let fragment = &raw_html;
        let expected = EXPECTED_HTML;
        assert_eq!(raw_html, EXPECTED_HTML);
        if fragment == expected {
            println!("html matches");
            desktop_context.close();
        }
    }

    let dyn_value = 0;
    let dyn_element = rsx! { div { dangerous_inner_html: "<p>hello world</p>" } };

    rsx! {
        div { id: "main_div",
            div {
                width: "100px",
                height: "100px",
                color: "rgb({dyn_value}, {dyn_value}, {dyn_value})",
                id: 5,
                input { "type": "checkbox" }
                h1 { "text" }
                {dyn_element}
            }
        }
    }
}
