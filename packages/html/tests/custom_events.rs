use dioxus::html::{CustomData, CustomEvent, HtmlEvent, SerializedCustomData};
use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;
use std::{any::Any, rc::Rc, sync::Mutex};

dioxus::html::register_event! {
    event: onnotice;
    js-event: "notice";
    bubbles: true;
}

dioxus::html::register_event! {
    /// Heard only on the element it is dispatched on.
    event: onwhisper;
    js-event: "whisper";
    bubbles: false;
}

const SHOUT: &str = "shout";

dioxus::html::register_event! {
    event: onshout;
    js-event: SHOUT;
    bubbles: true;
}

static HEARD: Mutex<Vec<(&'static str, Option<String>)>> = Mutex::new(Vec::new());

fn custom_event(detail: Option<&str>) -> Event<dyn Any> {
    Event::new(
        Rc::new(PlatformEventData::new(Box::new(SerializedCustomData::new(
            detail.map(str::to_string),
        )))) as Rc<dyn Any>,
        true,
    )
}

fn heard_by_name(event: CustomEvent) {
    HEARD.lock().unwrap().push(("named", event.detail()));
}

fn app() -> Element {
    rsx! {
        section { onnotice: move |event| HEARD.lock().unwrap().push(("outer", event.detail())),
            div { onnotice: move |event| HEARD.lock().unwrap().push(("inner", event.detail())),
                span { onwhisper: heard_by_name }
                p { onshout: move |event| HEARD.lock().unwrap().push(("constant", event.detail())) }
            }
        }
    }
}

#[test]
fn a_registered_event_is_heard_by_its_handlers_with_its_detail_and_bubbles_as_registered() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    HEARD.lock().unwrap().clear();

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    let listeners: Vec<&str> = oracle
        .historical_event_listener_targets()
        .iter()
        .map(|target| target.name.as_str())
        .collect();
    assert_eq!(
        listeners.iter().filter(|name| **name == "notice").count(),
        2
    );
    assert!(listeners.contains(&"whisper"), "{listeners:?}");
    assert!(listeners.contains(&SHOUT), "{listeners:?}");
    assert!(dioxus::html::event_bubbles(SHOUT));
    assert!(dioxus::html::event_bubbles("notice"));
    assert!(!dioxus::html::event_bubbles("whisper"));

    let div = oracle.element_id_by_tag("div");
    dom.runtime()
        .handle_event("notice", custom_event(Some("{\"kind\":\"warm\"}")), div);
    assert_eq!(
        *HEARD.lock().unwrap(),
        vec![
            ("inner", Some("{\"kind\":\"warm\"}".to_string())),
            ("outer", Some("{\"kind\":\"warm\"}".to_string())),
        ],
        "A bubbling custom event is heard where it lands and then above it."
    );

    HEARD.lock().unwrap().clear();
    let span = oracle.element_id_by_tag("span");
    dom.runtime()
        .handle_event("whisper", custom_event(None), span);
    assert_eq!(*HEARD.lock().unwrap(), vec![("named", None)]);

    HEARD.lock().unwrap().clear();
    let shouted = oracle.element_id_by_tag("p");
    dom.runtime()
        .handle_event(SHOUT, custom_event(Some("loud")), shouted);
    assert_eq!(
        *HEARD.lock().unwrap(),
        vec![("constant", Some("loud".to_string()))],
        "An event registered under a constant is heard by its own handlers, and no other event's."
    );
}

#[test]
fn a_registered_event_is_read_from_the_page_with_its_string_detail() {
    dioxus::html::events::register_custom_event("notice-read", true);

    let event: HtmlEvent = serde_json::from_str(
        r#"{"element":3,"name":"notice-read","bubbles":true,"data":{"detail":"hello"}}"#,
    )
    .expect("A registered custom event is read.");
    assert_eq!(event.name, "notice-read");
    assert_eq!(
        event.data,
        dioxus::html::EventData::Custom(SerializedCustomData::new(Some("hello".to_string())))
    );

    let without: HtmlEvent =
        serde_json::from_str(r#"{"element":3,"name":"notice-read","bubbles":true,"data":{}}"#)
            .expect("A custom event whose detail was not a string is read without one.");
    assert_eq!(
        without.data,
        dioxus::html::EventData::Custom(SerializedCustomData::new(None))
    );
}

#[test]
fn an_event_nobody_registered_is_not_read() {
    let error = serde_json::from_str::<HtmlEvent>(
        r#"{"element":3,"name":"never-registered","bubbles":true,"data":{"detail":"hello"}}"#,
    )
    .expect_err("An event nobody registered is refused.");
    assert!(
        error
            .to_string()
            .contains("Unknown event type: never-registered"),
        "{error}"
    );
}

#[test]
fn custom_data_travels_as_its_detail() {
    let data = CustomData::new(SerializedCustomData::new(Some("there".to_string())));
    assert_eq!(data.detail().as_deref(), Some("there"));
    let written = serde_json::to_string(&data).unwrap();
    assert_eq!(written, r#"{"detail":"there"}"#);
    let read: CustomData = serde_json::from_str(&written).unwrap();
    assert_eq!(read, data);
    assert!(read.downcast::<SerializedCustomData>().is_some());
    assert_eq!(
        format!("{data:?}"),
        r#"CustomData { detail: Some("there") }"#
    );
}

#[test]
#[should_panic(expected = "\"click\" is the name of an event Dioxus already handles")]
fn a_built_in_event_cannot_be_registered_as_a_custom_one() {
    dioxus::html::events::register_custom_event("click", true);
}

#[test]
#[should_panic(expected = "cannot be registered again with bubbles: false")]
fn a_custom_event_cannot_be_registered_again_with_other_bubbling() {
    dioxus::html::events::register_custom_event("notice-twice", true);
    dioxus::html::events::register_custom_event("notice-twice", false);
}
