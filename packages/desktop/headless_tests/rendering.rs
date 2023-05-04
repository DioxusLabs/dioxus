use dioxus::prelude::*;
use dioxus_desktop::DesktopContext;

pub(crate) fn check_app_exits(app: Component) {
    // This is a deadman's switch to ensure that the app exits
    let should_panic = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let should_panic_clone = should_panic.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(100));
        if should_panic_clone.load(std::sync::atomic::Ordering::SeqCst) {
            std::process::exit(exitcode::SOFTWARE);
        }
    });

    dioxus_desktop::launch(app);

    should_panic.store(false, std::sync::atomic::Ordering::SeqCst);
}

fn main() {
    check_app_exits(check_html_renders);
}

fn use_inner_html(cx: &ScopeState, id: &'static str) -> Option<String> {
    let value: &UseRef<Option<String>> = use_ref(cx, || None);
    use_effect(cx, (), |_| {
        to_owned![value];
        let desktop_context: DesktopContext = cx.consume_context().unwrap();
        async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let html = desktop_context
                .eval(&format!(
                    r#"let element = document.getElementById('{}');
                return element.innerHTML;"#,
                    id
                ))
                .await;
            if let Ok(serde_json::Value::String(html)) = html {
                println!("html: {}", html);
                value.set(Some(html));
            }
        }
    });
    value.read().clone()
}

const EXPECTED_HTML: &str = r#"<div id="5" style="width: 100px; height: 100px; color: rgb(0, 0, 0);"><input type="checkbox"><h1>text</h1><div><p>hello world</p></div></div>"#;

fn check_html_renders(cx: Scope) -> Element {
    let inner_html = use_inner_html(cx, "main_div");
    let desktop_context: DesktopContext = cx.consume_context().unwrap();

    if let Some(raw_html) = inner_html.as_deref() {
        let fragment = scraper::Html::parse_fragment(raw_html);
        println!("fragment: {:?}", fragment.html());
        let expected = scraper::Html::parse_fragment(EXPECTED_HTML);
        println!("fragment: {:?}", expected.html());
        if fragment == expected {
            println!("html matches");
            desktop_context.close();
        }
    }

    let dyn_value = 0;
    let dyn_element = rsx! {
        div {
            dangerous_inner_html: "<p>hello world</p>",
        }
    };

    render! {
        div {
            id: "main_div",
            div {
                width: "100px",
                height: "100px",
                color: "rgb({dyn_value}, {dyn_value}, {dyn_value})",
                id: 5,
                input {
                    "type": "checkbox",
                },
                h1 {
                    "text"
                }
                dyn_element
            }
        }
    }
}
