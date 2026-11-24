use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};

/// A keyboard shortcut: a key pressed with exactly the given modifiers.
///
/// The key is identified by its physical position rather than the character
/// it produces, so shortcuts work regardless of keyboard layout and of the
/// characters modifiers produce (e.g. Option+I types "ˆ" on macOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyboardShortcut {
    pub modifiers: ModifiersState,
    pub key: KeyCode,
}

impl KeyboardShortcut {
    pub const fn new(modifiers: ModifiersState, key: KeyCode) -> Self {
        Self { modifiers, key }
    }

    /// Chrome's shortcuts for opening devtools on the current platform: F12,
    /// and Cmd+Option+I on macOS or Ctrl+Shift+I elsewhere
    pub fn chrome_devtools() -> [KeyboardShortcut; 2] {
        let modifiers = if cfg!(target_os = "macos") {
            ModifiersState::META | ModifiersState::ALT
        } else {
            ModifiersState::CONTROL | ModifiersState::SHIFT
        };
        [
            Self::new(ModifiersState::empty(), KeyCode::F12),
            Self::new(modifiers, KeyCode::KeyI),
        ]
    }

    /// Whether pressing `key` while `modifiers` are held triggers this shortcut
    pub fn matches(&self, modifiers: ModifiersState, key: PhysicalKey) -> bool {
        modifiers == self.modifiers && key == PhysicalKey::Code(self.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_exact_modifiers_and_physical_key() {
        let shortcut = KeyboardShortcut::new(
            ModifiersState::CONTROL | ModifiersState::SHIFT,
            KeyCode::KeyI,
        );
        let key = PhysicalKey::Code(KeyCode::KeyI);
        assert!(shortcut.matches(ModifiersState::CONTROL | ModifiersState::SHIFT, key));
        // Missing or extra modifiers don't match
        assert!(!shortcut.matches(ModifiersState::CONTROL, key));
        assert!(!shortcut.matches(
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT,
            key
        ));
        // Nor does another key
        assert!(!shortcut.matches(
            ModifiersState::CONTROL | ModifiersState::SHIFT,
            PhysicalKey::Code(KeyCode::KeyJ)
        ));
    }

    #[test]
    fn chrome_devtools_shortcuts() {
        let [f12, inspect] = KeyboardShortcut::chrome_devtools();
        assert!(f12.matches(ModifiersState::empty(), PhysicalKey::Code(KeyCode::F12)));
        let modifiers = if cfg!(target_os = "macos") {
            ModifiersState::META | ModifiersState::ALT
        } else {
            ModifiersState::CONTROL | ModifiersState::SHIFT
        };
        assert!(inspect.matches(modifiers, PhysicalKey::Code(KeyCode::KeyI)));
    }
}
