//! Ported events into Dioxus Synthetic Event system
//!
//! event porting is pretty boring, sorry.

use dioxus_core::events::on::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, UiEvent};

/// All events implement the generic event type - they're all UI events
trait WebsysGenericEvent {
    fn as_ui_event(&self) -> &UiEvent;
}

impl GenericEventInner for &dyn WebsysGenericEvent {
    /// On WebSys, this returns an &UiEvent which can be casted via dyn_ref into the correct sub type.
    fn raw_event(&self) -> &dyn std::any::Any {
        self.as_ui_event()
    }

    fn bubbles(&self) -> bool {
        self.as_ui_event().bubbles()
    }

    fn cancel_bubble(&self) {
        self.as_ui_event().cancel_bubble();
    }

    fn cancelable(&self) -> bool {
        self.as_ui_event().cancelable()
    }

    fn composed(&self) -> bool {
        self.as_ui_event().composed()
    }

    fn current_target(&self) {
        if cfg!(debug_assertions) {
            todo!("Current target does not return anything useful.\nPlease try casting the event directly.");
        }
        // self.as_ui_event().current_target();
    }

    fn default_prevented(&self) -> bool {
        self.as_ui_event().default_prevented()
    }

    fn event_phase(&self) -> u16 {
        self.as_ui_event().event_phase()
    }

    fn is_trusted(&self) -> bool {
        self.as_ui_event().is_trusted()
    }

    fn prevent_default(&self) {
        self.as_ui_event().prevent_default()
    }

    fn stop_immediate_propagation(&self) {
        self.as_ui_event().stop_immediate_propagation()
    }

    fn stop_propagation(&self) {
        self.as_ui_event().stop_propagation()
    }

    fn target(&self) {
        todo!()
    }

    fn time_stamp(&self) -> f64 {
        self.as_ui_event().time_stamp()
    }
}

macro_rules! implement_generic_event {
    (
        $($event:ident),*
    ) => {
        $(
            impl WebsysGenericEvent for $event {
                fn as_ui_event(&self) -> &UiEvent {
                    self.0.dyn_ref().unwrap()
                }
            }
        )*
    };
}

implement_generic_event! {
    WebsysClipboardEvent,
    WebsysCompositionEvent,
    WebsysKeyboardEvent,
    WebsysGenericUiEvent,
    WebsysFocusEvent,
    WebsysFormEvent,
    WebsysMouseEvent,
    WebsysPointerEvent,
    WebsysWheelEvent,
    WebsysAnimationEvent,
    WebsysTransitionEvent,
    WebsysTouchEvent,
    WebsysMediaEvent,
    WebsysToggleEvent
}

// unfortunately, currently experimental, and web_sys needs to be configured to use it :>(
pub struct WebsysClipboardEvent(pub Event);

impl ClipboardEventInner for WebsysClipboardEvent {}

pub struct WebsysCompositionEvent(pub web_sys::CompositionEvent);

impl CompositionEventInner for WebsysCompositionEvent {
    fn data(&self) -> String {
        self.0.data().unwrap_or_else(|| String::new())
    }
}

pub struct WebsysKeyboardEvent(pub web_sys::KeyboardEvent);
impl KeyboardEventInner for WebsysKeyboardEvent {
    fn alt_key(&self) -> bool {
        self.0.alt_key()
    }
    fn char_code(&self) -> u32 {
        self.0.char_code()
    }
    fn key(&self) -> String {
        self.0.key()
    }

    fn key_code(&self) -> KeyCode {
        KeyCode::from_raw_code(self.0.key_code() as u8)
    }

    fn ctrl_key(&self) -> bool {
        self.0.ctrl_key()
    }

    fn get_modifier_state(&self, key_code: &str) -> bool {
        self.0.get_modifier_state(key_code)
    }

    fn locale(&self) -> String {
        if cfg!(debug_assertions) {
            todo!("Locale is currently not supported. :(")
        } else {
            String::from("en-US")
        }
    }

    fn location(&self) -> usize {
        self.0.location() as usize
    }

    fn meta_key(&self) -> bool {
        self.0.meta_key()
    }

    fn repeat(&self) -> bool {
        self.0.repeat()
    }

    fn shift_key(&self) -> bool {
        self.0.shift_key()
    }

    fn which(&self) -> usize {
        self.0.which() as usize
    }
}

pub struct WebsysGenericUiEvent(pub UiEvent);
impl GenericEventInner for WebsysGenericUiEvent {
    fn raw_event(&self) -> &dyn std::any::Any {
        // self.0.raw_event()
        todo!()
    }

    fn bubbles(&self) -> bool {
        self.0.bubbles()
    }

    fn cancel_bubble(&self) {
        self.0.cancel_bubble();
    }

    fn cancelable(&self) -> bool {
        self.0.cancelable()
    }

    fn composed(&self) -> bool {
        self.0.composed()
    }

    fn current_target(&self) {
        // self.0.current_target()
    }

    fn default_prevented(&self) -> bool {
        self.0.default_prevented()
    }

    fn event_phase(&self) -> u16 {
        self.0.event_phase()
    }

    fn is_trusted(&self) -> bool {
        self.0.is_trusted()
    }

    fn prevent_default(&self) {
        self.0.prevent_default()
    }

    fn stop_immediate_propagation(&self) {
        self.0.stop_immediate_propagation()
    }

    fn stop_propagation(&self) {
        self.0.stop_propagation()
    }

    fn target(&self) {
        // self.0.target()
    }

    fn time_stamp(&self) -> f64 {
        self.0.time_stamp()
    }
}

impl UIEventInner for WebsysGenericUiEvent {
    fn detail(&self) -> i32 {
        todo!()
    }
}

impl SelectionEventInner for WebsysGenericUiEvent {}

pub struct WebsysFocusEvent(pub web_sys::FocusEvent);
impl FocusEventInner for WebsysFocusEvent {}

pub struct WebsysFormEvent(pub web_sys::Event);
impl FormEventInner for WebsysFormEvent {
    // technically a controlled component, so we need to manually grab out the target data
    fn value(&self) -> String {
        let this: web_sys::EventTarget = self.0.target().unwrap();
        (&this)
                .dyn_ref()
                .map(|input: &web_sys::HtmlInputElement| input.value())
                .or_else(|| {
                    this
                        .dyn_ref()
                        .map(|input: &web_sys::HtmlTextAreaElement| input.value())
                })
                // select elements are NOT input events - because - why woudn't they be??
                .or_else(|| {
                    this
                        .dyn_ref()
                        .map(|input: &web_sys::HtmlSelectElement| input.value())
                })
                .or_else(|| {
                    this
                        .dyn_ref::<web_sys::HtmlElement>()
                        .unwrap()
                        .text_content()
                })
                .expect("only an InputElement or TextAreaElement or an element with contenteditable=true can have an oninput event listener")
    }
}

pub struct WebsysMouseEvent(pub web_sys::MouseEvent);
impl MouseEventInner for WebsysMouseEvent {
    fn alt_key(&self) -> bool {
        self.0.alt_key()
    }
    fn button(&self) -> i16 {
        self.0.button()
    }
    fn buttons(&self) -> u16 {
        self.0.buttons()
    }
    fn client_x(&self) -> i32 {
        self.0.client_x()
    }
    fn client_y(&self) -> i32 {
        self.0.client_y()
    }
    fn ctrl_key(&self) -> bool {
        self.0.ctrl_key()
    }
    fn meta_key(&self) -> bool {
        self.0.meta_key()
    }
    fn page_x(&self) -> i32 {
        self.0.page_x()
    }
    fn page_y(&self) -> i32 {
        self.0.page_y()
    }
    fn screen_x(&self) -> i32 {
        self.0.screen_x()
    }
    fn screen_y(&self) -> i32 {
        self.0.screen_y()
    }
    fn shift_key(&self) -> bool {
        self.0.shift_key()
    }

    // yikes
    // https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key/Key_Values
    fn get_modifier_state(&self, key_code: &str) -> bool {
        self.0.get_modifier_state(key_code)
    }
}

pub struct WebsysPointerEvent(pub web_sys::PointerEvent);
impl PointerEventInner for WebsysPointerEvent {
    fn alt_key(&self) -> bool {
        self.0.alt_key()
    }
    fn button(&self) -> i16 {
        self.0.button()
    }
    fn buttons(&self) -> u16 {
        self.0.buttons()
    }
    fn client_x(&self) -> i32 {
        self.0.client_x()
    }
    fn client_y(&self) -> i32 {
        self.0.client_y()
    }
    fn ctrl_key(&self) -> bool {
        self.0.ctrl_key()
    }
    fn meta_key(&self) -> bool {
        self.0.meta_key()
    }
    fn page_x(&self) -> i32 {
        self.0.page_x()
    }
    fn page_y(&self) -> i32 {
        self.0.page_y()
    }
    fn screen_x(&self) -> i32 {
        self.0.screen_x()
    }
    fn screen_y(&self) -> i32 {
        self.0.screen_y()
    }
    fn shift_key(&self) -> bool {
        self.0.shift_key()
    }

    // yikes
    // https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key/Key_Values
    fn get_modifier_state(&self, key_code: &str) -> bool {
        self.0.get_modifier_state(key_code)
    }

    fn pointer_id(&self) -> i32 {
        self.0.pointer_id()
    }

    fn width(&self) -> i32 {
        self.0.width()
    }

    fn height(&self) -> i32 {
        self.0.height()
    }

    fn pressure(&self) -> f32 {
        self.0.pressure()
    }

    fn tangential_pressure(&self) -> f32 {
        self.0.tangential_pressure()
    }

    fn tilt_x(&self) -> i32 {
        self.0.tilt_x()
    }

    fn tilt_y(&self) -> i32 {
        self.0.tilt_y()
    }

    fn twist(&self) -> i32 {
        self.0.twist()
    }

    fn pointer_type(&self) -> String {
        self.0.pointer_type()
    }

    fn is_primary(&self) -> bool {
        self.0.is_primary()
    }
}

pub struct WebsysWheelEvent(pub web_sys::WheelEvent);
impl WheelEventInner for WebsysWheelEvent {
    fn delta_mode(&self) -> u32 {
        self.0.delta_mode()
    }

    fn delta_x(&self) -> f64 {
        self.0.delta_x()
    }

    fn delta_y(&self) -> f64 {
        self.0.delta_y()
    }

    fn delta_z(&self) -> f64 {
        self.0.delta_z()
    }
}
pub struct WebsysAnimationEvent(pub web_sys::AnimationEvent);
impl AnimationEventInner for WebsysAnimationEvent {
    fn animation_name(&self) -> String {
        self.0.animation_name()
    }

    fn pseudo_element(&self) -> String {
        self.0.pseudo_element()
    }

    fn elapsed_time(&self) -> f32 {
        self.0.elapsed_time()
    }
}

pub struct WebsysTransitionEvent(pub web_sys::TransitionEvent);
impl TransitionEventInner for WebsysTransitionEvent {
    fn property_name(&self) -> String {
        self.0.property_name()
    }

    fn pseudo_element(&self) -> String {
        self.0.pseudo_element()
    }

    fn elapsed_time(&self) -> f32 {
        self.0.elapsed_time()
    }
}

pub struct WebsysTouchEvent(pub web_sys::TouchEvent);
impl TouchEventInner for WebsysTouchEvent {
    fn alt_key(&self) -> bool {
        self.0.alt_key()
    }

    fn ctrl_key(&self) -> bool {
        self.0.ctrl_key()
    }

    fn meta_key(&self) -> bool {
        self.0.meta_key()
    }

    fn shift_key(&self) -> bool {
        self.0.shift_key()
    }

    fn get_modifier_state(&self, key_code: &str) -> bool {
        if cfg!(debug_assertions) {
            todo!("get_modifier_state is not currently supported for touch events");
        } else {
            false
        }
    }
}

pub struct WebsysMediaEvent(pub web_sys::UiEvent);
impl MediaEventInner for WebsysMediaEvent {}

pub struct WebsysToggleEvent(pub web_sys::UiEvent);
impl ToggleEventInner for WebsysToggleEvent {}
