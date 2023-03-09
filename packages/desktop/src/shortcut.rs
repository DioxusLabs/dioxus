use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dioxus_core::ScopeState;
use dioxus_html::input_data::keyboard_types::Modifiers;
use slab::Slab;
use wry::application::{
    accelerator::{Accelerator, AcceleratorId},
    event_loop::EventLoopWindowTarget,
    global_shortcut::{GlobalShortcut, ShortcutManager, ShortcutManagerError},
    keyboard::{KeyCode, ModifiersState},
};

use crate::{use_window, DesktopContext};

#[derive(Clone)]
pub(crate) struct ShortcutRegistry {
    manager: Rc<RefCell<ShortcutManager>>,
    shortcuts: ShortcutMap,
}

type ShortcutMap = Rc<RefCell<HashMap<AcceleratorId, Shortcut>>>;

struct Shortcut {
    shortcut: GlobalShortcut,
    callbacks: Slab<Box<dyn FnMut()>>,
}

impl Shortcut {
    fn insert(&mut self, callback: Box<dyn FnMut()>) -> usize {
        self.callbacks.insert(callback)
    }

    fn remove(&mut self, id: usize) {
        let _ = self.callbacks.remove(id);
    }

    fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }
}

impl ShortcutRegistry {
    pub fn new<T>(target: &EventLoopWindowTarget<T>) -> Self {
        let myself = Self {
            manager: Rc::new(RefCell::new(ShortcutManager::new(target))),
            shortcuts: Rc::new(RefCell::new(HashMap::new())),
        };
        // prevent CTRL+R from reloading the page which breaks apps
        let _ = myself.add_shortcut(
            Some(ModifiersState::CONTROL),
            KeyCode::KeyR,
            Box::new(|| {}),
        );
        myself
    }

    pub(crate) fn call_handlers(&self, id: AcceleratorId) {
        if let Some(Shortcut { callbacks, .. }) = self.shortcuts.borrow_mut().get_mut(&id) {
            for (_, callback) in callbacks.iter_mut() {
                (callback)();
            }
        }
    }

    pub(crate) fn add_shortcut(
        &self,
        modifiers: impl Into<Option<ModifiersState>>,
        key: KeyCode,
        callback: Box<dyn FnMut()>,
    ) -> Result<ShortcutId, ShortcutRegistryError> {
        let accelerator = Accelerator::new(modifiers, key);
        let accelerator_id = accelerator.clone().id();
        let mut shortcuts = self.shortcuts.borrow_mut();
        Ok(
            if let Some(callbacks) = shortcuts.get_mut(&accelerator_id) {
                let id = callbacks.insert(callback);
                ShortcutId {
                    id: accelerator_id,
                    number: id,
                }
            } else {
                match self.manager.borrow_mut().register(accelerator) {
                    Ok(global_shortcut) => {
                        let mut slab = Slab::new();
                        let id = slab.insert(callback);
                        let shortcut = Shortcut {
                            shortcut: global_shortcut,
                            callbacks: slab,
                        };
                        shortcuts.insert(accelerator_id, shortcut);
                        ShortcutId {
                            id: accelerator_id,
                            number: id,
                        }
                    }
                    Err(ShortcutManagerError::InvalidAccelerator(shortcut)) => {
                        return Err(ShortcutRegistryError::InvalidShortcut(shortcut))
                    }
                    Err(err) => return Err(ShortcutRegistryError::Other(Box::new(err))),
                }
            },
        )
    }

    pub(crate) fn remove_shortcut(&self, id: ShortcutId) {
        let mut shortcuts = self.shortcuts.borrow_mut();
        if let Some(callbacks) = shortcuts.get_mut(&id.id) {
            callbacks.remove(id.number);
            if callbacks.is_empty() {
                if let Some(shortcut) = shortcuts.remove(&id.id) {
                    let _ = self.manager.borrow_mut().unregister(shortcut.shortcut);
                }
            }
        }
    }

    pub(crate) fn remove_all(&self) {
        let mut shortcuts = self.shortcuts.borrow_mut();
        shortcuts.clear();
        let _ = self.manager.borrow_mut().unregister_all();
        // prevent CTRL+R from reloading the page which breaks apps
        let _ = self.add_shortcut(
            Some(ModifiersState::CONTROL),
            KeyCode::KeyR,
            Box::new(|| {}),
        );
    }
}

#[non_exhaustive]
#[derive(Debug)]
/// An error that can occur when registering a shortcut.
pub enum ShortcutRegistryError {
    /// The shortcut is invalid.
    InvalidShortcut(String),
    /// An unknown error occurred.
    Other(Box<dyn std::error::Error>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// An global id for a shortcut.
pub struct ShortcutId {
    id: AcceleratorId,
    number: usize,
}

/// A global shortcut. This will be automatically removed when it is dropped.
pub struct ShortcutHandle {
    desktop: DesktopContext,
    /// The id of the shortcut
    pub shortcut_id: ShortcutId,
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_global_shortcut(
    cx: &ScopeState,
    key: impl IntoKeyCode,
    modifiers: impl IntoModifersState,
    handler: impl FnMut() + 'static,
) -> &Result<ShortcutHandle, ShortcutRegistryError> {
    let desktop = use_window(cx);
    cx.use_hook(move || {
        let desktop = desktop.clone();

        let id = desktop.create_shortcut(
            key.into_key_code(),
            modifiers.into_modifiers_state(),
            handler,
        );

        Ok(ShortcutHandle {
            desktop,
            shortcut_id: id?,
        })
    })
}

impl ShortcutHandle {
    /// Remove the shortcut.
    pub fn remove(&self) {
        self.desktop.remove_shortcut(self.shortcut_id);
    }
}

impl Drop for ShortcutHandle {
    fn drop(&mut self) {
        self.remove()
    }
}

pub trait IntoModifersState {
    fn into_modifiers_state(self) -> ModifiersState;
}

impl IntoModifersState for ModifiersState {
    fn into_modifiers_state(self) -> ModifiersState {
        self
    }
}

impl IntoModifersState for Modifiers {
    fn into_modifiers_state(self) -> ModifiersState {
        let mut state = ModifiersState::empty();
        if self.contains(Modifiers::SHIFT) {
            state |= ModifiersState::SHIFT
        }
        if self.contains(Modifiers::CONTROL) {
            state |= ModifiersState::CONTROL
        }
        if self.contains(Modifiers::ALT) {
            state |= ModifiersState::ALT
        }
        if self.contains(Modifiers::META) || self.contains(Modifiers::SUPER) {
            state |= ModifiersState::SUPER
        }
        state
    }
}

pub trait IntoKeyCode {
    fn into_key_code(self) -> KeyCode;
}

impl IntoKeyCode for KeyCode {
    fn into_key_code(self) -> KeyCode {
        self
    }
}

impl IntoKeyCode for dioxus_html::KeyCode {
    fn into_key_code(self) -> KeyCode {
        match self {
            dioxus_html::KeyCode::Backspace => KeyCode::Backspace,
            dioxus_html::KeyCode::Tab => KeyCode::Tab,
            dioxus_html::KeyCode::Clear => KeyCode::NumpadClear,
            dioxus_html::KeyCode::Enter => KeyCode::Enter,
            dioxus_html::KeyCode::Shift => KeyCode::ShiftLeft,
            dioxus_html::KeyCode::Ctrl => KeyCode::ControlLeft,
            dioxus_html::KeyCode::Alt => KeyCode::AltLeft,
            dioxus_html::KeyCode::Pause => KeyCode::Pause,
            dioxus_html::KeyCode::CapsLock => KeyCode::CapsLock,
            dioxus_html::KeyCode::Escape => KeyCode::Escape,
            dioxus_html::KeyCode::Space => KeyCode::Space,
            dioxus_html::KeyCode::PageUp => KeyCode::PageUp,
            dioxus_html::KeyCode::PageDown => KeyCode::PageDown,
            dioxus_html::KeyCode::End => KeyCode::End,
            dioxus_html::KeyCode::Home => KeyCode::Home,
            dioxus_html::KeyCode::LeftArrow => KeyCode::ArrowLeft,
            dioxus_html::KeyCode::UpArrow => KeyCode::ArrowUp,
            dioxus_html::KeyCode::RightArrow => KeyCode::ArrowRight,
            dioxus_html::KeyCode::DownArrow => KeyCode::ArrowDown,
            dioxus_html::KeyCode::Insert => KeyCode::Insert,
            dioxus_html::KeyCode::Delete => KeyCode::Delete,
            dioxus_html::KeyCode::Num0 => KeyCode::Numpad0,
            dioxus_html::KeyCode::Num1 => KeyCode::Numpad1,
            dioxus_html::KeyCode::Num2 => KeyCode::Numpad2,
            dioxus_html::KeyCode::Num3 => KeyCode::Numpad3,
            dioxus_html::KeyCode::Num4 => KeyCode::Numpad4,
            dioxus_html::KeyCode::Num5 => KeyCode::Numpad5,
            dioxus_html::KeyCode::Num6 => KeyCode::Numpad6,
            dioxus_html::KeyCode::Num7 => KeyCode::Numpad7,
            dioxus_html::KeyCode::Num8 => KeyCode::Numpad8,
            dioxus_html::KeyCode::Num9 => KeyCode::Numpad9,
            dioxus_html::KeyCode::A => KeyCode::KeyA,
            dioxus_html::KeyCode::B => KeyCode::KeyB,
            dioxus_html::KeyCode::C => KeyCode::KeyC,
            dioxus_html::KeyCode::D => KeyCode::KeyD,
            dioxus_html::KeyCode::E => KeyCode::KeyE,
            dioxus_html::KeyCode::F => KeyCode::KeyF,
            dioxus_html::KeyCode::G => KeyCode::KeyG,
            dioxus_html::KeyCode::H => KeyCode::KeyH,
            dioxus_html::KeyCode::I => KeyCode::KeyI,
            dioxus_html::KeyCode::J => KeyCode::KeyJ,
            dioxus_html::KeyCode::K => KeyCode::KeyK,
            dioxus_html::KeyCode::L => KeyCode::KeyL,
            dioxus_html::KeyCode::M => KeyCode::KeyM,
            dioxus_html::KeyCode::N => KeyCode::KeyN,
            dioxus_html::KeyCode::O => KeyCode::KeyO,
            dioxus_html::KeyCode::P => KeyCode::KeyP,
            dioxus_html::KeyCode::Q => KeyCode::KeyQ,
            dioxus_html::KeyCode::R => KeyCode::KeyR,
            dioxus_html::KeyCode::S => KeyCode::KeyS,
            dioxus_html::KeyCode::T => KeyCode::KeyT,
            dioxus_html::KeyCode::U => KeyCode::KeyU,
            dioxus_html::KeyCode::V => KeyCode::KeyV,
            dioxus_html::KeyCode::W => KeyCode::KeyW,
            dioxus_html::KeyCode::X => KeyCode::KeyX,
            dioxus_html::KeyCode::Y => KeyCode::KeyY,
            dioxus_html::KeyCode::Z => KeyCode::KeyZ,
            dioxus_html::KeyCode::Numpad0 => KeyCode::Numpad0,
            dioxus_html::KeyCode::Numpad1 => KeyCode::Numpad1,
            dioxus_html::KeyCode::Numpad2 => KeyCode::Numpad2,
            dioxus_html::KeyCode::Numpad3 => KeyCode::Numpad3,
            dioxus_html::KeyCode::Numpad4 => KeyCode::Numpad4,
            dioxus_html::KeyCode::Numpad5 => KeyCode::Numpad5,
            dioxus_html::KeyCode::Numpad6 => KeyCode::Numpad6,
            dioxus_html::KeyCode::Numpad7 => KeyCode::Numpad7,
            dioxus_html::KeyCode::Numpad8 => KeyCode::Numpad8,
            dioxus_html::KeyCode::Numpad9 => KeyCode::Numpad9,
            dioxus_html::KeyCode::Multiply => KeyCode::NumpadMultiply,
            dioxus_html::KeyCode::Add => KeyCode::NumpadAdd,
            dioxus_html::KeyCode::Subtract => KeyCode::NumpadSubtract,
            dioxus_html::KeyCode::DecimalPoint => KeyCode::NumpadDecimal,
            dioxus_html::KeyCode::Divide => KeyCode::NumpadDivide,
            dioxus_html::KeyCode::F1 => KeyCode::F1,
            dioxus_html::KeyCode::F2 => KeyCode::F2,
            dioxus_html::KeyCode::F3 => KeyCode::F3,
            dioxus_html::KeyCode::F4 => KeyCode::F4,
            dioxus_html::KeyCode::F5 => KeyCode::F5,
            dioxus_html::KeyCode::F6 => KeyCode::F6,
            dioxus_html::KeyCode::F7 => KeyCode::F7,
            dioxus_html::KeyCode::F8 => KeyCode::F8,
            dioxus_html::KeyCode::F9 => KeyCode::F9,
            dioxus_html::KeyCode::F10 => KeyCode::F10,
            dioxus_html::KeyCode::F11 => KeyCode::F11,
            dioxus_html::KeyCode::F12 => KeyCode::F12,
            dioxus_html::KeyCode::NumLock => KeyCode::NumLock,
            dioxus_html::KeyCode::ScrollLock => KeyCode::ScrollLock,
            dioxus_html::KeyCode::Semicolon => KeyCode::Semicolon,
            dioxus_html::KeyCode::EqualSign => KeyCode::Equal,
            dioxus_html::KeyCode::Comma => KeyCode::Comma,
            dioxus_html::KeyCode::Period => KeyCode::Period,
            dioxus_html::KeyCode::ForwardSlash => KeyCode::Slash,
            dioxus_html::KeyCode::GraveAccent => KeyCode::Backquote,
            dioxus_html::KeyCode::OpenBracket => KeyCode::BracketLeft,
            dioxus_html::KeyCode::BackSlash => KeyCode::Backslash,
            dioxus_html::KeyCode::CloseBraket => KeyCode::BracketRight,
            dioxus_html::KeyCode::SingleQuote => KeyCode::Quote,
            key => panic!("Failed to convert {:?} to tao::keyboard::KeyCode, try using tao::keyboard::KeyCode directly", key),
        }
    }
}
