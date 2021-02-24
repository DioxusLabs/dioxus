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
}

pub enum VirtualEvent {
    // Real events
    ClipboardEvent(ClipboardEvent),
    CompositionEvent(CompositionEvent),
    KeyboardEvent(KeyboardEvent),
    FocusEvent(FocusEvent),
    FormEvent(FormEvent),
    GenericEvent(GenericEvent),
    MouseEvent(MouseEvent),
    PointerEvent(PointerEvent),
    SelectionEvent(SelectionEvent),
    TouchEvent(TouchEvent),
    UIEvent(UIEvent),
    WheelEvent(WheelEvent),
    MediaEvent(MediaEvent),
    ImageEvent(ImageEvent),
    AnimationEvent(AnimationEvent),
    TransitionEvent(TransitionEvent),

    OtherEvent,
}

// these should reference the underlying event

pub struct ClipboardEvent {}
pub struct CompositionEvent {}
pub struct KeyboardEvent {}
pub struct FocusEvent {}
pub struct FormEvent {}
pub struct GenericEvent {}
pub struct MouseEvent {}
pub struct PointerEvent {}
pub struct SelectionEvent {}
pub struct TouchEvent {}
pub struct UIEvent {}
pub struct WheelEvent {}
pub struct MediaEvent {}
pub struct ImageEvent {}
pub struct AnimationEvent {}
pub struct TransitionEvent {}
