use bevy::input::{
    keyboard::{KeyCode, KeyboardInput},
    ElementState,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum CustomUserEvent<CoreCommand> {
    CoreCommand(CoreCommand),
    KeyboardInput(KeyboardInput),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum WebKeyboardEvent {
    #[serde(rename = "keydown")]
    Keydown {
        code: String,
        #[serde(rename = "keyCode")]
        scan_code: u32,
    },
    #[serde(rename = "keyup")]
    Keyup {
        code: String,
        #[serde(rename = "keyCode")]
        scan_code: u32,
    },
}

pub fn parse_keyboard_input(val: Value) -> KeyboardInput {
    let event: WebKeyboardEvent = serde_json::from_value(val).unwrap();

    match event {
        WebKeyboardEvent::Keydown { code, scan_code } => KeyboardInput {
            scan_code,
            key_code: try_parse_code(code),
            state: ElementState::Pressed,
        },
        WebKeyboardEvent::Keyup { code, scan_code } => KeyboardInput {
            scan_code,
            key_code: try_parse_code(code),
            state: ElementState::Released,
        },
    }
}

// reference: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key/Key_Values
pub fn try_parse_code(code: String) -> Option<KeyCode> {
    match code.as_str() {
        "Digit0" => Some(KeyCode::Key0),
        "Digit1" => Some(KeyCode::Key1),
        "Digit2" => Some(KeyCode::Key2),
        "Digit3" => Some(KeyCode::Key3),
        "Digit4" => Some(KeyCode::Key4),
        "Digit5" => Some(KeyCode::Key5),
        "Digit6" => Some(KeyCode::Key6),
        "Digit7" => Some(KeyCode::Key7),
        "Digit8" => Some(KeyCode::Key8),
        "Digit9" => Some(KeyCode::Key9),

        "KeyA" => Some(KeyCode::A),
        "KeyB" => Some(KeyCode::B),
        "KeyC" => Some(KeyCode::C),
        "KeyD" => Some(KeyCode::D),
        "KeyE" => Some(KeyCode::E),
        "KeyF" => Some(KeyCode::F),
        "KeyG" => Some(KeyCode::G),
        "KeyH" => Some(KeyCode::H),
        "KeyI" => Some(KeyCode::I),
        "KeyJ" => Some(KeyCode::J),
        "KeyK" => Some(KeyCode::K),
        "KeyL" => Some(KeyCode::L),
        "KeyM" => Some(KeyCode::M),
        "KeyN" => Some(KeyCode::N),
        "KeyO" => Some(KeyCode::O),
        "KeyP" => Some(KeyCode::P),
        "KeyQ" => Some(KeyCode::Q),
        "KeyR" => Some(KeyCode::R),
        "KeyS" => Some(KeyCode::S),
        "KeyT" => Some(KeyCode::T),
        "KeyU" => Some(KeyCode::U),
        "KeyV" => Some(KeyCode::V),
        "KeyW" => Some(KeyCode::W),
        "KeyX" => Some(KeyCode::X),
        "KeyY" => Some(KeyCode::Y),
        "KeyZ" => Some(KeyCode::Z),

        "Escape" => Some(KeyCode::Escape),

        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7),
        "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9),
        "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11),
        "F12" => Some(KeyCode::F12),
        "F13" => Some(KeyCode::F13),
        "F14" => Some(KeyCode::F14),
        "F15" => Some(KeyCode::F15),
        "F16" => Some(KeyCode::F16),
        "F17" => Some(KeyCode::F17),
        "F18" => Some(KeyCode::F18),
        "F19" => Some(KeyCode::F19),
        "F20" => Some(KeyCode::F20),
        "F21" => Some(KeyCode::F21),
        "F22" => Some(KeyCode::F22),
        "F23" => Some(KeyCode::F23),
        "F24" => Some(KeyCode::F24),

        "PrintScreen" => Some(KeyCode::Snapshot),
        "ScrollLock" => Some(KeyCode::Scroll),
        "Pause" => Some(KeyCode::Pause),

        "Insert" => Some(KeyCode::Insert),
        "Home" => Some(KeyCode::Home),
        "Delete" => Some(KeyCode::Delete),
        "End" => Some(KeyCode::Delete),
        "PageDown" => Some(KeyCode::PageDown),
        "PageUp" => Some(KeyCode::PageUp),

        "Left" | "ArrowLeft" => Some(KeyCode::Left),
        "Up" | "ArrowUp" => Some(KeyCode::Up),
        "Right" | "ArrowRight" => Some(KeyCode::Right),
        "Down" | "ArrowDown" => Some(KeyCode::Down),

        "Backspace" => Some(KeyCode::Back),
        "Enter" => Some(KeyCode::Return),
        "Space" => Some(KeyCode::Space),

        "Compose" => Some(KeyCode::Compose),

        // Caret,
        "NumLock" => Some(KeyCode::Numlock),
        "Numpad0" => Some(KeyCode::Numpad0),
        "Numpad1" => Some(KeyCode::Numpad1),
        "Numpad2" => Some(KeyCode::Numpad2),
        "Numpad3" => Some(KeyCode::Numpad3),
        "Numpad4" => Some(KeyCode::Numpad4),
        "Numpad5" => Some(KeyCode::Numpad5),
        "Numpad6" => Some(KeyCode::Numpad6),
        "Numpad7" => Some(KeyCode::Numpad7),
        "Numpad8" => Some(KeyCode::Numpad8),
        "Numpad9" => Some(KeyCode::Numpad9),

        // AbntC1,
        // AbntC2,
        "NumpadAdd" => Some(KeyCode::NumpadAdd),
        "Quote" => Some(KeyCode::Apostrophe),
        // Apps,
        // Asterisk,
        // Plus,
        // At,
        // Ax,
        "Backslash" => Some(KeyCode::Backslash),
        // Calculator,
        // Capital,
        // Colon,
        "Comma" => Some(KeyCode::Comma),
        "Convert" => Some(KeyCode::Convert),
        "NumpadDecimal" => Some(KeyCode::NumpadDecimal),
        "NumpadDivide" => Some(KeyCode::NumpadDivide),
        "Equal" => Some(KeyCode::Equals),
        "Backquote" => Some(KeyCode::Grave),
        // Kana,
        // Kanji,
        "AltLeft" => Some(KeyCode::LAlt),
        "BracketLeft" => Some(KeyCode::LBracket),
        "ControlLeft" => Some(KeyCode::LControl),
        "ShiftLeft" => Some(KeyCode::LShift),
        "MetaLeft" => Some(KeyCode::LWin),
        // Mail,
        // MediaSelect,
        // MediaStop,
        "Minus" => Some(KeyCode::Minus),
        "NumpadMultiply" => Some(KeyCode::NumpadMultiply),
        // Mute,
        // MyComputer,
        // "BrowserForward" => Some(KeyCode::NavigateForward),
        // "BrowserBackward" => Some(KeyCode::NavigateBackward),
        // NextTrack,
        "NonConvert" => Some(KeyCode::NoConvert),
        "NumpadComma" => Some(KeyCode::NumpadComma),
        "NumpadEnter" => Some(KeyCode::NumpadEnter),
        "NumpadEqual" => Some(KeyCode::NumpadEquals),
        // Oem102,
        "Period" => Some(KeyCode::Period),
        // PlayPause,
        "Power" => Some(KeyCode::Power),
        // PrevTrack,
        "AltRight" => Some(KeyCode::RAlt),
        "BracketRight" => Some(KeyCode::RBracket),
        "ControlRight" => Some(KeyCode::RControl),
        "ShiftRight" => Some(KeyCode::RShift),
        "MetaRight" => Some(KeyCode::RWin),
        "Semicolon" => Some(KeyCode::Semicolon),
        "Slash" => Some(KeyCode::Slash),
        // Sleep,
        // Stop,
        "NumpadSubtract" => Some(KeyCode::NumpadSubtract),
        // Sysrq,
        "Tab" => Some(KeyCode::Tab),
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
        "IntlYen" => Some(KeyCode::Yen),
        "Copy" => Some(KeyCode::Copy),
        "Paste" => Some(KeyCode::Paste),
        "Cut" => Some(KeyCode::Cut),

        _ => None,
    }
}
