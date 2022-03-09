use crossterm::event::{
    Event as TermEvent, KeyCode as TermKeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use dioxus_core::*;

use dioxus_html::{on::*, KeyCode};
use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};
use stretch2::{prelude::Layout, Stretch};

use crate::TuiNode;

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
    mouse: Option<(MouseData, Vec<u16>)>,
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
            // limitations: only two buttons may be held at once
            EventData::Mouse(ref mut m) => match &mut self.mouse {
                Some(state) => {
                    let mut buttons = state.0.buttons;
                    state.0 = clone_mouse_data(m);
                    match evt.0 {
                        // this code only runs when there are no buttons down
                        "mouseup" => {
                            buttons = 0;
                            state.1 = Vec::new();
                        }
                        "mousedown" => {
                            if state.1.contains(&m.buttons) {
                                // if we already pressed a button and there is another button released the button crossterm sends is the button remaining
                                if state.1.len() > 1 {
                                    evt.0 = "mouseup";
                                    state.1 = vec![m.buttons];
                                }
                                // otherwise some other button was pressed. In testing it was consistantly this mapping
                                else {
                                    match m.buttons {
                                        0x01 => state.1.push(0x02),
                                        0x02 => state.1.push(0x01),
                                        0x04 => state.1.push(0x01),
                                        _ => (),
                                    }
                                }
                            } else {
                                state.1.push(m.buttons);
                            }

                            buttons = state.1.iter().copied().reduce(|a, b| a | b).unwrap();
                        }
                        _ => (),
                    }
                    state.0.buttons = buttons;
                    m.buttons = buttons;
                }
                None => {
                    self.mouse = Some((
                        clone_mouse_data(m),
                        if m.buttons == 0 {
                            Vec::new()
                        } else {
                            vec![m.buttons]
                        },
                    ));
                }
            },
            EventData::Wheel(ref w) => self.wheel = Some(clone_wheel_data(w)),
            EventData::Screen(ref s) => self.screen = Some(*s),
            EventData::Keyboard(ref mut k) => {
                let repeat = self
                    .last_key_pressed
                    .as_ref()
                    .filter(|k2| k2.0.key == k.key && k2.1.elapsed() < MAX_REPEAT_TIME)
                    .is_some();
                k.repeat = repeat;
                let new = clone_keyboard_data(k);
                self.last_key_pressed = Some((new, Instant::now()));
            }
        }
    }

    fn update<'a>(
        &mut self,
        dom: &'a VirtualDom,
        evts: &mut Vec<EventCore>,
        resolved_events: &mut Vec<UserEvent>,
        layout: &Stretch,
        layouts: &mut HashMap<ElementId, TuiNode<'a>>,
        node: &'a VNode<'a>,
    ) {
        struct Data<'b> {
            new_pos: (i32, i32),
            old_pos: Option<(i32, i32)>,
            clicked: bool,
            released: bool,
            wheel_delta: f64,
            mouse_data: &'b MouseData,
            wheel_data: &'b Option<WheelData>,
        }

        fn layout_contains_point(layout: &Layout, point: (i32, i32)) -> bool {
            layout.location.x as i32 <= point.0
                && layout.location.x as i32 + layout.size.width as i32 >= point.0
                && layout.location.y as i32 <= point.1
                && layout.location.y as i32 + layout.size.height as i32 >= point.1
        }

        fn get_mouse_events<'c, 'd>(
            dom: &'c VirtualDom,
            resolved_events: &mut Vec<UserEvent>,
            layout: &Stretch,
            layouts: &HashMap<ElementId, TuiNode<'c>>,
            node: &'c VNode<'c>,
            data: &'d Data<'d>,
        ) -> HashSet<&'static str> {
            match node {
                VNode::Fragment(f) => {
                    let mut union = HashSet::new();
                    for child in f.children {
                        union = union
                            .union(&get_mouse_events(
                                dom,
                                resolved_events,
                                layout,
                                layouts,
                                child,
                                data,
                            ))
                            .copied()
                            .collect();
                    }
                    return union;
                }

                VNode::Component(vcomp) => {
                    let idx = vcomp.scope.get().unwrap();
                    let new_node = dom.get_scope(idx).unwrap().root_node();
                    return get_mouse_events(dom, resolved_events, layout, layouts, new_node, data);
                }

                VNode::Placeholder(_) => return HashSet::new(),

                VNode::Element(_) | VNode::Text(_) => {}
            }

            let id = node.try_mounted_id().unwrap();
            let node = layouts.get(&id).unwrap();

            let node_layout = layout.layout(node.layout).unwrap();

            let previously_contained = data
                .old_pos
                .filter(|pos| layout_contains_point(node_layout, *pos))
                .is_some();
            let currently_contains = layout_contains_point(node_layout, data.new_pos);

            match node.node {
                VNode::Element(el) => {
                    let mut events = HashSet::new();
                    if previously_contained || currently_contains {
                        for c in el.children {
                            events = events
                                .union(&get_mouse_events(
                                    dom,
                                    resolved_events,
                                    layout,
                                    layouts,
                                    c,
                                    data,
                                ))
                                .copied()
                                .collect();
                        }
                    }
                    let mut try_create_event = |name| {
                        // only trigger event if the event was not triggered already by a child
                        if events.insert(name) {
                            resolved_events.push(UserEvent {
                                scope_id: None,
                                priority: EventPriority::Medium,
                                name,
                                element: Some(el.id.get().unwrap()),
                                data: Arc::new(clone_mouse_data(data.mouse_data)),
                            })
                        }
                    };
                    if currently_contains {
                        if !previously_contained {
                            try_create_event("mouseenter");
                            try_create_event("mouseover");
                        }
                        if data.clicked {
                            try_create_event("mousedown");
                        }
                        if data.released {
                            try_create_event("mouseup");
                            match data.mouse_data.button {
                                0 => try_create_event("click"),
                                2 => try_create_event("contextmenu"),
                                _ => (),
                            }
                        }
                        if let Some(w) = data.wheel_data {
                            if data.wheel_delta != 0.0 {
                                resolved_events.push(UserEvent {
                                    scope_id: None,
                                    priority: EventPriority::Medium,
                                    name: "wheel",
                                    element: Some(el.id.get().unwrap()),
                                    data: Arc::new(clone_wheel_data(w)),
                                })
                            }
                        }
                    } else if previously_contained {
                        try_create_event("mouseleave");
                        try_create_event("mouseout");
                    }
                    events
                }
                VNode::Text(_) => HashSet::new(),
                _ => todo!(),
            }
        }

        let previous_mouse = self
            .mouse
            .as_ref()
            .map(|m| (clone_mouse_data(&m.0), m.1.clone()));
        // println!("{previous_mouse:?}");

        self.wheel = None;

        for e in evts.iter_mut() {
            self.apply_event(e);
        }

        // resolve hover events
        if let Some(mouse) = &self.mouse {
            let new_pos = (mouse.0.screen_x, mouse.0.screen_y);
            let old_pos = previous_mouse
                .as_ref()
                .map(|m| (m.0.screen_x, m.0.screen_y));
            let clicked =
                (!mouse.0.buttons & previous_mouse.as_ref().map(|m| m.0.buttons).unwrap_or(0)) > 0;
            let released =
                (mouse.0.buttons & !previous_mouse.map(|m| m.0.buttons).unwrap_or(0)) > 0;
            let wheel_delta = self.wheel.as_ref().map_or(0.0, |w| w.delta_y);
            let mouse_data = &mouse.0;
            let wheel_data = &self.wheel;
            let data = Data {
                new_pos,
                old_pos,
                clicked,
                released,
                wheel_delta,
                mouse_data,
                wheel_data,
            };
            get_mouse_events(dom, resolved_events, layout, layouts, node, &data);
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
    /// limitations: GUI key modifier is never detected, key up events are not detected, and only two mouse buttons may be pressed at once
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

    pub fn get_events<'a>(
        &self,
        dom: &'a VirtualDom,
        layout: &Stretch,
        layouts: &mut HashMap<ElementId, TuiNode<'a>>,
        node: &'a VNode<'a>,
    ) -> Vec<UserEvent> {
        // todo: currently resolves events in all nodes, but once the focus system is added it should filter by focus
        fn inner(
            queue: &[(&'static str, Arc<dyn Any + Send + Sync>)],
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

        (*self.state).borrow_mut().update(
            dom,
            &mut (*self.queued_events).borrow_mut(),
            &mut resolved_events,
            layout,
            layouts,
            node,
        );

        let events: Vec<_> = self
            .queued_events
            .replace(Vec::new())
            .into_iter()
            // these events were added in the update stage
            .filter(|e| !["mousedown", "mouseup", "mousemove", "drag", "wheel"].contains(&e.0))
            .map(|e| (e.0, e.1.into_any()))
            .collect();

        inner(&events, &mut resolved_events, node);

        resolved_events
    }
}

// translate crossterm events into dioxus events
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
                MouseEventKind::ScrollDown => ("wheel", get_wheel_data(false)),
                MouseEventKind::ScrollUp => ("wheel", get_wheel_data(true)),
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
