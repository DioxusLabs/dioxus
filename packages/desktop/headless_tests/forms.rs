use dioxus::prelude::*;
use dioxus_desktop::DesktopContext;
use dioxus_document::eval;

#[path = "./utils.rs"]
mod utils;

fn main() {
    #[cfg(not(windows))]
    utils::check_app_exits(check_html_renders);
}

async fn inner_html(count: usize) -> String {
    let html = document::eval(&format!(
        r#"// Wait until the element is available
            const interval = setInterval(() => {{
                const element = document.getElementById('div-{count}');
                if (element) {{
                    dioxus.send(element.innerHTML);
                    clearInterval(interval);
                }}
            }}, 100);"#
    ))
    .recv::<String>()
    .await
    .unwrap();

    println!("html: {html}");
    html
}

fn check_html_renders() -> Element {
    let mut signal = use_signal(|| 0);

    use_effect(move || {
        let signal = signal();
        // Pass the test once the count is greater than 10
        if signal > 10 {
            let desktop_context: DesktopContext = consume_context();
            desktop_context.close();
        }

        spawn(async move {
            let raw_html = inner_html(signal).await;
            println!("{raw_html}");
            // Make sure the html contains count is {signal}
            assert!(raw_html.contains(&format!("count is {}", signal)));
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            eval(
                r#"let button = document.querySelector('#increment');
                    button.dispatchEvent(new MouseEvent('click', {
                        view: window,
                        bubbles: true,
                        cancelable: true,
                        buttons: 2,
                        button: 2,
                    }));"#,
            );

            // If the signal is even, also submit the form. This should not effect the count because the navigation is blocked
            if signal % 2 == 0 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                eval(
                    r#"let form = document.getElementById('form-submit');
                        form.dispatchEvent(new Event('click', {
                            view: window,
                            bubbles: true,
                            cancelable: true,
                            buttons: 2,
                            button: 2,
                        }));"#,
                );
            }
        });
    });

    rsx! {
        div {
            id: "div-{signal}",
            h1 { "Form" }
            form {
                input {
                    r#type: "text",
                    name: "username",
                    id: "username",
                    value: "goodbye"
                }
                input { r#type: "text", name: "full-name", value: "lorem" }
                input { r#type: "password", name: "password", value: "ipsum" }
                input {
                    r#type: "radio",
                    name: "color",
                    value: "red",
                    checked: true
                }
                input { r#type: "radio", name: "color", value: "blue" }
                button { id: "form-submit", r#type: "submit", value: "Submit", "Submit the form" }
            }

            button {
                id: "increment",
                onclick: move |_| {
                    signal += 1;
                },
                "count is {signal}"
            }
        }
    }
}
