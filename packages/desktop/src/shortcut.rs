#[cfg(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub use global_hotkey::{
    hotkey::{Code, HotKey},
    Error as HotkeyError, GlobalHotKeyEvent, GlobalHotKeyManager,
};

#[cfg(any(target_os = "ios", target_os = "android"))]
pub use crate::mobile_shortcut::*;

use crate::window;
use dioxus_html::input_data::keyboard_types::Modifiers;
use slab::Slab;
use std::{cell::RefCell, collections::HashMap, rc::Rc, str::FromStr};
use tao::keyboard::ModifiersState;

/// An global id for a shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShortcutHandle {
    id: u32,
    number: usize,
}

impl ShortcutHandle {
    /// Remove the shortcut.
    pub fn remove(&self) {
        window().remove_shortcut(*self);
    }
}

/// An error that can occur when registering a shortcut.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ShortcutRegistryError {
    /// The shortcut is invalid.
    InvalidShortcut(String),
    /// An unknown error occurred.
    Other(Rc<dyn std::error::Error>),
}

pub(crate) struct ShortcutRegistry {
    manager: GlobalHotKeyManager,
    shortcuts: RefCell<HashMap<u32, ShortcutInner>>,
}

struct ShortcutInner {
    #[allow(unused)]
    shortcut: HotKey,
    callbacks: Slab<Box<dyn FnMut()>>,
}

impl ShortcutRegistry {
    pub fn new() -> Self {
        Self {
            manager: GlobalHotKeyManager::new().unwrap(),
            shortcuts: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn call_handlers(&self, id: GlobalHotKeyEvent) {
        if let Some(ShortcutInner { callbacks, .. }) = self.shortcuts.borrow_mut().get_mut(&id.id) {
            for (_, callback) in callbacks.iter_mut() {
                (callback)();
            }
        }
    }

    pub(crate) fn add_shortcut(
        &self,
        hotkey: HotKey,
        callback: Box<dyn FnMut()>,
    ) -> Result<ShortcutHandle, ShortcutRegistryError> {
        let accelerator_id = hotkey.clone().id();

        let mut shortcuts = self.shortcuts.borrow_mut();

        if let Some(callbacks) = shortcuts.get_mut(&accelerator_id) {
            return Ok(ShortcutHandle {
                id: accelerator_id,
                number: callbacks.callbacks.insert(callback),
            });
        };

        self.manager.register(hotkey).map_err(|e| match e {
            HotkeyError::HotKeyParseError(shortcut) => {
                ShortcutRegistryError::InvalidShortcut(shortcut)
            }
            err => ShortcutRegistryError::Other(Rc::new(err)),
        })?;

        let mut shortcut = ShortcutInner {
            shortcut: hotkey,
            callbacks: Slab::new(),
        };

        let id = shortcut.callbacks.insert(callback);

        shortcuts.insert(accelerator_id, shortcut);

        Ok(ShortcutHandle {
            id: accelerator_id,
            number: id,
        })
    }

    pub(crate) fn remove_shortcut(&self, id: ShortcutHandle) {
        let mut shortcuts = self.shortcuts.borrow_mut();
        if let Some(callbacks) = shortcuts.get_mut(&id.id) {
            let _ = callbacks.callbacks.remove(id.number);
            if callbacks.callbacks.is_empty() {
                if let Some(_shortcut) = shortcuts.remove(&id.id) {
                    let _ = self.manager.unregister(_shortcut.shortcut);
                }
            }
        }
    }

    pub(crate) fn remove_all(&self) {
        let mut shortcuts = self.shortcuts.borrow_mut();
        let hotkeys: Vec<_> = shortcuts.drain().map(|(_, v)| v.shortcut).collect();
        let _ = self.manager.unregister_all(&hotkeys);
    }
}

pub trait IntoAccelerator {
    fn accelerator(&self) -> HotKey;
}

impl IntoAccelerator for (dioxus_html::KeyCode, ModifiersState) {
    fn accelerator(&self) -> HotKey {
        HotKey::new(Some(self.1.into_modifiers_state()), self.0.into_key_code())
    }
}

impl IntoAccelerator for (ModifiersState, dioxus_html::KeyCode) {
    fn accelerator(&self) -> HotKey {
        HotKey::new(Some(self.0.into_modifiers_state()), self.1.into_key_code())
    }
}

impl IntoAccelerator for dioxus_html::KeyCode {
    fn accelerator(&self) -> HotKey {
        HotKey::new(None, self.into_key_code())
    }
}

impl IntoAccelerator for &str {
    fn accelerator(&self) -> HotKey {
        HotKey::from_str(self).unwrap()
    }
}

pub trait IntoModifersState {
    fn into_modifiers_state(self) -> Modifiers;
}

impl IntoModifersState for ModifiersState {
    fn into_modifiers_state(self) -> Modifiers {
        let mut modifiers = Modifiers::default();
        if self.shift_key() {
            modifiers |= Modifiers::SHIFT;
        }
        if self.control_key() {
            modifiers |= Modifiers::CONTROL;
        }
        if self.alt_key() {
            modifiers |= Modifiers::ALT;
        }
        if self.super_key() {
            modifiers |= Modifiers::META;
        }

        modifiers
    }
}

impl IntoModifersState for Modifiers {
    fn into_modifiers_state(self) -> Modifiers {
        self
    }
}

pub trait IntoKeyCode {
    fn into_key_code(self) -> Code;
}

impl IntoKeyCode for Code {
    fn into_key_code(self) -> Code {
        self
    }
}

impl IntoKeyCode for dioxus_html::KeyCode {
    fn into_key_code(self) -> Code {
        match self {
            dioxus_html::KeyCode::Backspace => Code::Backspace,
            dioxus_html::KeyCode::Tab => Code::Tab,
            dioxus_html::KeyCode::Clear => Code::NumpadClear,
            dioxus_html::KeyCode::Enter => Code::Enter,
            dioxus_html::KeyCode::Shift => Code::ShiftLeft,
            dioxus_html::KeyCode::Ctrl => Code::ControlLeft,
            dioxus_html::KeyCode::Alt => Code::AltLeft,
            dioxus_html::KeyCode::Pause => Code::Pause,
            dioxus_html::KeyCode::CapsLock => Code::CapsLock,
            dioxus_html::KeyCode::Escape => Code::Escape,
            dioxus_html::KeyCode::Space => Code::Space,
            dioxus_html::KeyCode::PageUp => Code::PageUp,
            dioxus_html::KeyCode::PageDown => Code::PageDown,
            dioxus_html::KeyCode::End => Code::End,
            dioxus_html::KeyCode::Home => Code::Home,
            dioxus_html::KeyCode::LeftArrow => Code::ArrowLeft,
            dioxus_html::KeyCode::UpArrow => Code::ArrowUp,
            dioxus_html::KeyCode::RightArrow => Code::ArrowRight,
            dioxus_html::KeyCode::DownArrow => Code::ArrowDown,
            dioxus_html::KeyCode::Insert => Code::Insert,
            dioxus_html::KeyCode::Delete => Code::Delete,
            dioxus_html::KeyCode::Num0 => Code::Numpad0,
            dioxus_html::KeyCode::Num1 => Code::Numpad1,
            dioxus_html::KeyCode::Num2 => Code::Numpad2,
            dioxus_html::KeyCode::Num3 => Code::Numpad3,
            dioxus_html::KeyCode::Num4 => Code::Numpad4,
            dioxus_html::KeyCode::Num5 => Code::Numpad5,
            dioxus_html::KeyCode::Num6 => Code::Numpad6,
            dioxus_html::KeyCode::Num7 => Code::Numpad7,
            dioxus_html::KeyCode::Num8 => Code::Numpad8,
            dioxus_html::KeyCode::Num9 => Code::Numpad9,
            dioxus_html::KeyCode::A => Code::KeyA,
            dioxus_html::KeyCode::B => Code::KeyB,
            dioxus_html::KeyCode::C => Code::KeyC,
            dioxus_html::KeyCode::D => Code::KeyD,
            dioxus_html::KeyCode::E => Code::KeyE,
            dioxus_html::KeyCode::F => Code::KeyF,
            dioxus_html::KeyCode::G => Code::KeyG,
            dioxus_html::KeyCode::H => Code::KeyH,
            dioxus_html::KeyCode::I => Code::KeyI,
            dioxus_html::KeyCode::J => Code::KeyJ,
            dioxus_html::KeyCode::K => Code::KeyK,
            dioxus_html::KeyCode::L => Code::KeyL,
            dioxus_html::KeyCode::M => Code::KeyM,
            dioxus_html::KeyCode::N => Code::KeyN,
            dioxus_html::KeyCode::O => Code::KeyO,
            dioxus_html::KeyCode::P => Code::KeyP,
            dioxus_html::KeyCode::Q => Code::KeyQ,
            dioxus_html::KeyCode::R => Code::KeyR,
            dioxus_html::KeyCode::S => Code::KeyS,
            dioxus_html::KeyCode::T => Code::KeyT,
            dioxus_html::KeyCode::U => Code::KeyU,
            dioxus_html::KeyCode::V => Code::KeyV,
            dioxus_html::KeyCode::W => Code::KeyW,
            dioxus_html::KeyCode::X => Code::KeyX,
            dioxus_html::KeyCode::Y => Code::KeyY,
            dioxus_html::KeyCode::Z => Code::KeyZ,
            dioxus_html::KeyCode::Numpad0 => Code::Numpad0,
            dioxus_html::KeyCode::Numpad1 => Code::Numpad1,
            dioxus_html::KeyCode::Numpad2 => Code::Numpad2,
            dioxus_html::KeyCode::Numpad3 => Code::Numpad3,
            dioxus_html::KeyCode::Numpad4 => Code::Numpad4,
            dioxus_html::KeyCode::Numpad5 => Code::Numpad5,
            dioxus_html::KeyCode::Numpad6 => Code::Numpad6,
            dioxus_html::KeyCode::Numpad7 => Code::Numpad7,
            dioxus_html::KeyCode::Numpad8 => Code::Numpad8,
            dioxus_html::KeyCode::Numpad9 => Code::Numpad9,
            dioxus_html::KeyCode::Multiply => Code::NumpadMultiply,
            dioxus_html::KeyCode::Add => Code::NumpadAdd,
            dioxus_html::KeyCode::Subtract => Code::NumpadSubtract,
            dioxus_html::KeyCode::DecimalPoint => Code::NumpadDecimal,
            dioxus_html::KeyCode::Divide => Code::NumpadDivide,
            dioxus_html::KeyCode::F1 => Code::F1,
            dioxus_html::KeyCode::F2 => Code::F2,
            dioxus_html::KeyCode::F3 => Code::F3,
            dioxus_html::KeyCode::F4 => Code::F4,
            dioxus_html::KeyCode::F5 => Code::F5,
            dioxus_html::KeyCode::F6 => Code::F6,
            dioxus_html::KeyCode::F7 => Code::F7,
            dioxus_html::KeyCode::F8 => Code::F8,
            dioxus_html::KeyCode::F9 => Code::F9,
            dioxus_html::KeyCode::F10 => Code::F10,
            dioxus_html::KeyCode::F11 => Code::F11,
            dioxus_html::KeyCode::F12 => Code::F12,
            dioxus_html::KeyCode::NumLock => Code::NumLock,
            dioxus_html::KeyCode::ScrollLock => Code::ScrollLock,
            dioxus_html::KeyCode::Semicolon => Code::Semicolon,
            dioxus_html::KeyCode::EqualSign => Code::Equal,
            dioxus_html::KeyCode::Comma => Code::Comma,
            dioxus_html::KeyCode::Period => Code::Period,
            dioxus_html::KeyCode::ForwardSlash => Code::Slash,
            dioxus_html::KeyCode::GraveAccent => Code::Backquote,
            dioxus_html::KeyCode::OpenBracket => Code::BracketLeft,
            dioxus_html::KeyCode::BackSlash => Code::Backslash,
            dioxus_html::KeyCode::CloseBraket => Code::BracketRight,
            dioxus_html::KeyCode::SingleQuote => Code::Quote,
            key => panic!("Failed to convert {:?} to tao::keyboard::KeyCode, try using tao::keyboard::KeyCode directly", key),
        }
    }
}
