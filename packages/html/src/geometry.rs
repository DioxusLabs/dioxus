pub use euclid::*;

pub struct ScreenSpace;
pub type ScreenPoint = Point2D<f64, ScreenSpace>;

pub struct ClientSpace;
pub type ClientPoint = Point2D<f64, ClientSpace>;

pub struct ElementSpace;
pub type ElementPoint = Point2D<f64, ElementSpace>;

pub struct PageSpace;
pub type PagePoint = Point2D<f64, PageSpace>;
