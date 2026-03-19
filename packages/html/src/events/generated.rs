use super::*;

macro_rules! event_groups {
    (
        $(
            $data:ident => $converter:ident {
                $(
                    $( #[$attr:meta] )*
                    $name:ident : $raw:ident,
                )*
                $( @raw $raw_only:ident, )*
            }
        )*
    ) => {
        /// A converter between a platform specific event and a general event. All code in a renderer that has a large binary size should be placed in this trait. Each of these functions should be snipped in high levels of optimization.
        pub trait HtmlEventConverter: Send + Sync {
            $(
                fn $converter(&self, event: &PlatformEventData) -> $data;
            )*
        }

        $(
            impl From<&PlatformEventData> for $data {
                fn from(val: &PlatformEventData) -> Self {
                    with_event_converter(|c| c.$converter(val))
                }
            }
        )*

        $(
            impl_event! {
                $data;
                $(
                    $( #[$attr] )*
                    $name: concat!("on", stringify!($raw));
                )*
            }
        )*
    };
}

event_groups! {
    AnimationData => convert_animation_data {
        /// onanimationstart
        onanimationstart: animationstart,
        /// onanimationend
        onanimationend: animationend,
        /// onanimationiteration
        onanimationiteration: animationiteration,
    }

    CancelData => convert_cancel_data {
        /// oncancel
        oncancel: cancel,
    }

    ClipboardData => convert_clipboard_data {
        /// oncopy
        oncopy: copy,
        /// oncut
        oncut: cut,
        /// onpaste
        onpaste: paste,
    }

    CompositionData => convert_composition_data {
        /// oncompositionstart
        oncompositionstart: compositionstart,
        /// oncompositionend
        oncompositionend: compositionend,
        /// oncompositionupdate
        oncompositionupdate: compositionupdate,
    }

    DragData => convert_drag_data {
        /// ondrag
        ondrag: drag,
        /// ondragend
        ondragend: dragend,
        /// ondragenter
        ondragenter: dragenter,
        /// ondragexit
        ondragexit: dragexit,
        /// ondragleave
        ondragleave: dragleave,
        /// ondragover
        ondragover: dragover,
        /// ondragstart
        ondragstart: dragstart,
        /// ondrop
        ondrop: drop,
    }

    FocusData => convert_focus_data {
        /// onfocus
        onfocus: focus,
        onfocusout: focusout,
        onfocusin: focusin,
        /// onblur
        onblur: blur,
    }

    FormData => convert_form_data {
        /// onchange
        onchange: change,
        /// The `oninput` event is fired when the value of a `<input>`, `<select>`, or `<textarea>` element is changed.
        ///
        /// There are two main approaches to updating your input element:
        /// 1) Controlled inputs directly update the value of the input element as the user interacts with the element
        ///
        /// ```rust
        /// use dioxus::prelude::*;
        ///
        /// fn App() -> Element {
        ///     let mut value = use_signal(|| "hello world".to_string());
        ///
        ///     rsx! {
        ///         input {
        ///             value: "{value}",
        ///             oninput: move |event| value.set(event.value())
        ///         }
        ///         button {
        ///             onclick: move |_| value.write().clear(),
        ///             "Clear"
        ///         }
        ///     }
        /// }
        /// ```
        ///
        /// 2) Uncontrolled inputs just read the value of the input element as it changes
        ///
        /// ```rust
        /// use dioxus::prelude::*;
        ///
        /// fn App() -> Element {
        ///     rsx! {
        ///         input {
        ///             oninput: move |event| println!("{}", event.value()),
        ///         }
        ///     }
        /// }
        /// ```
        oninput: input,
        /// oninvalid
        oninvalid: invalid,
        /// onreset
        onreset: reset,
        /// onsubmit
        onsubmit: submit,
    }

    ImageData => convert_image_data {
        /// onerror
        onerror: error,
        /// onload
        onload: load,
    }

    KeyboardData => convert_keyboard_data {
        /// onkeydown
        onkeydown: keydown,
        /// onkeypress
        onkeypress: keypress,
        /// onkeyup
        onkeyup: keyup,
    }

    MediaData => convert_media_data {
        ///abort
        onabort: abort,
        ///canplay
        oncanplay: canplay,
        ///canplaythrough
        oncanplaythrough: canplaythrough,
        ///durationchange
        ondurationchange: durationchange,
        ///emptied
        onemptied: emptied,
        ///encrypted
        onencrypted: encrypted,
        ///ended
        onended: ended,
        ///loadeddata
        onloadeddata: loadeddata,
        ///loadedmetadata
        onloadedmetadata: loadedmetadata,
        ///loadstart
        onloadstart: loadstart,
        ///pause
        onpause: pause,
        ///play
        onplay: play,
        ///playing
        onplaying: playing,
        ///progress
        onprogress: progress,
        ///ratechange
        onratechange: ratechange,
        ///seeked
        onseeked: seeked,
        ///seeking
        onseeking: seeking,
        ///stalled
        onstalled: stalled,
        ///suspend
        onsuspend: suspend,
        ///timeupdate
        ontimeupdate: timeupdate,
        ///volumechange
        onvolumechange: volumechange,
        ///waiting
        onwaiting: waiting,
        @raw interruptbegin,
        @raw interruptend,
        @raw loadend,
        @raw timeout,
    }

    MountedData => convert_mounted_data {
        #[doc(alias = "ref")]
        #[doc(alias = "createRef")]
        #[doc(alias = "useRef")]
        /// The onmounted event is fired when the element is first added to the DOM. This event gives you a [`MountedData`] object and lets you interact with the raw DOM element.
        onmounted: mounted,
    }

    MouseData => convert_mouse_data {
        /// Execute a callback when a button is clicked.
        onclick: click,
        /// oncontextmenu
        oncontextmenu: contextmenu,
        #[deprecated(since = "0.5.0", note = "use ondoubleclick instead")]
        ondblclick: dblclick,
        ondoubleclick: dblclick,
        /// onmousedown
        onmousedown: mousedown,
        /// onmouseenter
        onmouseenter: mouseenter,
        /// onmouseleave
        onmouseleave: mouseleave,
        /// onmousemove
        onmousemove: mousemove,
        /// onmouseout
        onmouseout: mouseout,
        /// onmouseover
        onmouseover: mouseover,
        /// onmouseup
        onmouseup: mouseup,
        @raw doubleclick,
    }

    PointerData => convert_pointer_data {
        /// pointerdown
        onpointerdown: pointerdown,
        /// pointermove
        onpointermove: pointermove,
        /// pointerup
        onpointerup: pointerup,
        /// pointercancel
        onpointercancel: pointercancel,
        /// gotpointercapture
        ongotpointercapture: gotpointercapture,
        /// lostpointercapture
        onlostpointercapture: lostpointercapture,
        /// pointerenter
        onpointerenter: pointerenter,
        /// pointerleave
        onpointerleave: pointerleave,
        /// pointerover
        onpointerover: pointerover,
        /// pointerout
        onpointerout: pointerout,
        /// auxclick
        onauxclick: auxclick,
        @raw pointerlockchange,
        @raw pointerlockerror,
    }

    ResizeData => convert_resize_data {
        /// onresize
        onresize: resize,
    }

    ScrollData => convert_scroll_data {
        /// onscroll
        onscroll: scroll,
        /// onscrollend
        onscrollend: scrollend,
    }

    SelectionData => convert_selection_data {
        /// select
        onselect: select,
        /// selectstart
        onselectstart: selectstart,
        /// selectionchange
        onselectionchange: selectionchange,
    }

    ToggleData => convert_toggle_data {
        /// ontoggle
        ontoggle: toggle,
        /// onbeforetoggle
        onbeforetoggle: beforetoggle,
    }

    TouchData => convert_touch_data {
        /// touchstart
        ontouchstart: touchstart,
        /// touchmove
        ontouchmove: touchmove,
        /// touchend
        ontouchend: touchend,
        /// touchcancel
        ontouchcancel: touchcancel,
    }

    TransitionData => convert_transition_data {
        /// transitionend
        ontransitionend: transitionend,
    }

    VisibleData => convert_visible_data {
        /// onvisible
        onvisible: visible,
    }

    WheelData => convert_wheel_data {
        /// Called when the mouse wheel is rotated over an element.
        onwheel: wheel,
    }
}
