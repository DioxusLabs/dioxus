//! Fallible checks let the app guard clean up without trapping the shared wasm instance.
#![cfg(all(target_arch = "wasm32", feature = "web"))]

use dioxus::prelude::VirtualDom;
use dioxus_playwright_markerless_hydration_edges_test::app;

use futures_util::future::{AbortHandle, abortable};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_futures::wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{DomParser, HtmlTextAreaElement, MouseEvent, MouseEventInit, SupportedType};

wasm_bindgen_test_configure!(run_in_browser);

const SSR: &str = include_str!("fixtures/ssr.html");
const WAIT_MS: f64 = 5_000.0;

type Outcome = Result<(), JsValue>;

macro_rules! ensure {
    ($condition:expr, $($message:tt)+) => {{
        let passed = $condition;
        if !passed {
            return Err(JsValue::from_str(&format!($($message)+)));
        }
    }};
}

macro_rules! ensure_eq {
    ($actual:expr, $expected:expr) => {
        match (&$actual, &$expected) {
            (actual, expected) => ensure!(
                actual == expected,
                "{} is {actual:?}, expected {expected:?}",
                stringify!($actual)
            ),
        }
    };
}

fn window() -> web_sys::Window {
    web_sys::window().unwrap()
}

/// Stops the app and detaches its root between tests.
struct HydratedApp {
    root: web_sys::Element,
    app: AbortHandle,
}

impl Drop for HydratedApp {
    fn drop(&mut self) {
        self.app.abort();
        self.root.remove();
    }
}

/// Runs the fixture's inline scripts in order before hydrating its imported `#main`.
fn hydrate_fixture() -> HydratedApp {
    let document = window().document().unwrap();
    let parsed = DomParser::new()
        .unwrap()
        .parse_from_string(SSR, SupportedType::TextHtml)
        .unwrap();
    let root: web_sys::Element = document
        .import_node_with_deep(&parsed.get_element_by_id("main").unwrap(), true)
        .unwrap()
        .unchecked_into();
    document.body().unwrap().append_child(&root).unwrap();

    js_sys::eval(
        "window.__page_errors = [];\
         window.onerror = (message) => { window.__page_errors.push(String(message)); };\
         window.onunhandledrejection = (event) => { window.__page_errors.push(String(event.reason)); };",
    )
    .unwrap();
    let scripts = parsed
        .query_selector_all("script:not([src]):not([type=module])")
        .unwrap();
    for index in 0..scripts.length() {
        let source = scripts.item(index).unwrap().text_content().unwrap();
        js_sys::Function::new_no_args(&source)
            .call0(&window())
            .unwrap();
    }

    let (app, abort) = abortable(dioxus::web::run(
        VirtualDom::new(app),
        dioxus::web::Config::new()
            .hydrate(true)
            .rootelement(root.clone()),
    ));
    wasm_bindgen_futures::spawn_local(async move {
        let _ = app.await;
    });
    HydratedApp { root, app: abort }
}

async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    JsFuture::from(promise).await.unwrap();
}

fn page_errors() -> Vec<String> {
    let errors = js_sys::Reflect::get(&window(), &JsValue::from_str("__page_errors")).unwrap();
    js_sys::Array::from(&errors)
        .iter()
        .map(|error| error.as_string().unwrap_or_default())
        .collect()
}

fn no_page_errors() -> Outcome {
    let errors = page_errors();
    ensure!(errors.is_empty(), "page errors: {errors:?}");
    Ok(())
}

/// Polls on the monotonic page clock with a deadline and page-error checks.
async fn eventually(what: &str, mut check: impl FnMut() -> bool) -> Outcome {
    let clock = window().performance().unwrap();
    let deadline = clock.now() + WAIT_MS;
    while !check() {
        let errors = page_errors();
        ensure!(
            errors.is_empty(),
            "page errors while waiting for {what}: {errors:?}"
        );
        ensure!(clock.now() < deadline, "timed out waiting for {what}");
        sleep(10).await;
    }
    Ok(())
}

/// The client task materializes `#late-empty-root` after hydration binds the tree.
async fn hydrated(root: &web_sys::Element) -> Outcome {
    eventually("hydration to finish", || {
        find(root, "#late-empty-root").is_some()
    })
    .await
}

fn find(root: &web_sys::Element, selector: &str) -> Option<web_sys::Element> {
    root.query_selector(selector).unwrap()
}

fn text(root: &web_sys::Element, selector: &str) -> Option<String> {
    find(root, selector).and_then(|element| element.text_content())
}

/// Dispatches a bubbling click, which also reaches SVG elements that lack `HTMLElement.click()`.
fn click(root: &web_sys::Element, selector: &str) -> Outcome {
    let Some(target) = find(root, selector) else {
        return Err(JsValue::from_str(&format!("{selector} is missing")));
    };
    let init = MouseEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = MouseEvent::new_with_mouse_event_init_dict("click", &init)?;
    target.dispatch_event(&event)?;
    Ok(())
}

async fn expect_text(root: &web_sys::Element, selector: &str, expected: &str) -> Outcome {
    eventually(&format!("{selector} to read {expected:?}"), || {
        text(root, selector).as_deref() == Some(expected)
    })
    .await
}

fn textarea_value(root: &web_sys::Element, selector: &str) -> String {
    find(root, selector)
        .map(|element| element.unchecked_into::<HtmlTextAreaElement>().value())
        .unwrap_or_default()
}

fn no_comments(root: &web_sys::Element, selector: &str) -> Outcome {
    let inner = find(root, selector)
        .map(|element| element.inner_html())
        .unwrap_or_default();
    ensure!(
        !inner.contains("<!--"),
        "comment left in {selector}: {inner}"
    );
    Ok(())
}

#[wasm_bindgen_test]
async fn scope_with_no_ssr_roots_hydrates_virtual_root_anchors() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    expect_text(root, "#late-empty-root", "late root ready").await?;
    ensure!(
        root.text_content()
            .unwrap_or_default()
            .contains("root text ready"),
        "root text never rendered"
    );
    no_page_errors()
}

#[wasm_bindgen_test]
async fn streamed_suspense_resolving_to_empty_hydrates_its_empty_slot() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    expect_text(root, "#streamed-empty-hydration", "streamed empty hydrated").await?;
    no_comments(root, "#streamed-empty-hydration")?;
    no_page_errors()
}

#[wasm_bindgen_test]
async fn dangerous_inner_html_hydrates_host_and_updates_inner_html() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;
    ensure!(
        find(root, "#raw-inner-host #raw-inner-child").is_some(),
        "server-rendered inner HTML is gone"
    );
    ensure!(
        text(root, "#raw-inner-host")
            .unwrap_or_default()
            .contains("raw HTML"),
        "host text changed before the swap"
    );

    click(root, "#swap-raw-inner")?;
    eventually("innerHTML to be replaced", || {
        find(root, "#raw-inner-host #raw-inner-child-updated").is_some()
    })
    .await?;
    ensure!(
        find(root, "#raw-inner-host #raw-inner-child").is_none(),
        "old inner HTML survived the swap"
    );
    ensure!(
        text(root, "#raw-inner-host")
            .unwrap_or_default()
            .contains("changed HTML"),
        "host text did not change"
    );
    no_page_errors()
}

#[wasm_bindgen_test]
async fn adjacent_dynamic_texts_split_correctly_after_hydration() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;
    ensure_eq!(text(root, "#adjacent-texts").as_deref(), Some("AAABBB"));
    click(root, "#swap-adjacent")?;
    expect_text(root, "#adjacent-texts", "CCC").await?;

    ensure_eq!(
        text(root, "#utf16-text").as_deref(),
        Some("before 💧 | 🌊🌊 after")
    );
    click(root, "#utf16-swap")?;
    expect_text(root, "#utf16-text", "before é💧é |  after").await?;
    no_page_errors()
}

#[wasm_bindgen_test]
async fn empty_dynamic_texts_hydrate_in_source_order_in_every_position() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;
    ensure_eq!(text(root, "#trailing-empties-10").as_deref(), Some("HEAD"));
    ensure_eq!(text(root, "#leading-empties-10").as_deref(), Some("TAIL"));
    ensure_eq!(text(root, "#all-empties-10").as_deref(), Some(""));

    click(root, "#fill-runs")?;
    let labels = "[a][b][c][d][e][f][g][h][i][j]";
    expect_text(root, "#trailing-empties-10", &format!("HEAD{labels}")).await?;
    expect_text(root, "#leading-empties-10", &format!("{labels}TAIL")).await?;
    expect_text(root, "#all-empties-10", labels).await?;

    ensure_eq!(text(root, "#empty-middle").as_deref(), Some("AAABBB"));
    click(root, "#fill-middle")?;
    expect_text(root, "#empty-middle", "leftMIDright").await?;

    ensure_eq!(text(root, "#separated-empty-slots").as_deref(), Some("S"));
    click(root, "#fill-separated-slot-b")?;
    expect_text(root, "#separated-empty-slots", "SB").await?;
    click(root, "#fill-separated-slot-a")?;
    expect_text(root, "#separated-empty-slots", "ASB").await?;
    no_page_errors()
}

#[wasm_bindgen_test]
async fn child_component_contributing_to_parent_text_run_hydrates_correctly() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;
    ensure_eq!(
        text(root, "#component-in-run").as_deref(),
        Some("before MID after")
    );
    click(root, "#swap-component-text")?;
    expect_text(root, "#component-in-run", "before UPDATED after").await?;
    no_page_errors()
}

#[wasm_bindgen_test]
async fn virtual_placeholders_anchor_replace_insert_remove_without_comments() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;

    ensure_eq!(
        text(root, "#placeholder-to-content").as_deref(),
        Some("before  after")
    );
    no_comments(root, "#placeholder-to-content")?;
    click(root, "#toggle-placeholder")?;
    expect_text(root, "#placeholder-to-content", "before HELLO after").await?;
    ensure!(
        find(root, "#placeholder-content").is_some(),
        "#placeholder-content is missing"
    );
    click(root, "#toggle-placeholder")?;
    expect_text(root, "#placeholder-to-content", "before  after").await?;
    no_comments(root, "#placeholder-to-content")?;

    ensure_eq!(text(root, "#trailing-placeholder").as_deref(), Some("HEAD"));
    click(root, "#append-trailing")?;
    expect_text(root, "#trailing-placeholder", "HEAD(1)[1]").await?;
    click(root, "#append-trailing")?;
    expect_text(root, "#trailing-placeholder", "HEAD(2)[1][2]").await?;
    click(root, "#append-trailing")?;
    expect_text(root, "#trailing-placeholder", "HEAD(3)[1][2][3]").await?;

    ensure_eq!(
        text(root, "#remove-placeholder").as_deref(),
        Some("edges PRESENT edges")
    );
    click(root, "#hide-removable")?;
    expect_text(root, "#remove-placeholder", "edges  edges").await?;
    no_comments(root, "#remove-placeholder")?;
    no_page_errors()
}

#[wasm_bindgen_test]
async fn trailing_root_level_placeholder_keeps_the_mount_parent() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;
    ensure!(
        find(root, "#trailing-root-late").is_none(),
        "late root rendered before the click"
    );
    click(root, "#show-trailing-root")?;
    expect_text(root, "#trailing-root-late", "late trailing root").await?;

    let ids = [
        "show-trailing-root",
        "trailing-root-before",
        "trailing-root-late",
    ];
    let matches = root.query_selector_all("button, div")?;
    let order: Vec<String> = (0..matches.length())
        .filter_map(|index| matches.item(index))
        .map(|node| node.unchecked_into::<web_sys::Element>().id())
        .filter(|id| ids.contains(&id.as_str()))
        .collect();
    ensure_eq!(order, ids);
    no_page_errors()
}

#[wasm_bindgen_test]
async fn trailing_conditional_under_a_nested_non_root_element_hydrates_and_appends() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;
    ensure_eq!(text(root, "#nested-trailing").as_deref(), Some("HEAD"));
    ensure!(
        find(root, "#nested-trailing-late").is_none(),
        "late content rendered before the click"
    );

    click(root, "#show-nested-trailing")?;
    expect_text(root, "#nested-trailing-late", "LATE").await?;
    ensure_eq!(text(root, "#nested-trailing").as_deref(), Some("HEADLATE"));
    no_page_errors()
}

#[wasm_bindgen_test]
async fn svg_elements_can_receive_hydrated_listeners() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;
    ensure_eq!(
        text(root, "#svg-click-count").as_deref(),
        Some("svg clicks: 0")
    );
    click(root, "#hydrated-circle")?;
    expect_text(root, "#svg-click-count", "svg clicks: 1").await?;
    let id = find(root, "#hydrated-circle")
        .and_then(|circle| circle.get_attribute("data-dioxus-id"))
        .unwrap_or_default();
    ensure!(
        id.chars().any(|c| c.is_ascii_digit()),
        "data-dioxus-id is {id:?}"
    );
    no_page_errors()
}

#[wasm_bindgen_test]
async fn textarea_dynamic_body_hydrates_without_markers_and_updates() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;
    ensure_eq!(
        textarea_value(root, "#textarea-hydration"),
        "initial textarea body"
    );
    click(root, "#textarea-hydration-update")?;
    eventually("the textarea body to update", || {
        textarea_value(root, "#textarea-hydration") == "updated textarea body"
    })
    .await?;
    no_page_errors()
}

#[wasm_bindgen_test]
async fn raw_text_elements_with_a_leading_newline_hydrate_and_update_correctly() -> Outcome {
    let app = hydrate_fixture();
    let root = &app.root;
    hydrated(root).await?;

    let body = textarea_value(root, "#textarea-leading-newline");
    ensure!(body.contains("BODY"), "textarea reads {body:?}");
    click(root, "#textarea-leading-newline-update")?;
    eventually("the textarea to read NEW", || {
        textarea_value(root, "#textarea-leading-newline").contains("NEW")
    })
    .await?;
    // A body bound one character late keeps a stray "B" ("BNEW").
    let updated = textarea_value(root, "#textarea-leading-newline");
    ensure!(!updated.contains('B'), "textarea reads {updated:?}");

    let code = text(root, "#pre-leading-newline").unwrap_or_default();
    ensure!(code.contains("CODE"), "pre reads {code:?}");
    click(root, "#pre-leading-newline-update")?;
    eventually("the pre to read CHANGED", || {
        text(root, "#pre-leading-newline").is_some_and(|text| text.contains("CHANGED"))
    })
    .await?;
    let changed = text(root, "#pre-leading-newline").unwrap_or_default();
    ensure!(!changed.contains("CODE"), "pre reads {changed:?}");
    no_page_errors()
}
