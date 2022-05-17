use crate::converter;
use bevy::{
    input::{keyboard::KeyboardInput, ElementState},
    window::{ReceivedCharacter, WindowId},
};
use serde::Deserialize;
use serde_json::Value;
use serde_repr::*;

#[derive(Debug, Clone)]
pub struct DomUpdated {
    pub id: WindowId,
}

#[derive(Debug, Clone)]
pub struct WindowDragged {
    pub id: WindowId,
}

#[derive(Debug, Clone)]
pub struct WindowMinimized {
    pub id: WindowId,
    pub minimized: bool,
}

#[derive(Debug, Clone)]
pub struct WindowMaximized {
    pub id: WindowId,
    pub maximized: bool,
}

#[derive(Debug, Clone)]
pub struct MaximizeToggled {
    pub id: WindowId,
    pub maximized: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum KeyboardEvent {
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

impl KeyboardEvent {
    pub fn from_value(value: Value) -> KeyboardEvent {
        serde_json::from_value(value).unwrap()
    }

    pub fn to_input(&self) -> KeyboardInput {
        match self {
            KeyboardEvent::Keydown {
                key,
                scan_code,
                location,
            } => KeyboardInput {
                scan_code: *scan_code,
                key_code: converter::try_convert_key_code(key, location),
                state: ElementState::Pressed,
            },
            KeyboardEvent::Keyup {
                key,
                scan_code,
                location,
            } => KeyboardInput {
                scan_code: *scan_code,
                key_code: converter::try_convert_key_code(key, location),
                state: ElementState::Released,
            },
        }
    }

    pub fn try_to_char(&self) -> Option<ReceivedCharacter> {
        let id = WindowId::primary();

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
            KeyboardEvent::Keyup { key, .. } | KeyboardEvent::Keydown { key, .. } => key,
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
