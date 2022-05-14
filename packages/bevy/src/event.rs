use bevy::{
    input::{
        keyboard::{KeyCode, KeyboardInput},
        ElementState,
    },
    window::{ReceivedCharacter, WindowId},
};
use serde::Deserialize;
use serde_json::Value;
use serde_repr::*;

#[derive(Debug, Clone)]
pub struct VirtualDomUpdated {
    pub window_id: WindowId,
}

#[derive(Debug, Clone)]
pub struct WindowDragged {
    pub window_id: WindowId,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum WebKeyboardEvent {
    #[serde(rename = "keydown")]
    Keydown {
        key: String,
        #[serde(rename = "key_code")]
        scan_code: u32,
        location: Location,
    },
    #[serde(rename = "keyup")]
    Keyup {
        key: String,
        #[serde(rename = "key_code")]
        scan_code: u32,
        location: Location,
    },
}

impl WebKeyboardEvent {
    pub fn from_value(value: Value) -> WebKeyboardEvent {
        serde_json::from_value(value).unwrap()
    }

    pub fn to_input(&self) -> KeyboardInput {
        match self {
            WebKeyboardEvent::Keydown {
                key,
                scan_code,
                location,
            } => KeyboardInput {
                scan_code: *scan_code,
                key_code: Self::try_parse_key(key, location),
                state: ElementState::Pressed,
            },
            WebKeyboardEvent::Keyup {
                key,
                scan_code,
                location,
            } => KeyboardInput {
                scan_code: *scan_code,
                key_code: Self::try_parse_key(key, location),
                state: ElementState::Released,
            },
        }
    }

    pub fn try_to_char(&self) -> Option<ReceivedCharacter> {
        let id = WindowId::primary();
        println!("key: {}", self.key());

        match self.key() {
            "Enter" => Some(ReceivedCharacter { id, char: '\r' }),
            "Backspace" => Some(ReceivedCharacter { id, char: '\u{7f}' }),
            key if key.len() > 1 => None,
            _ => Some(ReceivedCharacter {
                id,
                char: self.key().chars().next().unwrap(),
            }),
        }
    }

    pub fn key(&self) -> &str {
        match self {
            WebKeyboardEvent::Keyup { key, .. } | WebKeyboardEvent::Keydown { key, .. } => key,
        }
    }

    // reference: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key/Key_Values
    fn try_parse_key(key: &String, location: &Location) -> Option<KeyCode> {
        match (key.as_str(), location) {
            ("0" | ")", Location::Standard) => Some(KeyCode::Key0),
            ("1" | "!", Location::Standard) => Some(KeyCode::Key1),
            ("2" | "@", Location::Standard) => Some(KeyCode::Key2),
            ("3" | "#", Location::Standard) => Some(KeyCode::Key3),
            ("4" | "$", Location::Standard) => Some(KeyCode::Key4),
            ("5" | "%", Location::Standard) => Some(KeyCode::Key5),
            ("6" | "^", Location::Standard) => Some(KeyCode::Key6),
            ("7" | "&", Location::Standard) => Some(KeyCode::Key7),
            ("8" | "*", Location::Standard) => Some(KeyCode::Key8),
            ("9" | "(", Location::Standard) => Some(KeyCode::Key9),

            ("a" | "A", _) => Some(KeyCode::A),
            ("b" | "B", _) => Some(KeyCode::B),
            ("c" | "C", _) => Some(KeyCode::C),
            ("d" | "D", _) => Some(KeyCode::D),
            ("e" | "E", _) => Some(KeyCode::E),
            ("f" | "F", _) => Some(KeyCode::F),
            ("g" | "G", _) => Some(KeyCode::G),
            ("h" | "H", _) => Some(KeyCode::H),
            ("i" | "I", _) => Some(KeyCode::I),
            ("j" | "J", _) => Some(KeyCode::J),
            ("k" | "K", _) => Some(KeyCode::K),
            ("l" | "L", _) => Some(KeyCode::L),
            ("m" | "M", _) => Some(KeyCode::M),
            ("n" | "N", _) => Some(KeyCode::N),
            ("o" | "O", _) => Some(KeyCode::O),
            ("p" | "P", _) => Some(KeyCode::P),
            ("q" | "Q", _) => Some(KeyCode::Q),
            ("r" | "R", _) => Some(KeyCode::R),
            ("s" | "S", _) => Some(KeyCode::S),
            ("t" | "T", _) => Some(KeyCode::T),
            ("u" | "U", _) => Some(KeyCode::U),
            ("v" | "V", _) => Some(KeyCode::V),
            ("w" | "W", _) => Some(KeyCode::W),
            ("x" | "X", _) => Some(KeyCode::X),
            ("y" | "Y", _) => Some(KeyCode::Y),
            ("z" | "Z", _) => Some(KeyCode::Z),

            ("Escape", _) => Some(KeyCode::Escape),

            ("F1", _) => Some(KeyCode::F1),
            ("F2", _) => Some(KeyCode::F2),
            ("F3", _) => Some(KeyCode::F3),
            ("F4", _) => Some(KeyCode::F4),
            ("F5", _) => Some(KeyCode::F5),
            ("F6", _) => Some(KeyCode::F6),
            ("F7", _) => Some(KeyCode::F7),
            ("F8", _) => Some(KeyCode::F8),
            ("F9", _) => Some(KeyCode::F9),
            ("F10", _) => Some(KeyCode::F10),
            ("F11", _) => Some(KeyCode::F11),
            ("F12", _) => Some(KeyCode::F12),
            ("F13", _) => Some(KeyCode::F13),
            ("F14", _) => Some(KeyCode::F14),
            ("F15", _) => Some(KeyCode::F15),
            ("F16", _) => Some(KeyCode::F16),
            ("F17", _) => Some(KeyCode::F17),
            ("F18", _) => Some(KeyCode::F18),
            ("F19", _) => Some(KeyCode::F19),
            ("F20", _) => Some(KeyCode::F20),
            ("F21", _) => Some(KeyCode::F21),
            ("F22", _) => Some(KeyCode::F22),
            ("F23", _) => Some(KeyCode::F23),
            ("F24", _) => Some(KeyCode::F24),

            ("PrintScreen", _) => Some(KeyCode::Snapshot),
            ("ScrollLock", _) => Some(KeyCode::Scroll),
            ("Pause", _) => Some(KeyCode::Pause),

            ("Insert", _) => Some(KeyCode::Insert),
            ("Home", _) => Some(KeyCode::Home),
            ("Delete", _) => Some(KeyCode::Delete),
            ("End", _) => Some(KeyCode::Delete),
            ("PageDown", _) => Some(KeyCode::PageDown),
            ("PageUp", _) => Some(KeyCode::PageUp),

            ("Left", _) | ("ArrowLeft", _) => Some(KeyCode::Left),
            ("Up", _) | ("ArrowUp", _) => Some(KeyCode::Up),
            ("Right", _) | ("ArrowRight", _) => Some(KeyCode::Right),
            ("Down", _) | ("ArrowDown", _) => Some(KeyCode::Down),

            ("Backspace", _) => Some(KeyCode::Back),
            ("Enter", Location::Standard) => Some(KeyCode::Return),
            ("Space", _) => Some(KeyCode::Space),

            ("Compose", _) => Some(KeyCode::Compose),

            // Caret,
            ("NumLock", _) => Some(KeyCode::Numlock),
            ("0", Location::Numpad) => Some(KeyCode::Numpad0),
            ("1", Location::Numpad) => Some(KeyCode::Numpad1),
            ("2", Location::Numpad) => Some(KeyCode::Numpad2),
            ("3", Location::Numpad) => Some(KeyCode::Numpad3),
            ("4", Location::Numpad) => Some(KeyCode::Numpad4),
            ("5", Location::Numpad) => Some(KeyCode::Numpad5),
            ("6", Location::Numpad) => Some(KeyCode::Numpad6),
            ("7", Location::Numpad) => Some(KeyCode::Numpad7),
            ("8", Location::Numpad) => Some(KeyCode::Numpad8),
            ("9", Location::Numpad) => Some(KeyCode::Numpad9),

            // AbntC1,
            // AbntC2,
            ("NumpadAdd", _) => Some(KeyCode::NumpadAdd),
            ("'" | "\"", _) => Some(KeyCode::Apostrophe),
            // Apps,
            // Asterisk,
            // Plus,
            // At,
            // Ax,
            ("\\" | "|", _) => Some(KeyCode::Backslash),
            // Calculator,
            // Capital,
            // Colon,
            ("," | "<", _) => Some(KeyCode::Comma),
            ("Convert", _) => Some(KeyCode::Convert),
            ("NumpadDecimal", _) => Some(KeyCode::NumpadDecimal),
            ("NumpadDivide", _) => Some(KeyCode::NumpadDivide),
            ("=" | "+", _) => Some(KeyCode::Equals),
            ("`" | "~", _) => Some(KeyCode::Grave),
            // Kana,
            // Kanji,
            ("Alt", Location::Left) => Some(KeyCode::LAlt),
            ("Bracket", Location::Left) => Some(KeyCode::LBracket),
            ("Control", Location::Left) => Some(KeyCode::LControl),
            ("Shift", Location::Left) => Some(KeyCode::LShift),
            ("Meta", Location::Left) => Some(KeyCode::LWin),
            // Mail,
            // MediaSelect,
            // MediaStop,
            ("-" | "_", _) => Some(KeyCode::Minus),
            ("Multiply", Location::Numpad) => Some(KeyCode::NumpadMultiply),
            // Mute,
            // MyComputer,
            // "BrowserForward" => Some(KeyCode::NavigateForward),
            // "BrowserBackward" => Some(KeyCode::NavigateBackward),
            // NextTrack,
            ("NonConvert", _) => Some(KeyCode::NoConvert),
            ("Comma", Location::Numpad) => Some(KeyCode::NumpadComma),
            ("Enter", Location::Numpad) => Some(KeyCode::NumpadEnter),
            ("Equal", Location::Numpad) => Some(KeyCode::NumpadEquals),
            // Oem102,
            ("." | ">", _) => Some(KeyCode::Period),
            // PlayPause,
            ("Power", _) => Some(KeyCode::Power),
            // PrevTrack,
            ("Alt", Location::Right) => Some(KeyCode::RAlt),
            ("Bracket", Location::Right) => Some(KeyCode::RBracket),
            ("Control", Location::Right) => Some(KeyCode::RControl),
            ("Shift", Location::Right) => Some(KeyCode::RShift),
            ("Meta", Location::Right) => Some(KeyCode::RWin),
            (";" | ":", _) => Some(KeyCode::Semicolon),
            ("/" | "?", _) => Some(KeyCode::Slash),
            // Sleep,
            // Stop,
            ("NumpadSubtract", _) => Some(KeyCode::NumpadSubtract),
            // Sysrq,
            ("Tab", _) => Some(KeyCode::Tab),
            // Underline,
            // Unlabeled,
            // VolumeDown,
            // VolumeUp,
            // Wake,
            // WebBack,
            // WebFavorites,
            // WebForward,
            // WebHome,
            // WebRefresh,
            // WebSearch,
            // WebStop,
            ("IntlYen", _) => Some(KeyCode::Yen),
            ("Copy", _) => Some(KeyCode::Copy),
            ("Paste", _) => Some(KeyCode::Paste),
            ("Cut", _) => Some(KeyCode::Cut),

            _ => None,
        }
    }
}

#[derive(Deserialize_repr, Debug, Clone)]
#[repr(u8)]
pub enum Location {
    Standard,
    Left,
    Right,
    Numpad,
    Mobile,
    Joystick,
}
