//! Virtual Events
//! This module provides a wrapping of platform-specific events with a list of events easier to work with.
//!
//! 3rd party renderers are responsible for forming this virtual events from events
//!
//! The goal here is to provide a consistent event interface across all renderer types

pub enum VirtualEvent {
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
