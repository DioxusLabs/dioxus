//! This module provides a set of common events for all Dioxus apps to target, regardless of host platform.
//! -------------------------------------------------------------------------------------------------------
//!
//! 3rd party renderers are responsible for converting their native events into these virtual event types. Events might
//! be heavy or need to interact through FFI, so the events themselves are designed to be lazy.

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

    // ImageEvent(event_data::ImageEvent),
    OtherEvent,
}

pub mod on {
    #![allow(unused)]
    use std::{ops::Deref, rc::Rc};

    use crate::{
        builder::ElementBuilder,
        innerlude::{Attribute, Listener, VNode},
        virtual_dom::NodeCtx,
    };

    use super::VirtualEvent;

    macro_rules! event_directory {
        ( $( $eventdata:ident: [ $( $name:ident )* ]; )* ) => {
            $(
                $(
                    pub fn $name<'a>(
                        c: &'_ NodeCtx<'a>,
                        callback: impl Fn($eventdata) + 'a,
                    ) -> Listener<'a> {
                        let bump = &c.bump();
                        Listener {
                            event: stringify!($name),
                            id: *c.listener_id.borrow(),
                            scope: c.scope_ref.arena_idx,
                            callback: bump.alloc(move |evt: VirtualEvent| match evt {
                                VirtualEvent::$eventdata(event) => callback(event),
                                _ => unreachable!("Downcasted VirtualEvent to wrong event type - this is an internal bug!")
                            }),
                        }
                    }
                )*
            )*
        };
    }

    event_directory! {
        ClipboardEvent: [copy cut paste];
        CompositionEvent: [compositionend compositionstart compositionupdate];
        KeyboardEvent: [keydown keypress keyup];
        FocusEvent: [focus blur];
        FormEvent: [change input invalid reset submit];
        GenericEvent: [];
        MouseEvent: [
            click contextmenu doubleclick drag dragend dragenter dragexit
            dragleave dragover dragstart drop mousedown mouseenter mouseleave
            mousemove mouseout mouseover mouseup
        ];
        PointerEvent: [
            pointerdown pointermove pointerup pointercancel gotpointercapture
            lostpointercapture pointerenter pointerleave pointerover pointerout
        ];
        SelectionEvent: [select];
        TouchEvent: [touchcancel touchend touchmove touchstart];
        UIEvent: [scroll];
        WheelEvent: [wheel];
        MediaEvent: [
            abort canplay canplaythrough durationchange emptied encrypted
            ended error loadeddata loadedmetadata loadstart pause play
            playing progress ratechange seeked seeking stalled suspend
            timeupdate volumechange waiting
        ];
        AnimationEvent: [animationstart animationend animationiteration];
        TransitionEvent: [transitionend];
        ToggleEvent: [toggle];
    }

    pub struct GetModifierKey(pub Box<dyn Fn(usize) -> bool>);
    impl std::fmt::Debug for GetModifierKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Ok(()) // just skip for now
        }
    }

    // DOMDataTransfer clipboardData
    #[derive(Debug)]
    pub struct ClipboardEvent {}

    #[derive(Debug)]
    pub struct CompositionEvent {
        data: String,
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
    pub struct KeyboardEvent2(pub Rc<dyn KeyboardEventT>);
    impl std::ops::Deref for KeyboardEvent2 {
        type Target = dyn KeyboardEventT;
        fn deref(&self) -> &Self::Target {
            self.0.as_ref()
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

    #[derive(Debug)]
    pub struct FocusEvent {/* DOMEventTarget relatedTarget */}

    #[derive(Debug)]
    pub struct FormEvent {
        pub value: String,
    }

    #[derive(Debug)]
    pub struct GenericEvent {/* Error Load */}

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

    #[derive(Debug)]
    pub struct SelectionEvent {}

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

    #[derive(Debug)]
    pub struct UIEvent {
        // DOMAbstractView view
        detail: i32,
    }

    #[derive(Debug)]
    pub struct WheelEvent {
        delta_mode: i32,
        delta_x: i32,
        delta_y: i32,
        delta_z: i32,
    }

    #[derive(Debug)]
    pub struct MediaEvent {}

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

    #[derive(Debug)]
    pub struct TransitionEvent {
        property_name: String,
        pseudo_element: String,
        elapsed_time: f32,
    }

    #[derive(Debug)]
    pub struct ToggleEvent {}
}
