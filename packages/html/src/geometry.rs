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

/// A pixel unit: one unit corresponds to 1 pixel
pub struct Pixels;
/// A vector expressed in Pixels
pub type PixelsVector = Vector3D<f64, Pixels>;

/// A unit in terms of Lines
///
/// One unit is relative to the size of one line
pub struct Lines;
/// A vector expressed in Lines
pub type LinesVector = Vector3D<f64, Lines>;

/// A unit in terms of Screens:
///
/// One unit is relative to the size of a page
pub struct Pages;
/// A vector expressed in Pages
pub type PagesVector = Vector3D<f64, Pages>;

/// A vector representing the amount the mouse wheel was moved
///
/// This may be expressed in Pixels, Lines or Pages
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum WheelDelta {
    /// Movement in Pixels
    Pixels(PixelsVector),
    /// Movement in Lines
    Lines(LinesVector),
    /// Movement in Pages
    Pages(PagesVector),
}

impl WheelDelta {
    /// Construct from the attributes of the web wheel event
    pub fn from_web_attributes(delta_mode: u32, delta_x: f64, delta_y: f64, delta_z: f64) -> Self {
        match delta_mode {
            0 => WheelDelta::Pixels(PixelsVector::new(delta_x, delta_y, delta_z)),
            1 => WheelDelta::Lines(LinesVector::new(delta_x, delta_y, delta_z)),
            2 => WheelDelta::Pages(PagesVector::new(delta_x, delta_y, delta_z)),
            _ => panic!("Invalid delta mode, {:?}", delta_mode),
        }
    }

    /// Convenience function for constructing a WheelDelta with pixel units
    pub fn pixels(x: f64, y: f64, z: f64) -> Self {
        WheelDelta::Pixels(PixelsVector::new(x, y, z))
    }

    /// Convenience function for constructing a WheelDelta with line units
    pub fn lines(x: f64, y: f64, z: f64) -> Self {
        WheelDelta::Lines(LinesVector::new(x, y, z))
    }

    /// Convenience function for constructing a WheelDelta with page units
    pub fn pages(x: f64, y: f64, z: f64) -> Self {
        WheelDelta::Pages(PagesVector::new(x, y, z))
    }

    /// Returns true iff there is no wheel movement
    ///
    /// i.e. the x, y and z delta is zero (disregards units)
    pub fn is_zero(&self) -> bool {
        self.strip_units() == Vector3D::new(0., 0., 0.)
    }

    /// A Vector3D proportional to the amount scrolled
    ///
    /// Note that this disregards the 3 possible units: this could be expressed in terms of pixels, lines, or pages.
    ///
    /// In most cases, to properly handle scrolling, you should handle all 3 possible enum variants instead of stripping units. Otherwise, if you assume that the units will always be pixels, the user may experience some unexpectedly slow scrolling if their mouse/OS sends values expressed in lines or pages.
    pub fn strip_units(&self) -> Vector3D<f64, UnknownUnit> {
        match self {
            WheelDelta::Pixels(v) => v.cast_unit(),
            WheelDelta::Lines(v) => v.cast_unit(),
            WheelDelta::Pages(v) => v.cast_unit(),
        }
    }
}

/// Coordinates of a point in the app's interface
#[derive(Debug, PartialEq)]
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
