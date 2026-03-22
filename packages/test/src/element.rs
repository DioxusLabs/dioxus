use blitz_dom::Node;
use dioxus_core::{ElementId, Event};
use dioxus_html::{
    Modifiers, PlatformEventData,
    geometry::{Coordinates, euclid::Point2D},
};
use dioxus_native_dom::{DioxusDocument, synthetic_click_event};
use std::rc::Rc;

/// A reference to DOM node managed by a [crate::DocumentTester].
///
/// This provides facilities for interacting with the node, querying its layout properties, and
/// obtaining its content.
pub struct ResolvedElement<'doc> {
    pub(crate) document: &'doc DioxusDocument,
    pub(crate) node: &'doc Node,
}

impl<'doc> ResolvedElement<'doc> {
    /// Dispatches a `click` event on this element.
    ///
    /// The exact location of the click is unspecified.
    ///
    /// If the element has an `onclick` handler, it will be invoked once [crate::Tester::pump] is
    /// called.
    pub fn click(&self) {
        self.send_event(
            "click",
            Event::new(
                Rc::new(PlatformEventData::new(synthetic_click_event(
                    self.node,
                    Modifiers::empty(),
                ))),
                true,
            ),
        );
    }

    /// Sends an event with the given `name` to this element.
    ///
    /// The event is registered with the Dioxus runtime. A subsequent call to [crate::Tester::pump]
    /// causes the event handler to be invoked, if one is present.
    ///
    /// If no event handler is registered corresponding to the event `name`, then this method has no
    /// effect.
    ///
    /// This operates directly on the element, so that is is guaranteed to receive the event. This
    /// might not reflect how the element would respond in reality. For example, a click at the
    /// coordinates of a button which is behind a frost element will not reach the button. But this
    /// method behaves as though it would.
    ///
    /// The `event` parameter must contain a [PlatformEventData] with a payload corresponding to the
    /// specific event type. This method panics if the event payload has the wrong type.
    pub fn send_event(&self, name: &str, event: Event<PlatformEventData>) {
        let propagates = event.propagates();
        self.document.vdom.runtime().handle_event(
            name,
            Event::new(event.data, propagates),
            self.get_element_id()
                .expect("Expected element to have a Dioxus ID"),
        );
    }

    /// Returns a `String` consisting of the HTML of this element and all of its children.
    pub fn outer_html(&self) -> String {
        self.node.outer_html()
    }

    /// Returns a `String` consisting of the HTML of this element's children, not including this
    /// element itself.
    pub fn inner_html(&self) -> String {
        let inner_html_parts: Vec<_> = self
            .node
            .children
            .iter()
            .filter_map(|child_id| {
                self.document
                    .get_node(*child_id)
                    .map(|child| child.outer_html())
            })
            .collect();
        inner_html_parts.join("")
    }

    /// Returns the calculated [Coordinates] of the centre of this element.
    pub fn center(&self) -> Coordinates {
        let upper_left = self.upper_left();
        let lower_right = self.lower_right();
        Coordinates::new(
            upper_left.screen().lerp(lower_right.screen(), 0.5),
            upper_left.client().lerp(lower_right.client(), 0.5),
            upper_left.element().lerp(lower_right.element(), 0.5),
            upper_left.page().lerp(lower_right.page(), 0.5),
        )
    }

    /// Returns the calculated [Coordinates] of the upper-left corner of this element.
    pub fn upper_left(&self) -> Coordinates {
        let upper_left = self.node.final_layout.location;
        Coordinates::new(
            Self::to_point2d(upper_left),
            Self::to_point2d(upper_left),
            Self::to_point2d(upper_left),
            Self::to_point2d(upper_left),
        )
    }

    /// Returns the calculated [Coordinates] of the upper-right corner of this element.
    pub fn upper_right(&self) -> Coordinates {
        let mut upper_right = self.node.final_layout.location;
        upper_right.x += self.node.final_layout.content_box_width();
        Coordinates::new(
            Self::to_point2d(upper_right),
            Self::to_point2d(upper_right),
            Self::to_point2d(upper_right),
            Self::to_point2d(upper_right),
        )
    }

    /// Returns the calculated [Coordinates] of the lower-left corner of this element.
    pub fn lower_left(&self) -> Coordinates {
        let mut lower_left = self.node.final_layout.location;
        lower_left.y += self.node.final_layout.content_box_height();
        Coordinates::new(
            Self::to_point2d(lower_left),
            Self::to_point2d(lower_left),
            Self::to_point2d(lower_left),
            Self::to_point2d(lower_left),
        )
    }

    /// Returns the calculated [Coordinates] of the lower-right corner of this element.
    pub fn lower_right(&self) -> Coordinates {
        let mut lower_right = self.node.final_layout.location;
        lower_right.x += self.node.final_layout.content_box_width();
        lower_right.y += self.node.final_layout.content_box_height();
        Coordinates::new(
            Self::to_point2d(lower_right),
            Self::to_point2d(lower_right),
            Self::to_point2d(lower_right),
            Self::to_point2d(lower_right),
        )
    }

    fn to_point2d<Space>(point: taffy::geometry::Point<f32>) -> Point2D<f64, Space> {
        Point2D::new(point.x as f64, point.y as f64)
    }

    /// Returns the calculated size of this element as a tuple (width, height) in screen pixels.
    pub fn size(&self) -> (f32, f32) {
        let height = self.node.final_layout.content_box_height();
        let width = self.node.final_layout.content_box_width();
        (width, height)
    }

    fn get_element_id(&self) -> Option<ElementId> {
        self.node
            .element_data()?
            .attrs
            .iter()
            .find(|attr| *attr.name.local == *"data-dioxus-id")
            .and_then(|attr| attr.value.parse::<usize>().ok())
            .map(ElementId)
    }
}

impl<'doc> std::fmt::Debug for ResolvedElement<'doc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedElement")
            .field("node", &self.node)
            .finish()
    }
}
