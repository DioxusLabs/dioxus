use bumpalo::boxed::Box as BumpBox;
use dioxus_core::exports::bumpalo;
use dioxus_core::*;

pub fn make_listener<'a, V: UiEvent>(
    factory: NodeFactory<'a>,
    mut callback: impl FnMut(&'a V) + 'a,

    // ie oncopy
    event_name: &'static str,
) -> Listener<'a> {
    let bump = &factory.bump();

    // we can't allocate unsized in bumpalo's box, so we need to craft the box manually
    // safety: this is essentially the same as calling Box::new() but manually
    // The box is attached to the lifetime of the bumpalo allocator
    let cb: &'a mut dyn FnMut(&'a dyn UiEvent) = bump.alloc(move |evt: &'a dyn UiEvent| {
        let evt = evt.downcast_ref::<V>().unwrap();
        callback(evt)
    });

    let callback: BumpBox<dyn FnMut(&'a dyn UiEvent) + 'a> = unsafe { BumpBox::from_raw(cb) };

    // ie copy
    let shortname: &'static str = &event_name[2..];

    let handler = bump.alloc(std::cell::RefCell::new(Some(callback)));
    factory.listener(shortname, handler)
}

#[macro_export]
macro_rules! event {
    ( $(
        $( #[$attr:meta] )*
        $data:ident:[
            $(
                $( #[$method_attr:meta] )*
                $name:ident
            )*
        ];
    )* ) => {
        $(
            $(
                $(#[$method_attr])*
                #[inline]
                pub fn $name<'a>( factory: NodeFactory<'a>, callback: impl FnMut(&'a $data) + 'a) -> Listener<'a> {
                    make_listener(factory, callback, stringify!($name))
                }
            )*
        )*
    };
}

pub mod animation;
pub mod clipboard;
pub mod composition;
pub mod drag;
pub mod focus;
pub mod form;
pub mod image;
pub mod keyboard;
pub mod media;
pub mod mouse;
pub mod pointer;
pub mod selection;
pub mod toggle;
pub mod touch;
pub mod transition;
pub mod wheel;

pub mod on {
    use super::*;

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
    pub use selection::*;
    pub use toggle::*;
    pub use touch::*;
    pub use transition::*;
    pub use wheel::*;
}

pub(crate) fn _event_meta(event: &UserEvent) -> (bool, EventPriority) {
    use EventPriority::*;

    match event.name {
        // clipboard
        "copy" | "cut" | "paste" => (true, Medium),

        // Composition
        "compositionend" | "compositionstart" | "compositionupdate" => (true, Low),

        // Keyboard
        "keydown" | "keypress" | "keyup" => (true, High),

        // Focus
        "focus" | "blur" | "focusout" | "focusin" => (true, Low),

        // Form
        "change" | "input" | "invalid" | "reset" | "submit" => (true, Medium),

        // Mouse
        "click" | "contextmenu" | "doubleclick" | "drag" | "dragend" | "dragenter" | "dragexit"
        | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown" | "mouseenter"
        | "mouseleave" | "mouseout" | "mouseover" | "mouseup" => (true, High),

        "mousemove" => (false, Medium),

        // Pointer
        "pointerdown" | "pointermove" | "pointerup" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" | "pointerenter" | "pointerleave" | "pointerover" | "pointerout" => {
            (true, Medium)
        }

        // Selection
        "select" | "touchcancel" | "touchend" => (true, Medium),

        // Touch
        "touchmove" | "touchstart" => (true, Medium),

        // Wheel
        "scroll" | "wheel" => (false, Medium),

        // Media
        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "error" | "loadeddata" | "loadedmetadata" | "loadstart" | "pause" | "play"
        | "playing" | "progress" | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend"
        | "timeupdate" | "volumechange" | "waiting" => (true, Medium),

        // Animation
        "animationstart" | "animationend" | "animationiteration" => (true, Medium),

        // Transition
        "transitionend" => (true, Medium),

        // Toggle
        "toggle" => (true, Medium),

        _ => (true, Low),
    }
}

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
        _ => panic!("unsupported event type {:?}", evt),
    }
}
