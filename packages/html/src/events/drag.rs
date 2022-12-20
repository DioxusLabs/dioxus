use dioxus_core::Event;

use crate::MouseData;

pub type DragEvent = Event<DragData>;

/// The DragEvent interface is a DOM event that represents a drag and drop interaction. The user initiates a drag by
/// placing a pointer device (such as a mouse) on the touch surface and then dragging the pointer to a new location
/// (such as another DOM element). Applications are free to interpret a drag and drop interaction in an
/// application-specific way.
pub struct DragData {
    /// Inherit mouse data
    pub mouse: MouseData,

    /// And then add the rest of the drag data
    pub data: Box<dyn std::any::Any>,
}

impl_event! {
    DragData;

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
}
