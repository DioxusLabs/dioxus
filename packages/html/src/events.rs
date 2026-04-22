#![doc = include_str!("../docs/event_handlers.md")]

pub use html_events::with_html_event_groups;
pub use html_events::*;

pub type AnimationEvent = dioxus_core::Event<AnimationData>;
pub type CancelEvent = dioxus_core::Event<CancelData>;
pub type ClipboardEvent = dioxus_core::Event<ClipboardData>;
pub type CompositionEvent = dioxus_core::Event<CompositionData>;
pub type DragEvent = dioxus_core::Event<DragData>;
pub type FocusEvent = dioxus_core::Event<FocusData>;
pub type FormEvent = dioxus_core::Event<FormData>;
pub type ImageEvent = dioxus_core::Event<ImageData>;
pub type KeyboardEvent = dioxus_core::Event<KeyboardData>;
pub type MediaEvent = dioxus_core::Event<MediaData>;
pub type MountedEvent = dioxus_core::Event<MountedData>;
pub type MouseEvent = dioxus_core::Event<MouseData>;
/// A synthetic event that wraps a web-style [`PointerEvent`](https://developer.mozilla.org/en-US/docs/Web/API/PointerEvent)
pub type PointerEvent = dioxus_core::Event<PointerData>;
pub type ResizeEvent = dioxus_core::Event<ResizeData>;
pub type ScrollEvent = dioxus_core::Event<ScrollData>;
pub type SelectionEvent = dioxus_core::Event<SelectionData>;
pub type ToggleEvent = dioxus_core::Event<ToggleData>;
pub type TouchEvent = dioxus_core::Event<TouchData>;
pub type TransitionEvent = dioxus_core::Event<TransitionData>;
pub type VisibleEvent = dioxus_core::Event<VisibleData>;
/// A synthetic event that wraps a web-style
/// [`WheelEvent`](https://developer.mozilla.org/en-US/docs/Web/API/WheelEvent)
pub type WheelEvent = dioxus_core::Event<WheelData>;

macro_rules! impl_event {
    (
        $data:ty;
        $(
            $( #[$attr:meta] )*
            $name:ident $( : $js_name:expr )?;
        )*
    ) => {
        $(
            $( #[$attr] )*
            /// <details open>
            /// <summary>General Event Handler Information</summary>
            ///
            #[doc = include_str!("../docs/event_handlers.md")]
            ///
            /// </details>
            ///
            #[doc = include_str!("../docs/common_event_handler_errors.md")]
            #[inline]
            pub fn $name<__Marker>(mut _f: impl ::dioxus_core::SuperInto<::dioxus_core::ListenerCallback<$data>, __Marker>) -> ::dioxus_core::Attribute {
                let event_handler = _f.super_into();
                ::dioxus_core::Attribute::new(
                    impl_event!(@name $name $($js_name)?),
                    ::dioxus_core::AttributeValue::listener(move |e: ::dioxus_core::Event<crate::PlatformEventData>| {
                        let event: ::dioxus_core::Event<$data> = e.map(|data| {
                            data.into()
                        });
                        event_handler.call(event.into_any());
                    }),
                    None,
                    false,
                ).into()
            }

            #[doc(hidden)]
            $( #[$attr] )*
            pub mod $name {
                use super::*;

                // When expanding the macro, we use this version of the function if we see an inline closure to give better type inference
                $( #[$attr] )*
                pub fn call_with_explicit_closure<
                    __Marker,
                    Return: ::dioxus_core::SpawnIfAsync<__Marker> + 'static,
                >(
                    event_handler: impl FnMut(::dioxus_core::Event<$data>) -> Return + 'static,
                ) -> ::dioxus_core::Attribute {
                    #[allow(deprecated)]
                    super::$name(event_handler)
                }
            }
        )*
    };

    (@name $name:ident $js_name:expr) => {
        $js_name
    };
    (@name $name:ident) => {
        stringify!($name)
    };
}

macro_rules! expand_html_event_listeners {
    (
        enum Event {
            $(
                #[convert = $converter:ident]
                #[events = [
                    $(
                        $( #[$attr:meta] )*
                        $name:ident => $raw:ident,
                    )*
                ]]
                $(#[raw = [$($raw_only:ident),* $(,)?]])?
                $group:ident($data:ident),
            )*
        }
    ) => {
        $(
            impl_event! {
                $data;
                $(
                    #[doc = concat!(stringify!($name))]
                    $( #[$attr] )*
                    $name: concat!("on", stringify!($raw));
                )*
            }
        )*
    };
}

with_html_event_groups!(expand_html_event_listeners);
