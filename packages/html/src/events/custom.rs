use dioxus_vdom::Event;

use super::EventHandlerValue;

/// An event dispatched in the page under a name registered with [`register_event!`](crate::register_event).
pub type CustomEvent = Event<CustomData>;

/// The data of a custom event: the `detail` it was dispatched with, when that is a string.
pub struct CustomData {
    inner: Box<dyn HasCustomData>,
}

impl CustomData {
    /// Create a new CustomData
    pub fn new(inner: impl HasCustomData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// The `detail` the event was dispatched with, or `None` when it was not a string.
    pub fn detail(&self) -> Option<String> {
        self.inner.detail()
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl<E: HasCustomData> From<E> for CustomData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for CustomData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomData")
            .field("detail", &self.detail())
            .finish()
    }
}

impl PartialEq for CustomData {
    fn eq(&self, other: &Self) -> bool {
        self.detail() == other.detail()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of CustomData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone, Default)]
pub struct SerializedCustomData {
    #[serde(default)]
    detail: Option<String>,
}

#[cfg(feature = "serialize")]
impl SerializedCustomData {
    /// Create a new SerializedCustomData
    pub fn new(detail: Option<String>) -> Self {
        Self { detail }
    }
}

#[cfg(feature = "serialize")]
impl From<&CustomData> for SerializedCustomData {
    fn from(data: &CustomData) -> Self {
        Self {
            detail: data.detail(),
        }
    }
}

#[cfg(feature = "serialize")]
impl HasCustomData for SerializedCustomData {
    fn detail(&self) -> Option<String> {
        self.detail.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for CustomData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedCustomData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for CustomData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedCustomData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

/// A trait for any object that has the data for a custom event
pub trait HasCustomData: std::any::Any {
    /// The `detail` the event was dispatched with, or `None` when it was not a string.
    fn detail(&self) -> Option<String>;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Register a custom event by the name it is dispatched with in the page, and whether it bubbles.
///
/// Handlers made by [`register_event!`](crate::register_event) call this as they are attached, so
/// their listeners bubble as registered and the events they hear are read as [`CustomData`].
///
/// # Panics
///
/// Panics if `name` is the name of an event Dioxus already handles, or if it was registered before
/// with different bubbling.
#[doc(hidden)]
pub fn register_custom_event(name: &'static str, bubbles: bool) {
    assert!(
        !super::is_built_in_event(name),
        "\"{name}\" is the name of an event Dioxus already handles, so it cannot be registered as a custom event."
    );
    crate::bubbles::register_custom_event(name, bubbles);
}

/// The listener attribute for a handler made by [`register_event!`](crate::register_event).
#[doc(hidden)]
pub fn custom_event_attribute<Marker>(
    listener: &'static str,
    event_handler: impl EventHandlerValue<CustomData, Marker>,
) -> dioxus_vdom::Attribute {
    super::event_attribute::<CustomData, Marker>(listener, event_handler)
}

/// Give an event dispatched in the page under a name of your own a typed handler that rsx attaches
/// like any built-in one.
///
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// dioxus::html::register_event! {
///     event: onmyevent;
///     js-event: "my-event";
///     bubbles: true;
/// }
///
/// fn app() -> Element {
///     rsx! {
///         div {
///             onmyevent: move |event| println!("{:?}", event.detail()),
///         }
///     }
/// }
/// ```
///
/// `js-event` is the name the event is dispatched under in the page: a string literal, or any
/// `&'static str` constant, such as one shared with the code that dispatches the event.
///
/// The handler hears a [`CustomData`], whose [`detail`](CustomData::detail) is the `detail` of the
/// `CustomEvent` dispatched in the page when it is a string, such as a message serialized as JSON.
/// The macro defines a trait named after the handler; bring it into scope wherever rsx attaches
/// the handler. A bubbling event is heard by a handler on any element it bubbles through, so it
/// must be dispatched with `bubbles: true`; a non-bubbling one is heard only on the element it is
/// dispatched on.
#[macro_export]
macro_rules! register_event {
    ($($input:tt)*) => {
        $crate::__register_event! { $crate; $($input)* }
    };
}
