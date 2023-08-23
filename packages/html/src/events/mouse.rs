use crate::point_interaction::{PointData, PointInteraction};
use dioxus_core::Event;

pub type MouseEvent = Event<MouseData>;

/// A synthetic event that wraps a web-style [`MouseEvent`](https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent)
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Data associated with a mouse event
pub struct MouseData {
    /// Common data for all pointer/mouse events
    #[cfg_attr(feature = "serialize", serde(flatten))]
    point_data: PointData,
}

impl_event! {
    MouseData;

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
    /// ```rust, ignore
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

    /// ondoubleclick
    ondblclick

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

    /// onmouseover
    ///
    /// Triggered when the users's mouse hovers over an element.
    onmouseover

    /// onmouseup
    onmouseup
}

impl MouseData {
    /// Construct MouseData with the specified properties
    ///
    /// Note: the current implementation truncates coordinates. In the future, when we change the internal representation, it may also support a fractional part.
    pub fn new(point_data: PointData) -> Self {
        Self { point_data }
    }
}

impl PointInteraction for MouseData {
    fn get_point_data(&self) -> PointData {
        self.point_data
    }
}
