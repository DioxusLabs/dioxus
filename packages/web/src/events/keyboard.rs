use std::str::FromStr;

use dioxus_html::{
    input_data::decode_key_location, Code, HasKeyboardData, Key, Location, Modifiers,
    ModifiersInteraction,
};
use web_sys::KeyboardEvent;

use super::{Synthetic, WebEventExt};

impl HasKeyboardData for Synthetic<KeyboardEvent> {
    fn key(&self) -> Key {
        // Handle undefined key values from browser autofill
        let key_str = wasm_bindgen::JsValue::from(self.event.key())
            .as_string()
            .unwrap_or_else(|| "Unidentified".to_string());
        Key::from_str(&key_str).unwrap_or(Key::Unidentified)
    }

    fn code(&self) -> Code {
        // Handle undefined code values from browser autofill
        let code_str = wasm_bindgen::JsValue::from(self.event.code())
            .as_string()
            .unwrap_or_else(|| "Unidentified".to_string());
        Code::from_str(&code_str).unwrap_or(Code::Unidentified)
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

        // Handle undefined modifier key values from browser autofill
        // Convert JsValue to bool, defaulting to false if undefined
        if wasm_bindgen::JsValue::from(self.event.alt_key())
            .as_bool()
            .unwrap_or(false)
        {
            modifiers.insert(Modifiers::ALT);
        }
        if wasm_bindgen::JsValue::from(self.event.ctrl_key())
            .as_bool()
            .unwrap_or(false)
        {
            modifiers.insert(Modifiers::CONTROL);
        }
        if wasm_bindgen::JsValue::from(self.event.meta_key())
            .as_bool()
            .unwrap_or(false)
        {
            modifiers.insert(Modifiers::META);
        }
        if wasm_bindgen::JsValue::from(self.event.shift_key())
            .as_bool()
            .unwrap_or(false)
        {
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
