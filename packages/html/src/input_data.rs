//! Data structures representing user input, such as modifier keys and mouse buttons
use enumset::{EnumSet, EnumSetType};

/// A re-export of keyboard_types
pub use keyboard_types;
use keyboard_types::Location;

/// A mouse button type (such as Primary/Secondary)
// note: EnumSetType also derives Copy and Clone for some reason
#[allow(clippy::unused_unit)]
#[derive(EnumSetType, Debug, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseButton {
    #[default]
    /// Primary button (typically the left button)
    Primary,
    /// Secondary button (typically the right button)
    Secondary,
    /// Auxiliary button (typically the middle button)
    Auxiliary,
    /// Fourth button (typically the "Browser Back" button)
    Fourth,
    /// Fifth button (typically the "Browser Forward" button)
    Fifth,
    /// A button with an unknown code
    Unknown,
}

impl MouseButton {
    /// Constructs a MouseButton for the specified button code
    ///
    /// E.g. 0 => Primary; 1 => Auxiliary
    ///
    /// Unknown codes get mapped to MouseButton::Unknown.
    pub fn from_web_code(code: i16) -> Self {
        match code {
            0 => MouseButton::Primary,
            // not a typo; auxiliary and secondary are swapped unlike in the `buttons` field.
            // https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/button
            1 => MouseButton::Auxiliary,
            2 => MouseButton::Secondary,
            3 => MouseButton::Fourth,
            4 => MouseButton::Fifth,
            _ => MouseButton::Unknown,
        }
    }

    /// Converts MouseButton into the corresponding button code
    ///
    /// MouseButton::Unknown will get mapped to -1
    pub fn into_web_code(self) -> i16 {
        match self {
            MouseButton::Primary => 0,
            // not a typo; auxiliary and secondary are swapped unlike in the `buttons` field.
            // https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/button
            MouseButton::Auxiliary => 1,
            MouseButton::Secondary => 2,
            MouseButton::Fourth => 3,
            MouseButton::Fifth => 4,
            MouseButton::Unknown => -1,
        }
    }
}

/// A set of mouse buttons
pub type MouseButtonSet = EnumSet<MouseButton>;

pub fn decode_mouse_button_set(code: u16) -> MouseButtonSet {
    let mut set = EnumSet::empty();

    // https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/buttons
    #[allow(deprecated)]
    {
        if code & 0b1 != 0 {
            set |= MouseButton::Primary;
        }
        if code & 0b10 != 0 {
            set |= MouseButton::Secondary;
        }
        if code & 0b100 != 0 {
            set |= MouseButton::Auxiliary;
        }
        if code & 0b1000 != 0 {
            set |= MouseButton::Fourth;
        }
        if code & 0b10000 != 0 {
            set |= MouseButton::Fifth;
        }
        if code & (!0b11111) != 0 {
            set |= MouseButton::Unknown;
        }
    }

    set
}

pub fn encode_mouse_button_set(set: MouseButtonSet) -> u16 {
    let mut code = 0;

    // https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/buttons
    {
        if set.contains(MouseButton::Primary) {
            code |= 0b1;
        }
        if set.contains(MouseButton::Secondary) {
            code |= 0b10;
        }
        if set.contains(MouseButton::Auxiliary) {
            code |= 0b100;
        }
        if set.contains(MouseButton::Fourth) {
            code |= 0b1000;
        }
        if set.contains(MouseButton::Fifth) {
            code |= 0b10000;
        }
        if set.contains(MouseButton::Unknown) {
            code |= 0b100000;
        }
    }

    code
}

pub fn decode_key_location(code: usize) -> Location {
    match code {
        0 => Location::Standard,
        1 => Location::Left,
        2 => Location::Right,
        3 => Location::Numpad,
        // keyboard_types doesn't yet support mobile/joystick locations
        4 | 5 => Location::Standard,
        // unknown location; Standard seems better than panicking
        _ => Location::Standard,
    }
}

pub fn encode_key_location(location: Location) -> usize {
    match location {
        Location::Standard => 0,
        Location::Left => 1,
        Location::Right => 2,
        Location::Numpad => 3,
    }
}
