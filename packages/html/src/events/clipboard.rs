use dioxus_core::Event;

use crate::data_transfer::{DataTransfer, HasDataTransferData};
use crate::file_data::{FileData, HasFileData};

pub type ClipboardEvent = Event<ClipboardData>;

/// Data fired alongside the `copy`, `cut` and `paste` events.
///
/// Clipboard events expose the data being transferred through the system clipboard via a
/// [`DataTransfer`] object — the same abstraction used by drag-and-drop. For a `paste`, read
/// the incoming payload with [`ClipboardData::data_transfer`] (e.g.
/// [`DataTransfer::get_as_text`] for text, or [`HasFileData::files`] for pasted files).
///
/// See <https://developer.mozilla.org/en-US/docs/Web/API/ClipboardEvent> for the underlying
/// DOM event.
pub struct ClipboardData {
    inner: Box<dyn HasClipboardData>,
}

impl<E: HasClipboardData> From<E> for ClipboardData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for ClipboardData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dt = self.data_transfer();
        f.debug_struct("ClipboardData")
            .field("text", &dt.get_as_text())
            .field("files", &dt.files().len())
            .finish()
    }
}

impl PartialEq for ClipboardData {
    fn eq(&self, other: &Self) -> bool {
        // `data_transfer()` rebuilds (and on serialized renderers clones) the transfer,
        // so grab each side once.
        let (this, that) = (self.data_transfer(), other.data_transfer());
        this.get_as_text() == that.get_as_text()
    }
}

impl ClipboardData {
    /// Create a new ClipboardData
    pub fn new(inner: impl HasClipboardData) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// The [`DataTransfer`] holding the data carried by the clipboard event.
    ///
    /// On `paste` this exposes the data being pasted (read it with
    /// [`DataTransfer::get_as_text`], [`DataTransfer::get_data`] or [`HasFileData::files`]).
    pub fn data_transfer(&self) -> DataTransfer {
        self.inner.data_transfer()
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_ref().as_any().downcast_ref::<T>()
    }
}

impl HasFileData for ClipboardData {
    fn files(&self) -> Vec<FileData> {
        self.inner.files()
    }
}

pub trait HasClipboardData: HasDataTransferData + HasFileData {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(feature = "serialize")]
pub use ser::*;

#[cfg(feature = "serialize")]
mod ser {
    use super::*;
    use crate::data_transfer::SerializedDataTransfer;

    /// A serialized version of [`ClipboardData`] used to transport the event across the
    /// IPC boundary on non-web renderers.
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    pub struct SerializedClipboardData {
        pub data_transfer: SerializedDataTransfer,
    }

    impl SerializedClipboardData {
        fn new(clipboard: &ClipboardData) -> Self {
            let data_transfer = clipboard.data_transfer();
            Self {
                data_transfer: SerializedDataTransfer::from_data_transfer(&data_transfer),
            }
        }
    }

    impl HasDataTransferData for SerializedClipboardData {
        fn data_transfer(&self) -> DataTransfer {
            DataTransfer::new(self.data_transfer.clone())
        }
    }

    impl HasFileData for SerializedClipboardData {
        fn files(&self) -> Vec<FileData> {
            self.data_transfer().files()
        }
    }

    impl HasClipboardData for SerializedClipboardData {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    impl serde::Serialize for ClipboardData {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            SerializedClipboardData::new(self).serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for ClipboardData {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let data = SerializedClipboardData::deserialize(deserializer)?;
            Ok(Self {
                inner: Box::new(data),
            })
        }
    }
}

#[cfg(all(test, feature = "serialize"))]
mod tests {
    use super::*;

    /// The browser-side serializer (packages/interpreter/src/ts/serialize.ts) emits a
    /// `data_transfer` object for clipboard events, mirroring the drag serializer. Pin the
    /// shape so a change on either side breaks the test instead of silently dropping the
    /// pasted data.
    #[test]
    fn deserializes_paste_payload_from_js_interpreter() {
        let payload = r#"{
            "data_transfer": {
                "items": [
                    { "kind": "string", "type_": "text/plain", "data": "hello pasted" }
                ],
                "files": [],
                "effect_allowed": "uninitialized",
                "drop_effect": "none"
            }
        }"#;

        let data: ClipboardData = serde_json::from_str(payload).unwrap();
        assert_eq!(
            data.data_transfer().get_as_text().as_deref(),
            Some("hello pasted")
        );
        assert_eq!(
            data.data_transfer().get_data("text/plain").as_deref(),
            Some("hello pasted")
        );
        assert!(data.files().is_empty());
    }

    #[test]
    fn paste_text_round_trips_through_serde() {
        let payload = r#"{
            "data_transfer": {
                "items": [
                    { "kind": "string", "type_": "text/plain", "data": "round trip" }
                ],
                "files": [],
                "effect_allowed": "",
                "drop_effect": ""
            }
        }"#;

        let data: ClipboardData = serde_json::from_str(payload).unwrap();
        let json = serde_json::to_string(&data).unwrap();
        let back: ClipboardData = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.data_transfer().get_as_text().as_deref(),
            Some("round trip")
        );
    }
}
