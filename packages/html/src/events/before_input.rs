use dioxus_core::Event;
use std::fmt::Debug;

pub type BeforeInputEvent = Event<BeforeInputData>;

/// Data fired alongside the `beforeinput` event.
///
/// The `beforeinput` event fires before an editable element (an `<input>`, `<textarea>`,
/// or any element with `contenteditable="true"`) is about to be modified. It exposes
/// the kind of mutation that is about to happen via [`BeforeInputData::input_type`]
/// and the text that is about to be inserted (if any) via [`BeforeInputData::data`].
///
/// Unlike `input`, `beforeinput` is cancellable: calling `event.prevent_default()`
/// from the handler will block the user-agent from applying the change.
///
/// See <https://developer.mozilla.org/en-US/docs/Web/API/Element/beforeinput_event>
/// for the underlying DOM event.
pub struct BeforeInputData {
    inner: Box<dyn HasBeforeInputData>,
}

impl BeforeInputData {
    /// Create a new `BeforeInputData` from any [`HasBeforeInputData`] implementation.
    pub fn new(inner: impl HasBeforeInputData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// The type of change that is about to be applied to the editable element.
    ///
    /// Common values include `"insertText"`, `"insertFromPaste"`, `"insertLineBreak"`,
    /// `"deleteContentBackward"`, `"deleteContentForward"`, `"historyUndo"`, and
    /// `"historyRedo"`. The full list is documented at
    /// <https://w3c.github.io/input-events/#interface-InputEvent-Attributes>.
    pub fn input_type(&self) -> String {
        self.inner.input_type()
    }

    /// The characters that are about to be inserted by the user agent, if any.
    ///
    /// `None` is returned for input types that don't carry text data, e.g. deletions
    /// or rich-text formatting changes.
    pub fn data(&self) -> Option<String> {
        self.inner.data()
    }

    /// Whether the event was fired while an IME composition session is active.
    pub fn is_composing(&self) -> bool {
        self.inner.is_composing()
    }

    /// The current value of the editable element, prior to the pending change.
    pub fn value(&self) -> String {
        self.inner.value()
    }

    /// Downcast this event to a concrete platform-specific event type.
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl<E: HasBeforeInputData> From<E> for BeforeInputData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl PartialEq for BeforeInputData {
    fn eq(&self, other: &Self) -> bool {
        self.input_type() == other.input_type()
            && self.data() == other.data()
            && self.is_composing() == other.is_composing()
            && self.value() == other.value()
    }
}

impl Debug for BeforeInputData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BeforeInputData")
            .field("input_type", &self.input_type())
            .field("data", &self.data())
            .field("is_composing", &self.is_composing())
            .field("value", &self.value())
            .finish()
    }
}

/// An object exposing all the data backing a [`BeforeInputData`] event.
pub trait HasBeforeInputData: std::any::Any {
    /// The type of input change. See [`BeforeInputData::input_type`].
    fn input_type(&self) -> String;

    /// The text data being inserted, or `None` if the change carries no text.
    fn data(&self) -> Option<String>;

    /// Whether the event is fired during an active IME composition session.
    fn is_composing(&self) -> bool;

    /// The current value of the editable target, prior to the change.
    fn value(&self) -> String;

    /// Return self as `Any` so the event can be downcast to its concrete type.
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(feature = "serialize")]
pub use serialize::*;

#[cfg(feature = "serialize")]
mod serialize {
    use super::*;

    /// A serialized version of [`BeforeInputData`] used to transport the event
    /// across the IPC boundary on non-web renderers.
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone, Default)]
    pub struct SerializedBeforeInputData {
        pub input_type: String,

        #[serde(default)]
        pub data: Option<String>,

        #[serde(default)]
        pub is_composing: bool,

        #[serde(default)]
        pub value: String,
    }

    impl SerializedBeforeInputData {
        pub fn new(
            input_type: String,
            data: Option<String>,
            is_composing: bool,
            value: String,
        ) -> Self {
            Self {
                input_type,
                data,
                is_composing,
                value,
            }
        }
    }

    impl HasBeforeInputData for SerializedBeforeInputData {
        fn input_type(&self) -> String {
            self.input_type.clone()
        }

        fn data(&self) -> Option<String> {
            self.data.clone()
        }

        fn is_composing(&self) -> bool {
            self.is_composing
        }

        fn value(&self) -> String {
            self.value.clone()
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    impl From<&BeforeInputData> for SerializedBeforeInputData {
        fn from(data: &BeforeInputData) -> Self {
            Self {
                input_type: data.input_type(),
                data: data.data(),
                is_composing: data.is_composing(),
                value: data.value(),
            }
        }
    }

    impl serde::Serialize for BeforeInputData {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            SerializedBeforeInputData::from(self).serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for BeforeInputData {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let data = SerializedBeforeInputData::deserialize(deserializer)?;
            Ok(Self {
                inner: Box::new(data),
            })
        }
    }
}

#[cfg(all(test, feature = "serialize"))]
mod tests {
    use super::*;

    #[test]
    fn serialized_before_input_data_deserializes_missing_optional_fields() {
        // `input_type` is required; it acts as the discriminator that keeps
        // beforeinput payloads from being silently matched by other variants
        // of `EventData` (see packages/html/src/transit.rs).
        let data: SerializedBeforeInputData =
            serde_json::from_str(r#"{"input_type": "insertText"}"#).unwrap();
        assert_eq!(data.input_type, "insertText");
        assert_eq!(data.data, None);
        assert!(!data.is_composing);
        assert_eq!(data.value, "");
    }

    #[test]
    fn serialized_before_input_data_rejects_missing_input_type() {
        assert!(serde_json::from_str::<SerializedBeforeInputData>("{}").is_err());
    }

    #[test]
    fn before_input_data_exposes_serialized_fields() {
        let event = BeforeInputData::new(SerializedBeforeInputData::new(
            "insertText".to_string(),
            Some("a".to_string()),
            false,
            "hello".to_string(),
        ));

        assert_eq!(event.input_type(), "insertText");
        assert_eq!(event.data().as_deref(), Some("a"));
        assert!(!event.is_composing());
        assert_eq!(event.value(), "hello");
    }

    /// The browser-side serializer (packages/interpreter/src/ts/serialize.ts)
    /// emits these exact field names for a `beforeinput` event. Pin the shape
    /// so changes on either side break the test instead of silently degrading
    /// the event payload.
    #[test]
    fn deserializes_payload_from_js_interpreter() {
        let payload = r#"{
            "input_type": "insertFromPaste",
            "data": "pasted",
            "is_composing": false,
            "value": "hello pasted",
            "values": [],
            "valid": true
        }"#;

        let data: SerializedBeforeInputData = serde_json::from_str(payload).unwrap();
        assert_eq!(data.input_type, "insertFromPaste");
        assert_eq!(data.data.as_deref(), Some("pasted"));
        assert!(!data.is_composing);
        assert_eq!(data.value, "hello pasted");
    }

    #[test]
    fn round_trips_through_serde() {
        let original = SerializedBeforeInputData::new(
            "deleteContentBackward".to_string(),
            None,
            true,
            "hell".to_string(),
        );

        let json = serde_json::to_string(&original).unwrap();
        let back: SerializedBeforeInputData = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
    }
}
