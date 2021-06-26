//! This module provides a set of common events for all Dioxus apps to target, regardless of host platform.
//! -------------------------------------------------------------------------------------------------------
//!
//! 3rd party renderers are responsible for converting their native events into these virtual event types. Events might
//! be heavy or need to interact through FFI, so the events themselves are designed to be lazy.

use std::rc::Rc;

use crate::{innerlude::ScopeIdx, virtual_dom::RealDomNode};

#[derive(Debug)]
pub struct EventTrigger {
    pub component_id: ScopeIdx,
    pub real_node_id: RealDomNode,
    pub event: VirtualEvent,
}

impl EventTrigger {
    pub fn new(event: VirtualEvent, scope: ScopeIdx, mounted_dom_id: RealDomNode) -> Self {
        Self {
            component_id: scope,
            real_node_id: mounted_dom_id,
            event,
        }
    }
}

#[derive(Debug)]
pub enum VirtualEvent {
    // Real events
    ClipboardEvent(Rc<dyn on::ClipboardEvent>),
    CompositionEvent(Rc<dyn on::CompositionEvent>),
    KeyboardEvent(Rc<dyn on::KeyboardEvent>),
    FocusEvent(Rc<dyn on::FocusEvent>),
    FormEvent(Rc<dyn on::FormEvent>),
    SelectionEvent(Rc<dyn on::SelectionEvent>),
    TouchEvent(Rc<dyn on::TouchEvent>),
    UIEvent(Rc<dyn on::UIEvent>),
    WheelEvent(Rc<dyn on::WheelEvent>),
    MediaEvent(Rc<dyn on::MediaEvent>),
    AnimationEvent(Rc<dyn on::AnimationEvent>),
    TransitionEvent(Rc<dyn on::TransitionEvent>),
    ToggleEvent(Rc<dyn on::ToggleEvent>),
    MouseEvent(Rc<dyn on::MouseEvent>),
    PointerEvent(Rc<dyn on::PointerEvent>),

    // Whenever a task is ready (complete) Dioxus produces this "FiberEvent"
    FiberEvent { task_id: u16 },

    // image event has conflicting method types
    // ImageEvent(event_data::ImageEvent),
    OtherEvent,
}

pub mod on {
    //! This module defines the synthetic events that all Dioxus apps enable. No matter the platform, every dioxus renderer
    //! will implement the same events and same behavior (bubbling, cancelation, etc).
    //!
    //! Synthetic events are immutable and wrapped in Arc. It is the intention for Dioxus renderers to re-use the underyling
    //! Arc allocation through "get_mut"
    //!
    //!
    //!

    #![allow(unused)]
    use std::{fmt::Debug, ops::Deref, rc::Rc};

    use crate::{
        builder::ElementBuilder,
        builder::NodeCtx,
        innerlude::{Attribute, Listener, RealDomNode, VNode},
    };
    use std::cell::Cell;

    use super::VirtualEvent;

    macro_rules! event_directory {
        ( $( $eventdata:ident: [ $( $name:ident )* ]; )* ) => {
            $(
                $(
                    pub fn $name<'a>(
                        c: &'_ NodeCtx<'a>,
                        callback: impl Fn(Rc<dyn $eventdata>) + 'a,
                    ) -> Listener<'a> {
                        let bump = &c.bump();
                        Listener {
                            event: stringify!($name),
                            mounted_node: bump.alloc(Cell::new(RealDomNode::empty())),
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

    trait GenericEvent {
        /// Returns whether or not a specific event is a bubbling event
        fn bubbles(&self) -> bool;
        /// Sets or returns whether the event should propagate up the hierarchy or not
        fn cancel_bubble(&self);
        /// Returns whether or not an event can have its default action prevented
        fn cancelable(&self) -> bool;
        /// Returns whether the event is composed or not
        fn composed(&self) -> bool;
        /// Returns the event's path
        fn composed_path(&self) -> String;
        /// Returns the element whose event listeners triggered the event
        fn current_target(&self);
        /// Returns whether or not the preventDefault method was called for the event
        fn default_prevented(&self) -> bool;
        /// Returns which phase of the event flow is currently being evaluated
        fn event_phase(&self) -> usize;
        /// Returns whether or not an event is trusted
        fn is_trusted(&self) -> bool;
        /// Cancels the event if it is cancelable, meaning that the default action that belongs to the event will
        fn prevent_default(&self);
        /// Prevents other listeners of the same event from being called
        fn stop_immediate_propagation(&self);
        /// Prevents further propagation of an event during event flow
        fn stop_propagation(&self);
        /// Returns the element that triggered the event
        fn target(&self);
        /// Returns the time (in milliseconds relative to the epoch) at which the event was created
        fn time_stamp(&self) -> usize;
    }

    pub trait ClipboardEvent: Debug {
        // DOMDataTransfer clipboardData
    }

    pub trait CompositionEvent: Debug {
        fn data(&self) -> String;
    }

    pub trait KeyboardEvent: Debug {
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
        fn get_modifier_state(&self, key_code: usize) -> bool;
    }

    pub trait FocusEvent: Debug {
        /* DOMEventTarget relatedTarget */
    }

    pub trait FormEvent: Debug {
        fn value(&self) -> String;
    }

    pub trait MouseEvent: Debug {
        fn alt_key(&self) -> bool;
        fn button(&self) -> i16;
        fn buttons(&self) -> u16;
        fn client_x(&self) -> i32;
        fn client_y(&self) -> i32;
        fn ctrl_key(&self) -> bool;
        fn meta_key(&self) -> bool;
        fn page_x(&self) -> i32;
        fn page_y(&self) -> i32;
        fn screen_x(&self) -> i32;
        fn screen_y(&self) -> i32;
        fn shift_key(&self) -> bool;
        fn get_modifier_state(&self, key_code: &str) -> bool;
    }

    pub trait PointerEvent: Debug {
        // Mouse only
        fn alt_key(&self) -> bool;
        fn button(&self) -> usize;
        fn buttons(&self) -> usize;
        fn client_x(&self) -> i32;
        fn client_y(&self) -> i32;
        fn ctrl_key(&self) -> bool;
        fn meta_key(&self) -> bool;
        fn page_x(&self) -> i32;
        fn page_y(&self) -> i32;
        fn screen_x(&self) -> i32;
        fn screen_y(&self) -> i32;
        fn shift_key(&self) -> bool;
        fn get_modifier_state(&self, key_code: usize) -> bool;
        fn pointer_id(&self) -> usize;
        fn width(&self) -> usize;
        fn height(&self) -> usize;
        fn pressure(&self) -> usize;
        fn tangential_pressure(&self) -> usize;
        fn tilt_x(&self) -> i32;
        fn tilt_y(&self) -> i32;
        fn twist(&self) -> i32;
        fn pointer_type(&self) -> String;
        fn is_primary(&self) -> bool;
    }

    pub trait SelectionEvent: Debug {}

    pub trait TouchEvent: Debug {
        fn alt_key(&self) -> bool;
        fn ctrl_key(&self) -> bool;
        fn meta_key(&self) -> bool;
        fn shift_key(&self) -> bool;
        fn get_modifier_state(&self, key_code: usize) -> bool;
        // changedTouches: DOMTouchList,
        // targetTouches: DOMTouchList,
        // touches: DOMTouchList,
    }

    pub trait UIEvent: Debug {
        // DOMAbstractView view
        fn detail(&self) -> i32;
    }

    pub trait WheelEvent: Debug {
        fn delta_mode(&self) -> i32;
        fn delta_x(&self) -> i32;
        fn delta_y(&self) -> i32;
        fn delta_z(&self) -> i32;
    }

    pub trait MediaEvent: Debug {}

    pub trait ImageEvent: Debug {
        //     load error
    }

    pub trait AnimationEvent: Debug {
        fn animation_name(&self) -> String;
        fn pseudo_element(&self) -> String;
        fn elapsed_time(&self) -> f32;
    }

    pub trait TransitionEvent: Debug {
        fn property_name(&self) -> String;
        fn pseudo_element(&self) -> String;
        fn elapsed_time(&self) -> f32;
    }

    pub trait ToggleEvent: Debug {}
}

mod tests {

    use std::rc::Rc;

    use crate as dioxus;
    use crate::events::on::MouseEvent;
    use crate::prelude::*;

    fn autocomplete() {
        // let v = move |evt| {
        //     let r = evt.alt_key();
        // };

        let g = rsx! {
            button {
                onclick: move |evt| {
                    let r = evt.alt_key();
                }
            }
        };
    }
}
