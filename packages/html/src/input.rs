use enumset::{EnumSet, EnumSetType};

// note: EnumSetType also derives Copy and Clone for some reason
/// A modifier key, such as Alt or Ctrl
#[derive(EnumSetType, Debug)]
pub enum Modifier {
    Alt,
    Ctrl,
    /// The meta key (windows key, or command key)
    Meta,
    Shift,
}

/// A set of modifier keys
pub type ModifierSet = EnumSet<Modifier>;

/// A mouse button type (such as Primary/Secondary)
// note: EnumSetType also derives Copy and Clone for some reason
#[derive(EnumSetType, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseButton {
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
}

/// A set of mouse buttons
pub type MouseButtonSet = EnumSet<MouseButton>;
