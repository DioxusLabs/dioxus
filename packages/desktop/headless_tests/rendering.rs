use dioxus::prelude::*;
use dioxus_core::Element;
use dioxus_desktop::DesktopContext;

fn main() {
    check_app_exits(check_html_renders);
}

pub(crate) fn check_app_exits(app: Component) {
    use dioxus_desktop::Config;
    use tao::window::WindowBuilder;
    // This is a deadman's switch to ensure that the app exits
    let should_panic = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let should_panic_clone = should_panic.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(100));
        if should_panic_clone.load(std::sync::atomic::Ordering::SeqCst) {
            std::process::exit(exitcode::SOFTWARE);
        }
    });

    dioxus_desktop::launch_cfg(
        app,
        Config::new().with_window(WindowBuilder::new().with_visible(true)),
    );

    should_panic.store(false, std::sync::atomic::Ordering::SeqCst);
}

fn use_inner_html(d: &'static str) -> Option<String> {
    let value: Signal<Option<String>> = use_signal(|| None);
    use_effect(|| async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        window().eval();
        let html = eval_provider(&format!(
            r#"let element = document.getElementById('{}');
                    return element.innerHTML"#,
            id
        ))
        .unwrap();
        if let Ok(serde_json::Value::String(html)) = html.await {
            println!("html: {}", html);
            value.set(Some(html));
        }
    });
    value.read().clone()
}

const EXPECTED_HTML: &str = r#"<div id="5" style="width: 100px; height: 100px; color: rgb(0, 0, 0);"><input type="checkbox"><h1>text</h1><div><p>hello world</p></div></div>"#;

fn check_html_renders() -> Element {
    let inner_html = use_inner_html("main_div");

    let desktop_context: DesktopContext = cx.consume_context().unwrap();

    if let Some(raw_html) = inner_html {
        println!("{}", raw_html);
        let fragment = &raw_html;
        let expected = EXPECTED_HTML;
        // let fragment = scraper::Html::parse_fragment(&raw_html);
        // println!("fragment: {}", fragment.html());
        // let expected = scraper::Html::parse_fragment(EXPECTED_HTML);
        // println!("expected: {}", expected.html());
        assert_eq!(raw_html, EXPECTED_HTML);
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

    rsx! {
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
                {dyn_element}
            }
        }
    }
}
