//! Virtual Events
//! This module provides a wrapping of platform-specific events with a list of events easier to work with.
//! 3rd party renderers are responsible for forming this virtual events from events.
//! The goal here is to provide a consistent event interface across all renderer types.
//!
//! also... websys integerates poorly with rust analyzer, so we handle that for you automatically.

use crate::innerlude::ScopeIdx;

#[derive(Debug)]
pub struct EventTrigger {
    pub component_id: ScopeIdx,
    pub listener_id: usize,
    pub event: VirtualEvent,
}

impl EventTrigger {
    pub fn new(event: VirtualEvent, scope: ScopeIdx, id: usize) -> Self {
        Self {
            component_id: scope,
            listener_id: id,
            event,
        }
    }
}

#[derive(Debug)]
pub enum VirtualEvent {
    // Real events
    ClipboardEvent(on::ClipboardEvent),
    CompositionEvent(on::CompositionEvent),
    KeyboardEvent(on::KeyboardEvent),
    FocusEvent(on::FocusEvent),
    FormEvent(on::FormEvent),
    GenericEvent(on::GenericEvent),
    SelectionEvent(on::SelectionEvent),
    TouchEvent(on::TouchEvent),
    UIEvent(on::UIEvent),
    WheelEvent(on::WheelEvent),
    MediaEvent(on::MediaEvent),
    AnimationEvent(on::AnimationEvent),
    TransitionEvent(on::TransitionEvent),
    ToggleEvent(on::ToggleEvent),

    // TODO these events are particularly heavy, so we box them
    MouseEvent(on::MouseEvent),
    PointerEvent(on::PointerEvent),

    // todo

    // ImageEvent(event_data::ImageEvent),
    OtherEvent,
}

pub mod on {
    #![allow(unused)]
    use std::ops::Deref;

    use crate::{
        builder::ElementBuilder,
        innerlude::{Attribute, Listener, VNode},
        virtual_dom::NodeCtx,
    };

    use super::VirtualEvent;

    macro_rules! event_builder {
            (
                $eventdata:ident;
            $(
                $(#[$attr:meta])*
                $name:ident
            )* ) => {
                $(
                    $(#[$attr])*
                    pub fn $name<'a>(
                        c: &'_ NodeCtx<'a>,
                        callback: impl Fn($eventdata) + 'a,
                    ) -> Listener<'a> {
                        let bump = &c.bump();
                        Listener {
                            event: stringify!($name),
                            id: *c.listener_id.borrow(),
                            scope: c.scope_ref.myidx,
                            callback: bump.alloc(move |evt: VirtualEvent| match evt {
                                VirtualEvent::$eventdata(event) => callback(event),
                                _ => {
                                    unreachable!("Downcasted VirtualEvent to wrong event type - this is a bug!")
                                }
                            }),
                        }
                    }
                )*
            };
        }

    pub struct GetModifierKey(pub Box<dyn Fn(usize) -> bool>);
    impl std::fmt::Debug for GetModifierKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // just skip for now
            Ok(())
        }
    }

    // DOMDataTransfer clipboardData
    #[derive(Debug)]
    pub struct ClipboardEvent {}
    event_builder! {
        ClipboardEvent;
        copy cut paste
    }

    // string data
    #[derive(Debug)]
    pub struct CompositionEvent {
        data: String,
    }
    event_builder! {
        CompositionEvent;
        compositionend compositionstart compositionupdate
    }

    #[derive(Debug)]
    pub struct KeyboardEvent {
        char_code: usize,
        ctrl_key: bool,
        key: String,
        key_code: usize,
        locale: String,
        location: usize,
        meta_key: bool,
        repeat: bool,
        shift_key: bool,
        which: usize,
        get_modifier_state: GetModifierKey,
    }
    pub struct KeyboardEvent2(pub Box<dyn KeyboardEventT>);
    impl std::ops::Deref for KeyboardEvent2 {
        type Target = Box<dyn KeyboardEventT>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    pub trait KeyboardEventT {
        fn char_code(&self) -> usize;
        fn ctrl_key(&self) -> bool;
        fn key(&self) -> String;
        fn key_code(&self) -> usize;
        fn locale(&self) -> String;
        fn location(&self) -> usize;
        fn meta_key(&self) -> bool;
        fn repeat(&self) -> bool;
        fn shift_key(&self) -> bool;
        fn which(&self) -> usize;
        fn get_modifier_state(&self) -> GetModifierKey;
    }

    event_builder! {
        KeyboardEvent;
        keydown keypress keyup
    }

    #[derive(Debug)]
    pub struct FocusEvent {/* DOMEventTarget relatedTarget */}
    event_builder! {
        FocusEvent;
        focus blur
    }

    #[derive(Debug)]
    pub struct FormEvent {
        pub value: String,
    }
    event_builder! {
        FormEvent;
        change input invalid reset submit
    }

    #[derive(Debug)]
    pub struct GenericEvent {/* Error Load */}
    event_builder! {
        GenericEvent;
    }

    #[derive(Debug)]
    pub struct MouseEvent(pub Box<RawMouseEvent>);

    #[derive(Debug)]
    pub struct RawMouseEvent {
        pub alt_key: bool,
        pub button: i32,
        pub buttons: i32,
        pub client_x: i32,
        pub client_y: i32,
        pub ctrl_key: bool,
        pub meta_key: bool,
        pub page_x: i32,
        pub page_y: i32,
        pub screen_x: i32,
        pub screen_y: i32,
        pub shift_key: bool,
        pub get_modifier_state: GetModifierKey,
        // relatedTarget: DOMEventTarget,
    }
    impl Deref for MouseEvent {
        type Target = RawMouseEvent;
        fn deref(&self) -> &Self::Target {
            self.0.as_ref()
        }
    }
    event_builder! {
        MouseEvent;
        click contextmenu doubleclick drag dragend dragenter dragexit
        dragleave dragover dragstart drop mousedown mouseenter mouseleave
        mousemove mouseout mouseover mouseup
    }

    #[derive(Debug)]
    pub struct PointerEvent(Box<RawPointerEvent>);
    impl Deref for PointerEvent {
        type Target = RawPointerEvent;
        fn deref(&self) -> &Self::Target {
            self.0.as_ref()
        }
    }

    #[derive(Debug)]
    pub struct RawPointerEvent {
        // Mouse only
        alt_key: bool,
        button: usize,
        buttons: usize,
        client_x: i32,
        client_y: i32,
        ctrl_key: bool,
        meta_key: bool,
        page_x: i32,
        page_y: i32,
        screen_x: i32,
        screen_y: i32,
        shift_key: bool,
        get_modifier_state: GetModifierKey,

        // Pointer-specific
        pointer_id: usize,
        width: usize,
        height: usize,
        pressure: usize,
        tangential_pressure: usize,
        tilt_x: i32,
        tilt_y: i32,
        twist: i32,
        pointer_type: String,
        is_primary: bool,
    }
    event_builder! {
        PointerEvent;
        pointerdown pointermove pointerup pointercancel gotpointercapture
        lostpointercapture pointerenter pointerleave pointerover pointerout
    }

    #[derive(Debug)]
    pub struct SelectionEvent {}
    event_builder! {
        SelectionEvent;
        select
    }

    #[derive(Debug)]
    pub struct TouchEvent {
        alt_key: bool,
        ctrl_key: bool,
        meta_key: bool,
        shift_key: bool,
        get_modifier_state: GetModifierKey,
        //
        // changedTouches: DOMTouchList,
        // todo
        // targetTouches: DOMTouchList,
        // touches: DOMTouchList,
        //  getModifierState(key): boolean
    }
    event_builder! {
        TouchEvent;
        touchcancel touchend touchmove touchstart
    }

    #[derive(Debug)]
    pub struct UIEvent {
        // DOMAbstractView view
        detail: i32,
    }
    event_builder! {
        UIEvent;
        scroll
    }

    #[derive(Debug)]
    pub struct WheelEvent {
        delta_mode: i32,
        delta_x: i32,
        delta_y: i32,
        delta_z: i32,
    }
    event_builder! {
        WheelEvent;
        wheel
    }

    #[derive(Debug)]
    pub struct MediaEvent {}
    event_builder! {
        MediaEvent;
        abort canplay canplaythrough durationchange emptied encrypted
        ended error loadeddata loadedmetadata loadstart pause play
        playing progress ratechange seeked seeking stalled suspend
        timeupdate volumechange waiting
    }

    // todo!
    // imageevent clashes with media event
    // might need to derive this e manually
    //
    // #[derive(Debug)]
    // pub struct ImageEvent {}
    // event_builder! {
    //     ImageEvent;
    //     load error
    // }

    #[derive(Debug)]
    pub struct AnimationEvent {
        animation_name: String,
        pseudo_element: String,
        elapsed_time: f32,
    }
    event_builder! {
        AnimationEvent;
        animationstart animationend animationiteration
    }

    #[derive(Debug)]
    pub struct TransitionEvent {
        property_name: String,
        pseudo_element: String,
        elapsed_time: f32,
    }
    event_builder! {
        TransitionEvent;
        transitionend
    }

    #[derive(Debug)]
    pub struct ToggleEvent {}
    event_builder! {
        ToggleEvent;
        toggle
    }
}
