use dioxus_core::Event;
use std::ops::Range;

pub type SelectionEvent = Event<SelectionData>;

/// The direction a text selection was created in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(rename_all = "lowercase"))]
pub enum SelectionDirection {
    /// The selection direction is unknown or directionless.
    #[default]
    None,
    /// The focus is after the anchor.
    Forward,
    /// The focus is before the anchor.
    Backward,
}

/// The selection inside a text control.
///
/// The range is measured in UTF-16 code units to match the browser selection
/// APIs on `input` and `textarea`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextSelection {
    range: Range<usize>,
    direction: SelectionDirection,
}

impl TextSelection {
    /// Create a new text selection from a UTF-16 range and selection direction.
    pub fn new(range: Range<usize>, direction: SelectionDirection) -> Self {
        Self { range, direction }
    }

    /// The selected UTF-16 range.
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// The direction the range was selected in.
    pub fn direction(&self) -> SelectionDirection {
        self.direction
    }

    /// Returns `true` if the selection is a caret with no selected text.
    pub fn is_collapsed(&self) -> bool {
        self.range.is_empty()
    }
}

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

    /// The selection inside a text control.
    ///
    /// This is only populated for event targets that expose the text-control
    /// selection APIs, such as `input` and `textarea` on the web. Some
    /// selection events, notably `selectstart`, can also fire when selecting
    /// normal document text. Those document selections are exposed by browser
    /// APIs like `document.getSelection()` and intentionally return `None` here.
    pub fn selection(&self) -> Option<TextSelection> {
        self.inner.selection()
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
            .field("selection", &self.selection())
            .finish()
    }
}

impl PartialEq for SelectionData {
    fn eq(&self, other: &Self) -> bool {
        self.selection() == other.selection()
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
    pub selection_direction: Option<SelectionDirection>,
}

#[cfg(feature = "serialize")]
impl SerializedSelectionData {
    /// Create a new serialized selection data object.
    pub fn new(
        selection_start: Option<usize>,
        selection_end: Option<usize>,
        selection_direction: Option<SelectionDirection>,
    ) -> Self {
        Self {
            selection_start,
            selection_end,
            selection_direction,
        }
    }
}

#[cfg(feature = "serialize")]
impl Default for SerializedSelectionData {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

#[cfg(feature = "serialize")]
impl From<&SelectionData> for SerializedSelectionData {
    fn from(data: &SelectionData) -> Self {
        let Some(selection) = data.selection() else {
            return Self::default();
        };
        let range = selection.range();
        Self::new(
            Some(range.start),
            Some(range.end),
            Some(selection.direction()),
        )
    }
}

#[cfg(feature = "serialize")]
impl HasSelectionData for SerializedSelectionData {
    fn selection(&self) -> Option<TextSelection> {
        let start = self.selection_start?;
        let end = self.selection_end?;
        Some(TextSelection::new(
            start..end,
            self.selection_direction.unwrap_or_default(),
        ))
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
    /// The selection inside a text control.
    ///
    /// Return `None` when the event did not originate from a text control with
    /// selection offsets. Document selections should use a separate API instead
    /// of being mixed into this payload.
    fn selection(&self) -> Option<TextSelection> {
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
            Some(SelectionDirection::Forward),
        ));

        assert_eq!(
            event.selection(),
            Some(TextSelection::new(1..4, SelectionDirection::Forward))
        );
    }
}
