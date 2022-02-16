use std::fmt::Arguments;

use crate::on::*;

use super::ElementBuilder;
use bumpalo::collections::Vec as BumpVec;
use dioxus_core::{
    self, exports::bumpalo, Attribute, Element, IntoVNode, Listener, NodeFactory, Scope,
    ScopeState, VNode,
};

use bumpalo::boxed::Box as BumpBox;

macro_rules! event_directory {
        ( $(
            $( #[$attr:meta] )*
            $wrapper:ident($data:ident): [
                $(
                    $( #[$method_attr:meta] )*
                    $name:ident
                )*
            ];
        )* ) => {
            $(
                $(
                    $(#[$method_attr])*
                    pub fn $name(
                        mut self,
                        mut callback: impl FnMut($wrapper) + 'a,
                    ) -> Self
                    {
                        let bump = &self.fac.bump();


                        use dioxus_core::{AnyEvent};
                        // we can't allocate unsized in bumpalo's box, so we need to craft the box manually
                        // safety: this is essentially the same as calling Box::new() but manually
                        // The box is attached to the lifetime of the bumpalo allocator
                        let cb: &mut dyn FnMut(AnyEvent) = bump.alloc(move |evt: AnyEvent| {
                            let event = evt.downcast::<$data>().unwrap();
                            callback(event)
                        });

                        let callback: BumpBox<dyn FnMut(AnyEvent) + 'a> = unsafe { BumpBox::from_raw(cb) };

                        // ie oncopy
                        let event_name = stringify!($name);

                        // ie copy
                        let shortname: &'static str = &event_name[2..];

                        let handler = bump.alloc(std::cell::RefCell::new(Some(callback)));
                        let listener = self.fac.listener(shortname, handler);
                        self.listeners.push(listener);
                        self
                    }
                )*
            )*
        };
    }

impl<'a, T> ElementBuilder<'a, T> {
    // The Dioxus Synthetic event system
    // todo: move these into the html event system. dioxus accepts *any* event, so having these here doesn't make sense.
    event_directory! {
        ClipboardEvent(ClipboardData): [
            /// Called when "copy"
            oncopy

            /// oncut
            oncut

            /// onpaste
            onpaste
        ];

        CompositionEvent(CompositionData): [
            /// oncompositionend
            oncompositionend

            /// oncompositionstart
            oncompositionstart

            /// oncompositionupdate
            oncompositionupdate
        ];

        KeyboardEvent(KeyboardData): [
            /// onkeydown
            onkeydown

            /// onkeypress
            onkeypress

            /// onkeyup
            onkeyup
        ];

        FocusEvent(FocusData): [
            /// onfocus
            onfocus

            // onfocusout
            onfocusout

            // onfocusin
            onfocusin

            /// onblur
            onblur
        ];

        FormEvent(FormData): [
            /// onchange
            onchange

            /// oninput handler
            oninput

            /// oninvalid
            oninvalid

            /// onreset
            onreset

            /// onsubmit
            onsubmit
        ];

        /// A synthetic event that wraps a web-style [`MouseEvent`](https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent)
        ///
        ///
        /// The MouseEvent interface represents events that occur due to the user interacting with a pointing device (such as a mouse).
        ///
        /// ## Trait implementation:
        /// ```rust, ignore
        ///     fn alt_key(&self) -> bool;
        ///     fn button(&self) -> i16;
        ///     fn buttons(&self) -> u16;
        ///     fn client_x(&self) -> i32;
        ///     fn client_y(&self) -> i32;
        ///     fn ctrl_key(&self) -> bool;
        ///     fn meta_key(&self) -> bool;
        ///     fn page_x(&self) -> i32;
        ///     fn page_y(&self) -> i32;
        ///     fn screen_x(&self) -> i32;
        ///     fn screen_y(&self) -> i32;
        ///     fn shift_key(&self) -> bool;
        ///     fn get_modifier_state(&self, key_code: &str) -> bool;
        /// ```
        ///
        /// ## Event Handlers
        /// - [`onclick`]
        /// - [`oncontextmenu`]
        /// - [`ondoubleclick`]
        /// - [`ondrag`]
        /// - [`ondragend`]
        /// - [`ondragenter`]
        /// - [`ondragexit`]
        /// - [`ondragleave`]
        /// - [`ondragover`]
        /// - [`ondragstart`]
        /// - [`ondrop`]
        /// - [`onmousedown`]
        /// - [`onmouseenter`]
        /// - [`onmouseleave`]
        /// - [`onmousemove`]
        /// - [`onmouseout`]
        /// - [`onmouseover`]
        /// - [`onmouseup`]
        MouseEvent(MouseData): [
            /// Execute a callback when a button is clicked.
            ///
            /// ## Description
            ///
            /// An element receives a click event when a pointing device button (such as a mouse's primary mouse button)
            /// is both pressed and released while the pointer is located inside the element.
            ///
            /// - Bubbles: Yes
            /// - Cancelable: Yes
            /// - Interface(InteData): [`MouseEvent`]
            ///
            /// If the button is pressed on one element and the pointer is moved outside the element before the button
            /// is released, the event is fired on the most specific ancestor element that contained both elements.
            /// `click` fires after both the `mousedown` and `mouseup` events have fired, in that order.
            ///
            /// ## Example
            /// ```
            /// rsx!( button { "click me", onclick: move |_| log::info!("Clicked!`") } )
            /// ```
            ///
            /// ## Reference
            /// - <https://www.w3schools.com/tags/ev_onclick.asp>
            /// - <https://developer.mozilla.org/en-US/docs/Web/API/Element/click_event>
            onclick

            /// oncontextmenu
            oncontextmenu

            /// ondoubleclick
            ondoubleclick

            /// ondrag
            ondrag

            /// ondragend
            ondragend

            /// ondragenter
            ondragenter

            /// ondragexit
            ondragexit

            /// ondragleave
            ondragleave

            /// ondragover
            ondragover

            /// ondragstart
            ondragstart

            /// ondrop
            ondrop

            /// onmousedown
            onmousedown

            /// onmouseenter
            onmouseenter

            /// onmouseleave
            onmouseleave

            /// onmousemove
            onmousemove

            /// onmouseout
            onmouseout

            ///
            onscroll

            /// onmouseover
            ///
            /// Triggered when the users's mouse hovers over an element.
            onmouseover

            /// onmouseup
            onmouseup
        ];

        PointerEvent(PointerData): [
            /// pointerdown
            onpointerdown

            /// pointermove
            onpointermove

            /// pointerup
            onpointerup

            /// pointercancel
            onpointercancel

            /// gotpointercapture
            ongotpointercapture

            /// lostpointercapture
            onlostpointercapture

            /// pointerenter
            onpointerenter

            /// pointerleave
            onpointerleave

            /// pointerover
            onpointerover

            /// pointerout
            onpointerout
        ];

        SelectionEvent(SelectionData): [
            /// onselect
            onselect
        ];

        TouchEvent(TouchData): [
            /// ontouchcancel
            ontouchcancel

            /// ontouchend
            ontouchend

            /// ontouchmove
            ontouchmove

            /// ontouchstart
            ontouchstart
        ];

        WheelEvent(WheelData): [
            ///
            onwheel
        ];

        MediaEvent(MediaData): [
            ///abort
            onabort

            ///canplay
            oncanplay

            ///canplaythrough
            oncanplaythrough

            ///durationchange
            ondurationchange

            ///emptied
            onemptied

            ///encrypted
            onencrypted

            ///ended
            onended

            ///error
            onerror

            ///loadeddata
            onloadeddata

            ///loadedmetadata
            onloadedmetadata

            ///loadstart
            onloadstart

            ///pause
            onpause

            ///play
            onplay

            ///playing
            onplaying

            ///progress
            onprogress

            ///ratechange
            onratechange

            ///seeked
            onseeked

            ///seeking
            onseeking

            ///stalled
            onstalled

            ///suspend
            onsuspend

            ///timeupdate
            ontimeupdate

            ///volumechange
            onvolumechange

            ///waiting
            onwaiting
        ];

        AnimationEvent(AnimationData): [
            /// onanimationstart
            onanimationstart

            /// onanimationend
            onanimationend

            /// onanimationiteration
            onanimationiteration
        ];

        TransitionEvent(TransitionData): [
            ///
            ontransitionend
        ];

        ToggleEvent(ToggleData): [
            ///
            ontoggle
        ];
    }
}
