use dioxus_core_types::Event;

pub type ClipboardEvent = Event<ClipboardData>;

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
        f.debug_struct("ClipboardData").finish()
    }
}

impl PartialEq for ClipboardData {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl ClipboardData {
    /// Create a new ClipboardData
    pub fn new(inner: impl HasClipboardData) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_ref().as_any().downcast_ref::<T>()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of ClipboardData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedClipboardData {}

#[cfg(feature = "serialize")]
impl From<&ClipboardData> for SerializedClipboardData {
    fn from(_: &ClipboardData) -> Self {
        Self {}
    }
}

#[cfg(feature = "serialize")]
impl HasClipboardData for SerializedClipboardData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for ClipboardData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedClipboardData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for ClipboardData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedClipboardData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasClipboardData: std::any::Any {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event![
    ClipboardData;

    /// oncopy
    oncopy

    /// oncut
    oncut

    /// onpaste
    onpaste
];
