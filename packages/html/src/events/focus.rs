use dioxus_core_types::Event;

pub type FocusEvent = Event<FocusData>;

pub struct FocusData {
    inner: Box<dyn HasFocusData>,
}

impl<E: HasFocusData> From<E> for FocusData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for FocusData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FocusData").finish()
    }
}

impl PartialEq for FocusData {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl FocusData {
    /// Create a new FocusData
    pub fn new(inner: impl HasFocusData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Downcast this event data to a specific type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of FocusData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone, Default)]
pub struct SerializedFocusData {}

#[cfg(feature = "serialize")]
impl From<&FocusData> for SerializedFocusData {
    fn from(_: &FocusData) -> Self {
        Self {}
    }
}

#[cfg(feature = "serialize")]
impl HasFocusData for SerializedFocusData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for FocusData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedFocusData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for FocusData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedFocusData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasFocusData: std::any::Any {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! [
    FocusData;

    /// onfocus
    onfocus

    // onfocusout
    onfocusout

    // onfocusin
    onfocusin

    /// onblur
    onblur
];
