use dioxus_core::Event;
use std::fmt::{self, Debug, Display};

pub type BeforeInputEvent = Event<BeforeInputData>;

/// Define a string-backed enum from a single source-of-truth list that maps each variant
/// to its canonical DOM string. The two conversion directions — `as_str` (variant → string)
/// and `From<&str>` (string → variant) — are generated from that one list, so they can never
/// drift out of sync: a new value only has to be added in one place. The trailing
/// `_ => Variant(String)` arm declares a catch-all variant whose owned field preserves any
/// value not covered by the known variants.
macro_rules! string_enum {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $variant:ident => $value:literal,
            )*
            $(#[$unknown_meta:meta])*
            _ => $unknown:ident($unknown_ty:ty),
        }
    ) => {
        $(#[$enum_meta])*
        $vis enum $name {
            $(
                $variant,
            )*
            $(#[$unknown_meta])*
            $unknown($unknown_ty),
        }

        impl $name {
            /// The canonical `inputType` string for this variant, exactly as emitted by the
            /// DOM. For [`InputType::Unknown`] this returns the original value.
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$variant => $value, )*
                    Self::$unknown(value) => value,
                }
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $( $value => Self::$variant, )*
                    other => Self::$unknown(other.to_string()),
                }
            }
        }
    };
}

string_enum! {
    /// The kind of mutation that is about to be applied to an editable element.
    ///
    /// These map to the `inputType` attribute of the underlying DOM `InputEvent`. The
    /// full list of values is defined in the W3C Input Events spec at
    /// <https://w3c.github.io/input-events/#interface-InputEvent-Attributes>. Any value
    /// not covered by a known variant (e.g. one introduced by a newer user agent) is
    /// preserved verbatim in [`InputType::Unknown`].
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "serialize", serde(from = "String", into = "String"))]
    pub enum InputType {
        InsertText => "insertText",
        InsertReplacementText => "insertReplacementText",
        InsertLineBreak => "insertLineBreak",
        InsertParagraph => "insertParagraph",
        InsertOrderedList => "insertOrderedList",
        InsertUnorderedList => "insertUnorderedList",
        InsertHorizontalRule => "insertHorizontalRule",
        InsertFromYank => "insertFromYank",
        InsertFromDrop => "insertFromDrop",
        InsertFromPaste => "insertFromPaste",
        InsertFromPasteAsQuotation => "insertFromPasteAsQuotation",
        InsertTranspose => "insertTranspose",
        InsertCompositionText => "insertCompositionText",
        InsertLink => "insertLink",
        DeleteWordBackward => "deleteWordBackward",
        DeleteWordForward => "deleteWordForward",
        DeleteSoftLineBackward => "deleteSoftLineBackward",
        DeleteSoftLineForward => "deleteSoftLineForward",
        DeleteEntireSoftLine => "deleteEntireSoftLine",
        DeleteHardLineBackward => "deleteHardLineBackward",
        DeleteHardLineForward => "deleteHardLineForward",
        DeleteByDrag => "deleteByDrag",
        DeleteByCut => "deleteByCut",
        DeleteContent => "deleteContent",
        DeleteContentBackward => "deleteContentBackward",
        DeleteContentForward => "deleteContentForward",
        HistoryUndo => "historyUndo",
        HistoryRedo => "historyRedo",
        FormatBold => "formatBold",
        FormatItalic => "formatItalic",
        FormatUnderline => "formatUnderline",
        FormatStrikeThrough => "formatStrikeThrough",
        FormatSuperscript => "formatSuperscript",
        FormatSubscript => "formatSubscript",
        FormatJustifyFull => "formatJustifyFull",
        FormatJustifyCenter => "formatJustifyCenter",
        FormatJustifyRight => "formatJustifyRight",
        FormatJustifyLeft => "formatJustifyLeft",
        FormatIndent => "formatIndent",
        FormatOutdent => "formatOutdent",
        FormatRemove => "formatRemove",
        FormatSetBlockTextDirection => "formatSetBlockTextDirection",
        FormatSetInlineTextDirection => "formatSetInlineTextDirection",
        FormatBackColor => "formatBackColor",
        FormatFontColor => "formatFontColor",
        FormatFontName => "formatFontName",
        /// An `inputType` value not covered by the known variants, preserved as-is.
        _ => Unknown(String),
    }
}

impl From<String> for InputType {
    fn from(value: String) -> Self {
        // Reuse the `&str` mapping; only `Unknown` needs to take ownership, and in
        // that case the borrowed match already produced an owned copy.
        InputType::from(value.as_str())
    }
}

impl From<InputType> for String {
    fn from(value: InputType) -> Self {
        match value {
            InputType::Unknown(value) => value,
            other => other.as_str().to_string(),
        }
    }
}

impl Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

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
    /// Returns an [`InputType`] so the common cases (e.g. [`InputType::InsertText`],
    /// [`InputType::DeleteContentBackward`]) can be matched on directly. Values not
    /// covered by a known variant are preserved in [`InputType::Unknown`]. The full
    /// list is documented at
    /// <https://w3c.github.io/input-events/#interface-InputEvent-Attributes>.
    pub fn input_type(&self) -> InputType {
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
    fn input_type(&self) -> InputType;

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
        fn input_type(&self) -> InputType {
            InputType::from(self.input_type.as_str())
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
                input_type: data.input_type().to_string(),
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

        assert_eq!(event.input_type(), InputType::InsertText);
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
    fn input_type_maps_known_values_and_preserves_unknown() {
        assert_eq!(InputType::from("insertText"), InputType::InsertText);
        assert_eq!(
            InputType::from("deleteContentBackward"),
            InputType::DeleteContentBackward
        );

        // Known variants round-trip back to their canonical DOM string.
        assert_eq!(InputType::InsertFromPaste.as_str(), "insertFromPaste");
        assert_eq!(InputType::InsertFromPaste.to_string(), "insertFromPaste");

        // Unrecognized values are preserved verbatim through the round-trip.
        let unknown = InputType::from("insertSomethingNew");
        assert_eq!(
            unknown,
            InputType::Unknown("insertSomethingNew".to_string())
        );
        assert_eq!(unknown.as_str(), "insertSomethingNew");
        assert_eq!(String::from(unknown), "insertSomethingNew");
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
