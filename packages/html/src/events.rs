mod keys;
pub use keys::*;

macro_rules! impl_event {
    (
        $data:ty;
        $(
            $( #[$attr:meta] )*
            $name:ident
        )*
    ) => {
        $(
            $( #[$attr] )*
            pub fn $name<'a>(_cx: &'a ::dioxus_core::ScopeState, _f: impl FnMut(::dioxus_core::Event<$data>) + 'a) -> ::dioxus_core::Attribute<'a> {
                ::dioxus_core::Attribute {
                    name: stringify!($name),
                    value: ::dioxus_core::AttributeValue::new_listener(_cx, _f),
                    namespace: None,
                    mounted_element: Default::default(),
                    volatile: false,
                }
            }
        )*
    };
}

mod animation;
mod clipboard;
mod composition;
mod drag;
mod focus;
mod form;
mod image;
mod keyboard;
mod media;
mod mouse;
mod pointer;
mod scroll;
mod selection;
mod toggle;
mod touch;
mod transition;
mod wheel;

pub use animation::*;
pub use clipboard::*;
pub use composition::*;
pub use drag::*;
pub use focus::*;
pub use form::*;
pub use image::*;
pub use keyboard::*;
pub use media::*;
pub use mouse::*;
pub use pointer::*;
pub use scroll::*;
pub use selection::*;
pub use toggle::*;
pub use touch::*;
pub use transition::*;
pub use wheel::*;

pub fn event_bubbles(evt: &str) -> bool {
    match evt {
        "copy" => true,
        "cut" => true,
        "paste" => true,
        "compositionend" => true,
        "compositionstart" => true,
        "compositionupdate" => true,
        "keydown" => true,
        "keypress" => true,
        "keyup" => true,
        "focus" => false,
        "focusout" => true,
        "focusin" => true,
        "blur" => false,
        "change" => true,
        "input" => true,
        "invalid" => true,
        "reset" => true,
        "submit" => true,
        "click" => true,
        "contextmenu" => true,
        "doubleclick" => true,
        "dblclick" => true,
        "drag" => true,
        "dragend" => true,
        "dragenter" => false,
        "dragexit" => false,
        "dragleave" => true,
        "dragover" => true,
        "dragstart" => true,
        "drop" => true,
        "mousedown" => true,
        "mouseenter" => false,
        "mouseleave" => false,
        "mousemove" => true,
        "mouseout" => true,
        "scroll" => false,
        "mouseover" => true,
        "mouseup" => true,
        "pointerdown" => true,
        "pointermove" => true,
        "pointerup" => true,
        "pointercancel" => true,
        "gotpointercapture" => true,
        "lostpointercapture" => true,
        "pointerenter" => false,
        "pointerleave" => false,
        "pointerover" => true,
        "pointerout" => true,
        "select" => true,
        "touchcancel" => true,
        "touchend" => true,
        "touchmove" => true,
        "touchstart" => true,
        "wheel" => true,
        "abort" => false,
        "canplay" => false,
        "canplaythrough" => false,
        "durationchange" => false,
        "emptied" => false,
        "encrypted" => true,
        "ended" => false,
        "error" => false,
        "loadeddata" => false,
        "loadedmetadata" => false,
        "loadstart" => false,
        "pause" => false,
        "play" => false,
        "playing" => false,
        "progress" => false,
        "ratechange" => false,
        "seeked" => false,
        "seeking" => false,
        "stalled" => false,
        "suspend" => false,
        "timeupdate" => false,
        "volumechange" => false,
        "waiting" => false,
        "animationstart" => true,
        "animationend" => true,
        "animationiteration" => true,
        "transitionend" => true,
        "toggle" => true,
        _ => true,
    }
}
