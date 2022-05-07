//! Geometry primitives for representing e.g. mouse events

/// A re-export of euclid, which we use for geometry primitives
pub use euclid;

use euclid::*;

/// Coordinate space relative to the screen
pub struct ScreenSpace;
/// A point in ScreenSpace
pub type ScreenPoint = Point2D<f64, ScreenSpace>;

/// Coordinate space relative to the viewport
pub struct ClientSpace;
/// A point in ClientSpace
pub type ClientPoint = Point2D<f64, ClientSpace>;

/// Coordinate space relative to an element
pub struct ElementSpace;
/// A point in ElementSpace
pub type ElementPoint = Point2D<f64, ElementSpace>;

/// Coordinate space relative to the page
pub struct PageSpace;
/// A point in PageSpace
pub type PagePoint = Point2D<f64, PageSpace>;

/// Coordinates of a point in the app's interface
pub struct Coordinates {
    screen: ScreenPoint,
    client: ClientPoint,
    element: ElementPoint,
    page: PagePoint,
}

impl Coordinates {
    /// Construct new coordinates with the specified screen-, client-, element- and page-relative points
    pub fn new(
        screen: ScreenPoint,
        client: ClientPoint,
        element: ElementPoint,
        page: PagePoint,
    ) -> Self {
        Self {
            screen,
            client,
            element,
            page,
        }
    }
    /// Coordinates relative to the entire screen. This takes into account the window's offset.
    pub fn screen(&self) -> ScreenPoint {
        self.screen
    }
    /// Coordinates relative to the application's viewport (as opposed to the coordinate within the page).
    ///
    /// For example, clicking in the top left corner of the viewport will always result in a mouse event with client coordinates (0., 0.), regardless of whether the page is scrolled horizontally.
    pub fn client(&self) -> ClientPoint {
        self.client
    }
    /// Coordinates relative to the padding edge of the target element
    ///
    /// For example, clicking in the top left corner of an element will result in element coordinates (0., 0.)
    pub fn element(&self) -> ElementPoint {
        self.element
    }
    /// Coordinates relative to the entire document. This includes any portion of the document not currently visible.
    ///
    /// For example, if the page is scrolled 200 pixels to the right and 300 pixels down, clicking in the top left corner of the viewport would result in page coordinates (200., 300.)
    pub fn page(&self) -> PagePoint {
        self.page
    }
}
