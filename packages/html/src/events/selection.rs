use dioxus_core::Event;

pub type SelectionEvent = Event<SelectionData>;

pub struct SelectionData {
    inner: Box<dyn HasSelectionData>,
}

impl SelectionData {
    /// Create a new SelectionData
    pub fn new(inner: impl HasSelectionData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// The start offset of the selected text in a text control.
    ///
    /// This is measured in UTF-16 code units and is only available for text
    /// controls that expose `selectionStart`.
    pub fn selection_start(&self) -> Option<usize> {
        self.inner.selection_start()
    }

    /// The end offset of the selected text in a text control.
    ///
    /// This is measured in UTF-16 code units and is only available for text
    /// controls that expose `selectionEnd`.
    pub fn selection_end(&self) -> Option<usize> {
        self.inner.selection_end()
    }

    /// The direction of the selection in a text control.
    ///
    /// Browsers return `"forward"`, `"backward"`, or `"none"` when this data is
    /// available.
    pub fn selection_direction(&self) -> Option<String> {
        self.inner.selection_direction()
    }

    /// The selected text, if it can be read from the event target or document.
    pub fn selected_text(&self) -> String {
        self.inner.selected_text()
    }

    /// The anchor offset of the current document selection.
    pub fn anchor_offset(&self) -> Option<usize> {
        self.inner.anchor_offset()
    }

    /// The focus offset of the current document selection.
    pub fn focus_offset(&self) -> Option<usize> {
        self.inner.focus_offset()
    }

    /// Whether the current document selection is collapsed.
    pub fn is_collapsed(&self) -> Option<bool> {
        self.inner.is_collapsed()
    }

    /// The number of ranges in the current document selection.
    pub fn range_count(&self) -> Option<usize> {
        self.inner.range_count()
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl<E: HasSelectionData> From<E> for SelectionData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for SelectionData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectionData")
            .field("selection_start", &self.selection_start())
            .field("selection_end", &self.selection_end())
            .field("selection_direction", &self.selection_direction())
            .field("selected_text", &self.selected_text())
            .field("anchor_offset", &self.anchor_offset())
            .field("focus_offset", &self.focus_offset())
            .field("is_collapsed", &self.is_collapsed())
            .field("range_count", &self.range_count())
            .finish()
    }
}

impl PartialEq for SelectionData {
    fn eq(&self, other: &Self) -> bool {
        self.selection_start() == other.selection_start()
            && self.selection_end() == other.selection_end()
            && self.selection_direction() == other.selection_direction()
            && self.selected_text() == other.selected_text()
            && self.anchor_offset() == other.anchor_offset()
            && self.focus_offset() == other.focus_offset()
            && self.is_collapsed() == other.is_collapsed()
            && self.range_count() == other.range_count()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of SelectionData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedSelectionData {
    #[serde(default)]
    pub selection_start: Option<usize>,
    #[serde(default)]
    pub selection_end: Option<usize>,
    #[serde(default)]
    pub selection_direction: Option<String>,
    #[serde(default)]
    pub selected_text: String,
    #[serde(default)]
    pub anchor_offset: Option<usize>,
    #[serde(default)]
    pub focus_offset: Option<usize>,
    #[serde(default)]
    pub is_collapsed: Option<bool>,
    #[serde(default)]
    pub range_count: Option<usize>,
}

#[cfg(feature = "serialize")]
impl SerializedSelectionData {
    /// Create a new serialized selection data object.
    pub fn new(
        selection_start: Option<usize>,
        selection_end: Option<usize>,
        selection_direction: Option<String>,
        selected_text: String,
        anchor_offset: Option<usize>,
        focus_offset: Option<usize>,
        is_collapsed: Option<bool>,
        range_count: Option<usize>,
    ) -> Self {
        Self {
            selection_start,
            selection_end,
            selection_direction,
            selected_text,
            anchor_offset,
            focus_offset,
            is_collapsed,
            range_count,
        }
    }
}

#[cfg(feature = "serialize")]
impl Default for SerializedSelectionData {
    fn default() -> Self {
        Self::new(None, None, None, String::new(), None, None, None, None)
    }
}

#[cfg(feature = "serialize")]
impl From<&SelectionData> for SerializedSelectionData {
    fn from(data: &SelectionData) -> Self {
        Self::new(
            data.selection_start(),
            data.selection_end(),
            data.selection_direction(),
            data.selected_text(),
            data.anchor_offset(),
            data.focus_offset(),
            data.is_collapsed(),
            data.range_count(),
        )
    }
}

#[cfg(feature = "serialize")]
impl HasSelectionData for SerializedSelectionData {
    fn selection_start(&self) -> Option<usize> {
        self.selection_start
    }

    fn selection_end(&self) -> Option<usize> {
        self.selection_end
    }

    fn selection_direction(&self) -> Option<String> {
        self.selection_direction.clone()
    }

    fn selected_text(&self) -> String {
        self.selected_text.clone()
    }

    fn anchor_offset(&self) -> Option<usize> {
        self.anchor_offset
    }

    fn focus_offset(&self) -> Option<usize> {
        self.focus_offset
    }

    fn is_collapsed(&self) -> Option<bool> {
        self.is_collapsed
    }

    fn range_count(&self) -> Option<usize> {
        self.range_count
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for SelectionData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedSelectionData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for SelectionData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedSelectionData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasSelectionData: std::any::Any {
    /// The start offset of the selected text in a text control.
    fn selection_start(&self) -> Option<usize> {
        None
    }

    /// The end offset of the selected text in a text control.
    fn selection_end(&self) -> Option<usize> {
        None
    }

    /// The direction of the selection in a text control.
    fn selection_direction(&self) -> Option<String> {
        None
    }

    /// The selected text.
    fn selected_text(&self) -> String {
        String::new()
    }

    /// The anchor offset of the current document selection.
    fn anchor_offset(&self) -> Option<usize> {
        None
    }

    /// The focus offset of the current document selection.
    fn focus_offset(&self) -> Option<usize> {
        None
    }

    /// Whether the current document selection is collapsed.
    fn is_collapsed(&self) -> Option<bool> {
        None
    }

    /// The number of ranges in the current document selection.
    fn range_count(&self) -> Option<usize> {
        None
    }

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(all(test, feature = "serialize"))]
mod tests {
    use super::*;

    #[test]
    fn serialized_selection_data_deserializes_missing_fields() {
        let data: SerializedSelectionData = serde_json::from_str("{}").unwrap();
        assert_eq!(data, SerializedSelectionData::default());
    }

    #[test]
    fn selection_data_exposes_serialized_fields() {
        let event = SelectionData::new(SerializedSelectionData::new(
            Some(1),
            Some(4),
            Some("forward".to_string()),
            "abc".to_string(),
            Some(2),
            Some(5),
            Some(false),
            Some(1),
        ));

        assert_eq!(event.selection_start(), Some(1));
        assert_eq!(event.selection_end(), Some(4));
        assert_eq!(event.selection_direction().as_deref(), Some("forward"));
        assert_eq!(event.selected_text(), "abc");
        assert_eq!(event.anchor_offset(), Some(2));
        assert_eq!(event.focus_offset(), Some(5));
        assert_eq!(event.is_collapsed(), Some(false));
        assert_eq!(event.range_count(), Some(1));
    }
}
