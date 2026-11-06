use std::sync::{PoisonError, RwLock};

/// Check if the event bubbles
///
/// The real solution to this problem is that events need to mark themselves as "bubbling" or "not bubbling"
/// in their definition, which gets passed as part of the mutations.
pub fn event_bubbles(evt: &str) -> bool {
    match evt {
        "cancel" => false,
        "copy" => true,
        "cut" => true,
        "paste" => true,
        "compositionend" => true,
        "compositionstart" => true,
        "compositionupdate" => true,
        "keydown" => true,
        "keypress" => true,
        "keyup" => true,
        "focus" => false,
        "focusout" => true,
        "focusin" => true,
        "blur" => false,
        "change" => true,
        "input" => true,
        "beforeinput" => true,
        "invalid" => true,
        "reset" => true,
        "submit" => true,
        "auxclick" => true,
        "click" => true,
        "contextmenu" => true,
        "doubleclick" => true,
        "dblclick" => true,
        "drag" => true,
        "dragend" => true,
        "dragenter" => false,
        "dragexit" => false,
        "dragleave" => true,
        "dragover" => true,
        "dragstart" => true,
        "drop" => true,
        "mousedown" => true,
        "mouseenter" => false,
        "mouseleave" => false,
        "mousemove" => true,
        "mouseout" => true,
        "scroll" => false,
        "scrollend" => false,
        "mouseover" => true,
        "mouseup" => true,
        "pointerdown" => true,
        "pointermove" => true,
        "pointerup" => true,
        "pointercancel" => true,
        "gotpointercapture" => true,
        "lostpointercapture" => true,
        "pointerenter" => false,
        "pointerleave" => false,
        "pointerover" => true,
        "pointerout" => true,
        "select" => true,
        "touchcancel" => true,
        "touchend" => true,
        "touchmove" => true,
        "touchstart" => true,
        "wheel" => true,
        "abort" => false,
        "canplay" => false,
        "canplaythrough" => false,
        "durationchange" => false,
        "emptied" => false,
        "encrypted" => true,
        "ended" => false,
        "error" => false,
        "loadeddata" => false,
        "loadedmetadata" => false,
        "loadstart" => false,
        "load" => false,
        "pause" => false,
        "play" => false,
        "playing" => false,
        "progress" => false,
        "ratechange" => false,
        "resize" => false,
        "seeked" => false,
        "seeking" => false,
        "stalled" => false,
        "suspend" => false,
        "timeupdate" => false,
        "volumechange" => false,
        "waiting" => false,
        "animationstart" => true,
        "animationend" => true,
        "animationiteration" => true,
        "transitionend" => true,
        "toggle" => false,
        "beforetoggle" => false,
        "mounted" => false,
        "visible" => false,
        _ => custom_event_bubbles(evt).unwrap_or(true),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RegisteredEvent {
    name: &'static str,
    bubbles: bool,
}

static CUSTOM_EVENTS: RwLock<Vec<RegisteredEvent>> = RwLock::new(Vec::new());

/// Register a custom event by the name it is dispatched with in the page, and whether it bubbles.
///
/// Registering the same name again with the same bubbling does nothing. Registering it again with
/// different bubbling panics, because listeners already attached for it would disagree with the new
/// ones about where they listen.
pub(crate) fn register_custom_event(name: &'static str, bubbles: bool) {
    let mut events = CUSTOM_EVENTS
        .write()
        .unwrap_or_else(PoisonError::into_inner);
    match events.iter().find(|event| event.name == name) {
        Some(event) if event.bubbles == bubbles => {}
        Some(event) => panic!(
            "The custom event \"{name}\" was registered with bubbles: {}, so it cannot be registered again with bubbles: {bubbles}.",
            event.bubbles
        ),
        None => events.push(RegisteredEvent { name, bubbles }),
    }
}

/// Whether a custom event registered by this name bubbles, or nothing if no custom event is registered by it.
pub(crate) fn custom_event_bubbles(name: &str) -> Option<bool> {
    CUSTOM_EVENTS
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|event| event.name == name)
        .map(|event| event.bubbles)
}

/// Whether a custom event is registered by this name.
pub(crate) fn is_custom_event(name: &str) -> bool {
    custom_event_bubbles(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unregistered_event_bubbles_as_before() {
        assert!(event_bubbles("an-event-nobody-registered"));
        assert!(!is_custom_event("an-event-nobody-registered"));
        assert_eq!(custom_event_bubbles("an-event-nobody-registered"), None);
    }

    #[test]
    fn a_registered_custom_event_bubbles_as_it_was_registered() {
        register_custom_event("custom-test-quiet", false);
        register_custom_event("custom-test-loud", true);
        assert!(!event_bubbles("custom-test-quiet"));
        assert!(event_bubbles("custom-test-loud"));
        assert!(is_custom_event("custom-test-quiet"));
        assert_eq!(custom_event_bubbles("custom-test-loud"), Some(true));
    }

    #[test]
    fn registering_the_same_event_again_the_same_way_changes_nothing() {
        register_custom_event("custom-test-twice", false);
        register_custom_event("custom-test-twice", false);
        assert_eq!(custom_event_bubbles("custom-test-twice"), Some(false));
    }

    #[test]
    #[should_panic(expected = "cannot be registered again with bubbles: true")]
    fn registering_the_same_event_again_another_way_panics() {
        register_custom_event("custom-test-changed", false);
        register_custom_event("custom-test-changed", true);
    }

    #[test]
    fn a_built_in_event_keeps_its_bubbling_whatever_is_registered() {
        assert!(!event_bubbles("focus"));
        assert!(event_bubbles("click"));
    }
}
