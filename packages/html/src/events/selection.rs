use dioxus_core_types::Event;

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
        f.debug_struct("SelectionData").finish()
    }
}

impl PartialEq for SelectionData {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of SelectionData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedSelectionData {}

#[cfg(feature = "serialize")]
impl From<&SelectionData> for SerializedSelectionData {
    fn from(_: &SelectionData) -> Self {
        Self {}
    }
}

#[cfg(feature = "serialize")]
impl HasSelectionData for SerializedSelectionData {
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
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! [
    SelectionData;

    /// select
    onselect

    /// selectstart
    onselectstart

    /// selectionchange
    onselectionchange
];
