//! Virtual Events
//! This module provides a wrapping of platform-specific events with a list of events easier to work with.
//!
//! 3rd party renderers are responsible for forming this virtual events from events
//!
//! The goal here is to provide a consistent event interface across all renderer types
use generational_arena::Index;

pub struct EventTrigger {
    pub component_id: Index,
    pub listener_id: u32,
    pub event: VirtualEvent,
}

impl EventTrigger {
    pub fn new() -> Self {
        todo!()
    }

    /// Create a new "start" event that boots up the virtual dom if it is paused
    pub fn start_event() -> Self {
        todo!()
    }
}

pub enum VirtualEvent {
    // the event to drain the current lifecycle queue
    // Used to initate the dom
    StartEvent,

    // Real events
    ClipboardEvent,
    CompositionEvent,
    KeyboardEvent,
    FocusEvent,
    FormEvent,
    GenericEvent,
    MouseEvent,
    PointerEvent,
    SelectionEvent,
    TouchEvent,
    UIEvent,
    WheelEvent,
    MediaEvent,
    ImageEvent,
    AnimationEvent,
    TransitionEvent,
    OtherEvent,
}
