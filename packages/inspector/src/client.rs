#![cfg(feature = "client")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, closure::Closure, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    Document, DomRect, Element, HtmlElement, KeyboardEvent, MouseEvent, Request, RequestInit,
    RequestMode, Response, window,
};

const DATA_ATTRIBUTE: &str = "data-inspector";

/// Errors that can be raised while installing the inspector client.
#[derive(Debug)]
pub enum InspectorClientError {
    WindowUnavailable,
    DocumentUnavailable,
    ListenerRegistrationFailed(String),
}

/// Keyboard modifiers that must be pressed to trigger inspection.
#[derive(Debug, Copy, Clone)]
pub struct ClickModifier {
    pub meta: bool,
    pub shift: bool,
}

impl Default for ClickModifier {
    fn default() -> Self {
        Self {
            meta: true,
            shift: true,
        }
    }
}

impl ClickModifier {
    fn matches(self, event: &MouseEvent) -> bool {
        (!self.meta || event.meta_key() || event.ctrl_key()) && (!self.shift || event.shift_key())
    }
}

/// Browser-side entry point that hooks click events.
#[derive(Debug, Clone)]
pub struct InspectorClient {
    endpoint: String,
    modifier: ClickModifier,
}

impl InspectorClient {
    /// Creates a new inspector client.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            modifier: ClickModifier::default(),
        }
    }

    /// Overrides the keyboard modifiers.
    pub fn with_modifier(mut self, modifier: ClickModifier) -> Self {
        self.modifier = modifier;
        self
    }

    /// Installs click handlers on the document.
    pub fn install(&self) -> Result<(), InspectorClientError> {
        let window = window().ok_or(InspectorClientError::WindowUnavailable)?;
        let document = window
            .document()
            .ok_or(InspectorClientError::DocumentUnavailable)?;

        let modifiers = self.modifier;
        let endpoint = self.endpoint.clone();

        let click_handler =
            Closure::<dyn FnMut(MouseEvent)>::wrap(Box::new(move |event: MouseEvent| {
                if !modifiers.matches(&event) {
                    return;
                }

                let target = match event.target().and_then(|t| t.dyn_into::<Element>().ok()) {
                    Some(element) => element,
                    None => return,
                };

                if let Some(marker) = find_marker(target) {
                    match serde_json::from_str::<DomMetadata>(&marker) {
                        Ok(payload) => dispatch(endpoint.clone(), payload),
                        Err(err) => warn(&format!("Failed to parse inspector payload: {err}")),
                    }
                }
            }));

        document
            .add_event_listener_with_callback("click", click_handler.as_ref().unchecked_ref())
            .map_err(|err| InspectorClientError::ListenerRegistrationFailed(format!("{err:?}")))?;
        click_handler.forget();

        if let Some(overlay) = HighlightOverlay::create(&document) {
            install_highlight_listeners(&document, modifiers, overlay)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DomMetadata {
    file: String,
    line: u32,
    column: u32,
    #[serde(default)]
    tag: Option<String>,
}

fn find_marker(mut element: Element) -> Option<String> {
    loop {
        if let Some(attr) = element.get_attribute(DATA_ATTRIBUTE) {
            return Some(attr);
        }

        match element.parent_element() {
            Some(parent) => element = parent,
            None => return None,
        }
    }
}

fn dispatch(endpoint: String, payload: DomMetadata) {
    log(&format!(
        "Inspector click captured for {} @{}:{} -> {}",
        payload.tag.as_deref().unwrap_or("node"),
        payload.file,
        payload.line,
        endpoint
    ));

    spawn_local(async move {
        if let Err(err) = send_to_server(&endpoint, &payload).await {
            warn(&format!("Failed to send inspector event: {err}"));
        }
    });
}

async fn send_to_server(endpoint: &str, payload: &DomMetadata) -> Result<(), String> {
    let window = window().ok_or("No window available")?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body = serde_json::to_string(payload).map_err(|e| e.to_string())?;
    let body_value = JsValue::from_str(&body);
    opts.set_body(&body_value);

    let request =
        Request::new_with_str_and_init(endpoint, &opts).map_err(|e| format!("{:?}", e))?;

    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let _resp: Response = resp_value.dyn_into().map_err(|_| "Response mismatch")?;

    log("Inspector event sent successfully");
    Ok(())
}

fn log(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

fn warn(message: &str) {
    web_sys::console::warn_1(&JsValue::from_str(message));
}

#[derive(Clone)]
struct HighlightOverlay {
    element: HtmlElement,
}

impl HighlightOverlay {
    fn create(document: &Document) -> Option<Self> {
        let body = document.body()?;
        let element: HtmlElement = document.create_element("div").ok()?.dyn_into().ok()?;
        let style = element.style();
        let _ = style.set_property("position", "fixed");
        let _ = style.set_property("border", "2px solid #38bdf8");
        let _ = style.set_property("border-radius", "6px");
        let _ = style.set_property("box-shadow", "0 0 0 2px rgba(56,189,248,0.35)");
        let _ = style.set_property("background", "rgba(56,189,248,0.15)");
        let _ = style.set_property("pointer-events", "none");
        let _ = style.set_property("z-index", "2147483647");
        let _ = style.set_property("display", "none");
        body.append_child(&element).ok()?;
        Some(Self { element })
    }

    fn show(&self, element: &Element) {
        let rect = element.get_bounding_client_rect();
        self.apply_rect(&rect);
    }

    fn apply_rect(&self, rect: &DomRect) {
        if rect.width() <= 0.0 && rect.height() <= 0.0 {
            self.hide();
            return;
        }

        let style = self.element.style();
        let _ = style.set_property("display", "block");
        let _ = style.set_property("left", &format!("{}px", rect.left()));
        let _ = style.set_property("top", &format!("{}px", rect.top()));
        let _ = style.set_property("width", &format!("{}px", rect.width()));
        let _ = style.set_property("height", &format!("{}px", rect.height()));
    }

    fn hide(&self) {
        let _ = self.element.style().set_property("display", "none");
    }
}

fn install_highlight_listeners(
    document: &Document,
    modifiers: ClickModifier,
    overlay: HighlightOverlay,
) -> Result<(), InspectorClientError> {
    let move_overlay = overlay.clone();
    let highlight_handler =
        Closure::<dyn FnMut(MouseEvent)>::wrap(Box::new(move |event: MouseEvent| {
            if !modifiers.matches(&event) {
                move_overlay.hide();
                return;
            }

            let target = match event.target().and_then(|t| t.dyn_into::<Element>().ok()) {
                Some(element) => element,
                None => {
                    move_overlay.hide();
                    return;
                }
            };

            if find_marker(target.clone()).is_some() {
                move_overlay.show(&target);
            } else {
                move_overlay.hide();
            }
        }));

    document
        .add_event_listener_with_callback("mousemove", highlight_handler.as_ref().unchecked_ref())
        .map_err(|err| InspectorClientError::ListenerRegistrationFailed(format!("{err:?}")))?;
    highlight_handler.forget();

    let key_overlay = overlay.clone();
    let keyup_handler =
        Closure::<dyn FnMut(KeyboardEvent)>::wrap(Box::new(move |_event: KeyboardEvent| {
            key_overlay.hide();
        }));

    document
        .add_event_listener_with_callback("keyup", keyup_handler.as_ref().unchecked_ref())
        .map_err(|err| InspectorClientError::ListenerRegistrationFailed(format!("{err:?}")))?;
    keyup_handler.forget();

    Ok(())
}
