//! Convert a serialized event to an event Trigger
//!

use std::rc::Rc;

use dioxus_core::{
    events::{
        on::{MouseEvent, MouseEventInner},
        SyntheticEvent,
    },
    ElementId, EventPriority, EventTrigger, ScopeId,
};

#[derive(serde::Serialize, serde::Deserialize)]
struct ImEvent {
    event: String,
    mounted_dom_id: u64,
    scope: u64,
}
pub fn trigger_from_serialized(val: serde_json::Value) -> EventTrigger {
    let mut data: Vec<ImEvent> = serde_json::from_value(val).unwrap();
    let data = data.drain(..).next().unwrap();

    let event = SyntheticEvent::MouseEvent(MouseEvent(Rc::new(WebviewMouseEvent)));
    let scope = ScopeId(data.scope as usize);
    let mounted_dom_id = Some(ElementId(data.mounted_dom_id as usize));
    let priority = EventPriority::High;
    EventTrigger::new(event, scope, mounted_dom_id, priority)
}

#[derive(Debug)]
struct WebviewMouseEvent;
impl MouseEventInner for WebviewMouseEvent {
    fn alt_key(&self) -> bool {
        todo!()
    }

    fn button(&self) -> i16 {
        todo!()
    }

    fn buttons(&self) -> u16 {
        todo!()
    }

    fn client_x(&self) -> i32 {
        todo!()
    }

    fn client_y(&self) -> i32 {
        todo!()
    }

    fn ctrl_key(&self) -> bool {
        todo!()
    }

    fn meta_key(&self) -> bool {
        todo!()
    }

    fn page_x(&self) -> i32 {
        todo!()
    }

    fn page_y(&self) -> i32 {
        todo!()
    }

    fn screen_x(&self) -> i32 {
        todo!()
    }

    fn screen_y(&self) -> i32 {
        todo!()
    }

    fn shift_key(&self) -> bool {
        todo!()
    }

    fn get_modifier_state(&self, key_code: &str) -> bool {
        todo!()
    }
}
