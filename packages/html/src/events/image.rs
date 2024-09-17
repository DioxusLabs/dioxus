use dioxus_core::Event;

pub type ImageEvent = Event<ImageData>;
pub struct ImageData {
    inner: Box<dyn HasImageData>,
}

impl<E: HasImageData> From<E> for ImageData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for ImageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageData")
            .field("load_error", &self.load_error())
            .finish()
    }
}

impl PartialEq for ImageData {
    fn eq(&self, other: &Self) -> bool {
        self.load_error() == other.load_error()
    }
}

impl ImageData {
    /// Create a new ImageData
    pub fn new(e: impl HasImageData) -> Self {
        Self { inner: Box::new(e) }
    }

    /// If the renderer encountered an error while loading the image
    pub fn load_error(&self) -> bool {
        self.inner.load_error()
    }

    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of ImageData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedImageData {
    load_error: bool,
}

#[cfg(feature = "serialize")]
impl From<&ImageData> for SerializedImageData {
    fn from(data: &ImageData) -> Self {
        Self {
            load_error: data.load_error(),
        }
    }
}

#[cfg(feature = "serialize")]
impl HasImageData for SerializedImageData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn load_error(&self) -> bool {
        self.load_error
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for ImageData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedImageData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for ImageData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedImageData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

/// A trait for any object that has the data for an image event
pub trait HasImageData: std::any::Any {
    /// If the renderer encountered an error while loading the image
    fn load_error(&self) -> bool;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! [
    ImageData;

    /// onerror
    onerror

    /// onload
    onload
];
