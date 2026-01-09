use std::str::FromStr;

use dioxus_html::{
    input_data::decode_key_location, Code, HasKeyboardData, Key, Location, Modifiers,
    ModifiersInteraction,
};
use web_sys::KeyboardEvent;

use super::{Synthetic, WebEventExt};

impl HasKeyboardData for Synthetic<KeyboardEvent> {
    fn key(&self) -> Key {
        Key::from_str(self.event.key().as_str()).unwrap_or(Key::Unidentified)
    }

    fn code(&self) -> Code {
        Code::from_str(self.event.code().as_str()).unwrap_or(Code::Unidentified)
    }

    fn location(&self) -> Location {
        decode_key_location(self.event.location() as usize)
    }

    fn is_auto_repeating(&self) -> bool {
        self.event.repeat()
    }

    fn is_composing(&self) -> bool {
        self.event.is_composing()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl ModifiersInteraction for Synthetic<KeyboardEvent> {
    fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();

        if self.event.alt_key() {
            modifiers.insert(Modifiers::ALT);
        }
        if self.event.ctrl_key() {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.event.meta_key() {
            modifiers.insert(Modifiers::META);
        }
        if self.event.shift_key() {
            modifiers.insert(Modifiers::SHIFT);
        }

        modifiers
    }
}

impl WebEventExt for dioxus_html::KeyboardData {
    type WebEvent = web_sys::KeyboardEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::KeyboardEvent> {
        self.downcast::<web_sys::KeyboardEvent>().cloned()
    }
}
