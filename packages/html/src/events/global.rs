use super::PlatformEventData;
use dioxus_vdom::{Event, SpawnIfAsync};
use std::{cell::RefCell, rc::Rc};

/// Relationship of the original event target to the requested containment root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize))]
pub enum Containment {
    /// The target is the root or inside it.
    Inside,
    /// The root exists, but the target is outside it.
    Outside,
    /// No element with the requested ID exists.
    RootMissing,
}

/// Information computed by the renderer before a global listener runs.
#[derive(Clone, Copy, Debug)]
pub struct GlobalListenerContext(Containment);

impl GlobalListenerContext {
    #[doc(hidden)]
    pub fn new(containment: Containment) -> Self {
        Self(containment)
    }

    /// Return the relationship to the element named by `containment_root`.
    pub fn containment(&self) -> Containment {
        self.0
    }
}

/// The layout viewport dimensions in CSS pixels for a window resize event.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize))]
pub struct ViewportResizeData {
    width: f64,
    height: f64,
}

impl ViewportResizeData {
    #[doc(hidden)]
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// The viewport width in CSS pixels.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// The viewport height in CSS pixels.
    pub fn height(&self) -> f64 {
        self.height
    }
}

impl From<&PlatformEventData> for ViewportResizeData {
    fn from(data: &PlatformEventData) -> Self {
        *data
            .downcast::<Self>()
            .expect("expected viewport dimensions")
    }
}

#[doc(hidden)]
pub struct GlobalEventInput {
    pub payload: GlobalEventPayload,
    pub containment: Containment,
    pub bubbles: bool,
    pub cancelable: bool,
    pub default_prevented: bool,
}

#[doc(hidden)]
pub enum GlobalEventPayload {
    Platform(PlatformEventData),
    #[cfg(feature = "serialize")]
    Serialized(serde_json::Value),
}

#[doc(hidden)]
#[derive(Default)]
pub struct GlobalEventResult {
    pub prevent_default: bool,
    pub stop_propagation: bool,
}

#[doc(hidden)]
pub type GlobalEventCallback = Rc<dyn Fn(GlobalEventInput) -> GlobalEventResult>;

#[doc(hidden)]
pub struct GlobalEventBinding {
    pub name: &'static str,
    pub callback: GlobalEventCallback,
}

pub(super) trait GlobalPayload: for<'a> From<&'a PlatformEventData> + 'static {
    fn decode(payload: GlobalEventPayload) -> Option<Self>;
}

#[cfg(feature = "serialize")]
impl<T: for<'a> From<&'a PlatformEventData> + serde::de::DeserializeOwned + 'static> GlobalPayload
    for T
{
    fn decode(payload: GlobalEventPayload) -> Option<Self> {
        match payload {
            GlobalEventPayload::Platform(data) => Some(Self::from(&data)),
            GlobalEventPayload::Serialized(data) => serde_json::from_value(data)
                .inspect_err(|error| tracing::debug!(%error, "Ignoring malformed global event"))
                .ok(),
        }
    }
}

#[cfg(not(feature = "serialize"))]
impl<T: for<'a> From<&'a PlatformEventData> + 'static> GlobalPayload for T {
    fn decode(payload: GlobalEventPayload) -> Option<Self> {
        let GlobalEventPayload::Platform(data) = payload;
        Some(Self::from(&data))
    }
}

pub(super) fn bind<Data, R, Marker>(
    name: &'static str,
    handler: impl FnMut(Event<Data>, GlobalListenerContext) -> R + 'static,
) -> GlobalEventBinding
where
    Data: GlobalPayload,
    R: SpawnIfAsync<Marker>,
{
    let handler = RefCell::new(handler);
    let callback = Rc::new(move |input: GlobalEventInput| {
        let Ok(mut handler) = handler.try_borrow_mut() else {
            return GlobalEventResult::default();
        };
        let Some(data) = Data::decode(input.payload) else {
            return GlobalEventResult::default();
        };
        let event = Event::new_with_flags(
            Rc::new(data),
            input.bubbles,
            input.cancelable,
            input.default_prevented,
        );
        handler(event.clone(), GlobalListenerContext::new(input.containment)).spawn();
        GlobalEventResult {
            prevent_default: !input.default_prevented && !event.default_action_enabled(),
            stop_propagation: event.propagation_stopped(),
        }
    });
    GlobalEventBinding { name, callback }
}
