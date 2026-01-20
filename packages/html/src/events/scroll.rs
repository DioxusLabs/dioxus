use dioxus_core::Event;

pub type ScrollEvent = Event<ScrollData>;

pub struct ScrollData {
    inner: Box<dyn HasScrollData>,
}

impl<E: HasScrollData> From<E> for ScrollData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl ScrollData {
    /// Create a new ScrollData
    pub fn new(inner: impl HasScrollData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }

    pub fn scroll_top(&self) -> f64 {
        self.inner.scroll_top()
    }

    pub fn scroll_left(&self) -> f64 {
        self.inner.scroll_left()
    }

    pub fn scroll_width(&self) -> i32 {
        self.inner.scroll_width()
    }

    pub fn scroll_height(&self) -> i32 {
        self.inner.scroll_height()
    }

    pub fn client_width(&self) -> i32 {
        self.inner.client_width()
    }

    pub fn client_height(&self) -> i32 {
        self.inner.client_height()
    }
}

impl std::fmt::Debug for ScrollData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScrollData")
            .field("scroll_top", &self.scroll_top())
            .field("scroll_left", &self.scroll_left())
            .field("scroll_width", &self.scroll_width())
            .field("scroll_height", &self.scroll_height())
            .field("client_width", &self.client_width())
            .field("client_height", &self.client_height())
            .finish()
    }
}

impl PartialEq for ScrollData {
    fn eq(&self, other: &Self) -> bool {
        self.scroll_top() == other.scroll_top()
            && self.scroll_left() == other.scroll_left()
            && self.scroll_width() == other.scroll_width()
            && self.scroll_height() == other.scroll_height()
            && self.client_width() == other.client_width()
            && self.client_height() == other.client_height()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of ScrollData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedScrollData {
    pub scroll_top: f64,
    pub scroll_left: f64,
    pub scroll_width: i32,
    pub scroll_height: i32,
    pub client_width: i32,
    pub client_height: i32,
}

#[cfg(feature = "serialize")]
impl From<&ScrollData> for SerializedScrollData {
    fn from(data: &ScrollData) -> Self {
        Self {
            scroll_top: data.inner.scroll_top(),
            scroll_left: data.inner.scroll_left(),
            scroll_width: data.inner.scroll_width(),
            scroll_height: data.inner.scroll_height(),
            client_width: data.inner.client_width(),
            client_height: data.inner.client_height(),
        }
    }
}

#[cfg(feature = "serialize")]
impl HasScrollData for SerializedScrollData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn scroll_top(&self) -> f64 {
        self.scroll_top
    }

    fn scroll_left(&self) -> f64 {
        self.scroll_left
    }

    fn scroll_width(&self) -> i32 {
        self.scroll_width
    }

    fn scroll_height(&self) -> i32 {
        self.scroll_height
    }

    fn client_width(&self) -> i32 {
        self.client_width
    }

    fn client_height(&self) -> i32 {
        self.client_height
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for ScrollData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedScrollData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for ScrollData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedScrollData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasScrollData: std::any::Any {
    /// Return self as Any
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get the vertical scroll position
    fn scroll_top(&self) -> f64;

    /// Get the horizontal scroll position
    fn scroll_left(&self) -> f64;

    /// Get the total scrollable width
    fn scroll_width(&self) -> i32;

    /// Get the total scrollable height
    fn scroll_height(&self) -> i32;

    /// Get the viewport width
    fn client_width(&self) -> i32;

    /// Get the viewport height
    fn client_height(&self) -> i32;
}

impl_event! {
    ScrollData;

    /// onscroll
    onscroll

    /// onscrollend
    onscrollend
}
