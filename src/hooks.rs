use crossterm::event::{
    Event as TermEvent, KeyCode as TermKeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use dioxus::core::*;

use dioxus_html::{on::*, KeyCode};
use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
use std::{
    any::Any,
    borrow::BorrowMut,
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

// a wrapper around the input state for easier access
// todo: fix loop
// pub struct InputState(Rc<Rc<RefCell<InnerInputState>>>);
// impl InputState {
//     pub fn get(cx: &ScopeState) -> InputState {
//         let inner = cx
//             .consume_context::<Rc<RefCell<InnerInputState>>>()
//             .expect("Rink InputState can only be used in Rink apps!");
//         (**inner).borrow_mut().subscribe(cx.schedule_update());
//         InputState(inner)
//     }

//     pub fn mouse(&self) -> Option<MouseData> {
//         let data = (**self.0).borrow();
//         data.mouse.as_ref().map(|m| clone_mouse_data(m))
//     }

//     pub fn wheel(&self) -> Option<WheelData> {
//         let data = (**self.0).borrow();
//         data.wheel.as_ref().map(|w| clone_wheel_data(w))
//     }

//     pub fn screen(&self) -> Option<(u16, u16)> {
//         let data = (**self.0).borrow();
//         data.screen.as_ref().map(|m| m.clone())
//     }

//     pub fn last_key_pressed(&self) -> Option<KeyboardData> {
//         let data = (**self.0).borrow();
//         data.last_key_pressed
//             .as_ref()
//             .map(|k| clone_keyboard_data(&k.0))
//     }
// }

type EventCore = (&'static str, EventData);

#[derive(Debug)]
enum EventData {
    Mouse(MouseData),
    Wheel(WheelData),
    Screen((u16, u16)),
    Keyboard(KeyboardData),
}
impl EventData {
    fn into_any(self) -> Arc<dyn Any + Send + Sync> {
        match self {
            Self::Mouse(m) => Arc::new(m),
            Self::Wheel(w) => Arc::new(w),
            Self::Screen(s) => Arc::new(s),
            Self::Keyboard(k) => Arc::new(k),
        }
    }
}

const MAX_REPEAT_TIME: Duration = Duration::from_millis(100);

pub struct InnerInputState {
    mouse: Option<MouseData>,
    wheel: Option<WheelData>,
    last_key_pressed: Option<(KeyboardData, Instant)>,
    screen: Option<(u16, u16)>,
    // subscribers: Vec<Rc<dyn Fn() + 'static>>,
}

impl InnerInputState {
    fn new() -> Self {
        Self {
            mouse: None,
            wheel: None,
            last_key_pressed: None,
            screen: None,
            // subscribers: Vec::new(),
        }
    }

    // stores current input state and transforms events based on that state
    fn apply_event(&mut self, evt: &mut EventCore) {
        match evt.1 {
            EventData::Mouse(ref mut m) => match &mut self.mouse {
                Some(state) => {
                    *state = clone_mouse_data(m);
                    // crossterm always outputs the left mouse button on mouse up
                    // let mut buttons = state.buttons;
                    // *state = clone_mouse_data(m);
                    // match evt.0 {
                    //     "mouseup" => {
                    //         buttons &= !m.buttons;
                    //     }
                    //     "mousedown" => {
                    //         buttons |= m.buttons;
                    //     }
                    //     _ => (),
                    // }
                    // state.buttons = buttons;
                    // m.buttons = buttons;
                }
                None => {
                    self.mouse = Some(clone_mouse_data(m));
                }
            },
            EventData::Wheel(ref w) => self.wheel = Some(clone_wheel_data(w)),
            EventData::Screen(ref s) => self.screen = Some(s.clone()),
            EventData::Keyboard(ref mut k) => {
                let repeat = self
                    .last_key_pressed
                    .as_ref()
                    .filter(|k2| k2.0.key == k.key && k2.1.elapsed() < MAX_REPEAT_TIME)
                    .is_some();
                k.repeat = repeat;
                let mut new = clone_keyboard_data(k);
                new.repeat = repeat;
                self.last_key_pressed = Some((new, Instant::now()));
            }
        }
    }

    fn update(&mut self, evts: &mut [EventCore]) {
        for e in evts {
            self.apply_event(e)
        }
        // for s in &self.subscribers {
        //     s();
        // }
    }

    // fn subscribe(&mut self, f: Rc<dyn Fn() + 'static>) {
    //     self.subscribers.push(f)
    // }
}

pub struct RinkInputHandler {
    state: Rc<RefCell<InnerInputState>>,
    queued_events: Rc<RefCell<Vec<EventCore>>>,
}

impl RinkInputHandler {
    /// global context that handles events
    /// limitations: GUI key modifier is never detected, key up events are not detected, and mouse up events are not specific to a key
    pub fn new(
        mut receiver: UnboundedReceiver<TermEvent>,
        cx: &ScopeState,
    ) -> (Self, Rc<RefCell<InnerInputState>>) {
        let queued_events = Rc::new(RefCell::new(Vec::new()));
        let queued_events2 = Rc::<RefCell<std::vec::Vec<_>>>::downgrade(&queued_events);

        cx.push_future(async move {
            while let Some(evt) = receiver.next().await {
                if let Some(evt) = get_event(evt) {
                    if let Some(v) = queued_events2.upgrade() {
                        (*v).borrow_mut().push(evt);
                    } else {
                        break;
                    }
                }
            }
        });

        let state = Rc::new(RefCell::new(InnerInputState::new()));

        (
            Self {
                state: state.clone(),
                queued_events,
            },
            state,
        )
    }

    pub fn resolve_events(&self, dom: &mut VirtualDom) {
        // todo: currently resolves events in all nodes, but once the focus system is added it should filter by focus
        fn inner(
            queue: &Vec<(&'static str, Arc<dyn Any + Send + Sync>)>,
            resolved: &mut Vec<UserEvent>,
            node: &VNode,
        ) {
            match node {
                VNode::Fragment(frag) => {
                    for c in frag.children {
                        inner(queue, resolved, c);
                    }
                }
                VNode::Element(el) => {
                    for l in el.listeners {
                        for (name, data) in queue.iter() {
                            if *name == l.event {
                                if let Some(id) = el.id.get() {
                                    resolved.push(UserEvent {
                                        scope_id: None,
                                        priority: EventPriority::Medium,
                                        name: *name,
                                        element: Some(id),
                                        data: data.clone(),
                                    });
                                }
                            }
                        }
                    }
                    for c in el.children {
                        inner(queue, resolved, c);
                    }
                }
                _ => (),
            }
        }

        let mut resolved_events = Vec::new();

        (*self.state)
            .borrow_mut()
            .update(&mut (*self.queued_events).borrow_mut());

        let events: Vec<_> = self
            .queued_events
            .replace(Vec::new())
            .into_iter()
            .map(|e| (e.0, e.1.into_any()))
            .collect();

        inner(&events, &mut resolved_events, dom.base_scope().root_node());

        for e in resolved_events {
            dom.handle_message(SchedulerMsg::Event(e));
        }
    }
}

fn get_event(evt: TermEvent) -> Option<(&'static str, EventData)> {
    let (name, data): (&str, EventData) = match evt {
        TermEvent::Key(k) => {
            let key = translate_key_code(k.code)?;
            (
                "keydown",
                // from https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent
                EventData::Keyboard(KeyboardData {
                    char_code: key.raw_code(),
                    key: format!("{key:?}"),
                    key_code: key,
                    alt_key: k.modifiers.contains(KeyModifiers::ALT),
                    ctrl_key: k.modifiers.contains(KeyModifiers::CONTROL),
                    meta_key: false,
                    shift_key: k.modifiers.contains(KeyModifiers::SHIFT),
                    locale: Default::default(),
                    location: 0x00,
                    repeat: Default::default(),
                    which: Default::default(),
                }),
            )
        }
        TermEvent::Mouse(m) => {
            let (x, y) = (m.column.into(), m.row.into());
            let alt = m.modifiers.contains(KeyModifiers::ALT);
            let shift = m.modifiers.contains(KeyModifiers::SHIFT);
            let ctrl = m.modifiers.contains(KeyModifiers::CONTROL);
            let meta = false;

            let get_mouse_data = |b| {
                let buttons = match b {
                    None => 0,
                    Some(MouseButton::Left) => 1,
                    Some(MouseButton::Right) => 2,
                    Some(MouseButton::Middle) => 4,
                };
                let button_state = match b {
                    None => 0,
                    Some(MouseButton::Left) => 0,
                    Some(MouseButton::Middle) => 1,
                    Some(MouseButton::Right) => 2,
                };
                // from https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent
                EventData::Mouse(MouseData {
                    alt_key: alt,
                    button: button_state,
                    buttons,
                    client_x: x,
                    client_y: y,
                    ctrl_key: ctrl,
                    meta_key: meta,
                    page_x: x,
                    page_y: y,
                    screen_x: x,
                    screen_y: y,
                    shift_key: shift,
                })
            };

            let get_wheel_data = |up| {
                // from https://developer.mozilla.org/en-US/docs/Web/API/WheelEvent
                EventData::Wheel(WheelData {
                    delta_mode: 0x01,
                    delta_x: 0.0,
                    delta_y: if up { -1.0 } else { 1.0 },
                    delta_z: 0.0,
                })
            };

            match m.kind {
                MouseEventKind::Down(b) => ("mousedown", get_mouse_data(Some(b))),
                MouseEventKind::Up(b) => ("mouseup", get_mouse_data(Some(b))),
                MouseEventKind::Drag(b) => ("drag", get_mouse_data(Some(b))),
                MouseEventKind::Moved => ("mousemove", get_mouse_data(None)),
                MouseEventKind::ScrollDown => ("scroll", get_wheel_data(false)),
                MouseEventKind::ScrollUp => ("scroll", get_wheel_data(true)),
            }
        }
        TermEvent::Resize(x, y) => ("resize", EventData::Screen((x, y))),
    };

    Some((name, data))
}

fn translate_key_code(c: TermKeyCode) -> Option<KeyCode> {
    match c {
        TermKeyCode::Backspace => Some(KeyCode::Backspace),
        TermKeyCode::Enter => Some(KeyCode::Enter),
        TermKeyCode::Left => Some(KeyCode::LeftArrow),
        TermKeyCode::Right => Some(KeyCode::RightArrow),
        TermKeyCode::Up => Some(KeyCode::UpArrow),
        TermKeyCode::Down => Some(KeyCode::DownArrow),
        TermKeyCode::Home => Some(KeyCode::Home),
        TermKeyCode::End => Some(KeyCode::End),
        TermKeyCode::PageUp => Some(KeyCode::PageUp),
        TermKeyCode::PageDown => Some(KeyCode::PageDown),
        TermKeyCode::Tab => Some(KeyCode::Tab),
        TermKeyCode::BackTab => None,
        TermKeyCode::Delete => Some(KeyCode::Delete),
        TermKeyCode::Insert => Some(KeyCode::Insert),
        TermKeyCode::F(fn_num) => match fn_num {
            1 => Some(KeyCode::F1),
            2 => Some(KeyCode::F2),
            3 => Some(KeyCode::F3),
            4 => Some(KeyCode::F4),
            5 => Some(KeyCode::F5),
            6 => Some(KeyCode::F6),
            7 => Some(KeyCode::F7),
            8 => Some(KeyCode::F8),
            9 => Some(KeyCode::F9),
            10 => Some(KeyCode::F10),
            11 => Some(KeyCode::F11),
            12 => Some(KeyCode::F12),
            _ => None,
        },
        TermKeyCode::Char(c) => match c.to_uppercase().next().unwrap() {
            'A' => Some(KeyCode::A),
            'B' => Some(KeyCode::B),
            'C' => Some(KeyCode::C),
            'D' => Some(KeyCode::D),
            'E' => Some(KeyCode::E),
            'F' => Some(KeyCode::F),
            'G' => Some(KeyCode::G),
            'H' => Some(KeyCode::H),
            'I' => Some(KeyCode::I),
            'J' => Some(KeyCode::J),
            'K' => Some(KeyCode::K),
            'L' => Some(KeyCode::L),
            'M' => Some(KeyCode::M),
            'N' => Some(KeyCode::N),
            'O' => Some(KeyCode::O),
            'P' => Some(KeyCode::P),
            'Q' => Some(KeyCode::Q),
            'R' => Some(KeyCode::R),
            'S' => Some(KeyCode::S),
            'T' => Some(KeyCode::T),
            'U' => Some(KeyCode::U),
            'V' => Some(KeyCode::V),
            'W' => Some(KeyCode::W),
            'X' => Some(KeyCode::X),
            'Y' => Some(KeyCode::Y),
            'Z' => Some(KeyCode::Z),
            _ => None,
        },
        TermKeyCode::Null => None,
        TermKeyCode::Esc => Some(KeyCode::Escape),
    }
}

fn clone_mouse_data(m: &MouseData) -> MouseData {
    MouseData {
        client_x: m.client_x,
        client_y: m.client_y,
        page_x: m.page_x,
        page_y: m.page_y,
        screen_x: m.screen_x,
        screen_y: m.screen_y,
        alt_key: m.alt_key,
        ctrl_key: m.ctrl_key,
        meta_key: m.meta_key,
        shift_key: m.shift_key,
        button: m.button,
        buttons: m.buttons,
    }
}

fn clone_keyboard_data(k: &KeyboardData) -> KeyboardData {
    KeyboardData {
        char_code: k.char_code,
        key: k.key.clone(),
        key_code: k.key_code,
        alt_key: k.alt_key,
        ctrl_key: k.ctrl_key,
        meta_key: k.meta_key,
        shift_key: k.shift_key,
        locale: k.locale.clone(),
        location: k.location,
        repeat: k.repeat,
        which: k.which,
    }
}

fn clone_wheel_data(w: &WheelData) -> WheelData {
    WheelData {
        delta_mode: w.delta_mode,
        delta_x: w.delta_x,
        delta_y: w.delta_y,
        delta_z: w.delta_x,
    }
}
