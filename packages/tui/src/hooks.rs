use crossterm::event::{
    Event as TermEvent, KeyCode as TermKeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use dioxus_core::*;

use dioxus_html::{on::*, KeyCode};
use dioxus_native_core::{Tree, TreeNode};
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

use crate::{style_attributes::StyleModifier, RinkLayout};

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
        evts: &mut Vec<EventCore>,
        resolved_events: &mut Vec<UserEvent>,
        layout: &Stretch,
        tree: &mut Tree<RinkLayout, StyleModifier>,
    ) {
        let previous_mouse = self
            .mouse
            .as_ref()
            .map(|m| (clone_mouse_data(&m.0), m.1.clone()));

        self.wheel = None;

        for e in evts.iter_mut() {
            self.apply_event(e);
        }

        self.resolve_mouse_events(previous_mouse, resolved_events, layout, tree);

        // for s in &self.subscribers {
        //     s();
        // }
    }

    fn resolve_mouse_events(
        &self,
        previous_mouse: Option<(MouseData, Vec<u16>)>,
        resolved_events: &mut Vec<UserEvent>,
        layout: &Stretch,
        tree: &mut Tree<RinkLayout, StyleModifier>,
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

        fn try_create_event(
            name: &'static str,
            data: Arc<dyn Any + Send + Sync>,
            will_bubble: &mut HashSet<ElementId>,
            resolved_events: &mut Vec<UserEvent>,
            node: &TreeNode<RinkLayout, StyleModifier>,
            tree: &Tree<RinkLayout, StyleModifier>,
        ) {
            // only trigger event if the event was not triggered already by a child
            if will_bubble.insert(node.id) {
                let mut parent = node.parent;
                while let Some(parent_id) = parent {
                    will_bubble.insert(parent_id);
                    parent = tree.get(parent_id.0).parent;
                }
                resolved_events.push(UserEvent {
                    scope_id: None,
                    priority: EventPriority::Medium,
                    name,
                    element: Some(node.id),
                    data: data,
                })
            }
        }

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

            {
                // mousemove
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("mousemove") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let previously_contained = data
                        .old_pos
                        .filter(|pos| layout_contains_point(node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if currently_contains {
                        if previously_contained {
                            try_create_event(
                                "mousemove",
                                Arc::new(clone_mouse_data(data.mouse_data)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                tree,
                            );
                        }
                    }
                }
            }

            {
                // mouseenter
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("mouseenter") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let previously_contained = data
                        .old_pos
                        .filter(|pos| layout_contains_point(node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if currently_contains {
                        if !previously_contained {
                            try_create_event(
                                "mouseenter",
                                Arc::new(clone_mouse_data(data.mouse_data)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                tree,
                            );
                        }
                    }
                }
            }

            {
                // mouseover
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("mouseover") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let previously_contained = data
                        .old_pos
                        .filter(|pos| layout_contains_point(node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if currently_contains {
                        if !previously_contained {
                            try_create_event(
                                "mouseover",
                                Arc::new(clone_mouse_data(data.mouse_data)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                tree,
                            );
                        }
                    }
                }
            }

            {
                // mousedown
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("mousedown") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if currently_contains {
                        if data.clicked {
                            try_create_event(
                                "mousedown",
                                Arc::new(clone_mouse_data(data.mouse_data)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                tree,
                            );
                        }
                    }
                }
            }

            {
                // mouseup
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("mouseup") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if currently_contains {
                        if data.released {
                            try_create_event(
                                "mouseup",
                                Arc::new(clone_mouse_data(data.mouse_data)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                tree,
                            );
                        }
                    }
                }
            }

            {
                // click
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("click") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if currently_contains {
                        if data.released && data.mouse_data.button == 0 {
                            try_create_event(
                                "click",
                                Arc::new(clone_mouse_data(data.mouse_data)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                tree,
                            );
                        }
                    }
                }
            }

            {
                // contextmenu
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("contextmenu") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if currently_contains {
                        if data.released && data.mouse_data.button == 2 {
                            try_create_event(
                                "contextmenu",
                                Arc::new(clone_mouse_data(data.mouse_data)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                tree,
                            );
                        }
                    }
                }
            }

            {
                // wheel
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("wheel") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if currently_contains {
                        if let Some(w) = data.wheel_data {
                            if data.wheel_delta != 0.0 {
                                try_create_event(
                                    "wheel",
                                    Arc::new(clone_wheel_data(w)),
                                    &mut will_bubble,
                                    resolved_events,
                                    node,
                                    tree,
                                );
                            }
                        }
                    }
                }
            }

            {
                // mouseleave
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("mouseleave") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let previously_contained = data
                        .old_pos
                        .filter(|pos| layout_contains_point(node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if !currently_contains && previously_contained {
                        try_create_event(
                            "mouseleave",
                            Arc::new(clone_mouse_data(data.mouse_data)),
                            &mut will_bubble,
                            resolved_events,
                            node,
                            tree,
                        );
                    }
                }
            }

            {
                // mouseout
                let mut will_bubble = HashSet::new();
                for node in tree.get_listening_sorted("mouseout") {
                    let node_layout = layout.layout(node.up_state.node.unwrap()).unwrap();
                    let previously_contained = data
                        .old_pos
                        .filter(|pos| layout_contains_point(node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(node_layout, data.new_pos);

                    if !currently_contains && previously_contained {
                        try_create_event(
                            "mouseout",
                            Arc::new(clone_mouse_data(data.mouse_data)),
                            &mut will_bubble,
                            resolved_events,
                            node,
                            tree,
                        );
                    }
                }
            }
        }
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
        let queued_events2 = Rc::downgrade(&queued_events);

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
        layout: &Stretch,
        tree: &mut Tree<RinkLayout, StyleModifier>,
    ) -> Vec<UserEvent> {
        let mut resolved_events = Vec::new();

        (*self.state).borrow_mut().update(
            &mut (*self.queued_events).borrow_mut(),
            &mut resolved_events,
            layout,
            tree,
        );

        let events = self
            .queued_events
            .replace(Vec::new())
            .into_iter()
            // these events were added in the update stage
            .filter(|e| {
                ![
                    "mouseenter",
                    "mouseover",
                    "mouseleave",
                    "mouseout",
                    "mousedown",
                    "mouseup",
                    "mousemove",
                    "drag",
                    "wheel",
                    "click",
                    "contextmenu",
                ]
                .contains(&e.0)
            })
            .map(|evt| (evt.0, evt.1.into_any()));

        // todo: currently resolves events in all nodes, but once the focus system is added it should filter by focus
        let mut hm: HashMap<&'static str, Vec<Arc<dyn Any + Send + Sync>>> = HashMap::new();
        for (event, data) in events {
            if let Some(v) = hm.get_mut(event) {
                v.push(data);
            } else {
                hm.insert(event, vec![data]);
            }
        }
        for (event, datas) in hm {
            for node in tree.get_listening_sorted(event) {
                for data in &datas {
                    resolved_events.push(UserEvent {
                        scope_id: None,
                        priority: EventPriority::Medium,
                        name: event,
                        element: Some(node.id),
                        data: data.clone(),
                    });
                }
            }
        }

        resolved_events
    }
}

// translate crossterm events into dioxus events
fn get_event(evt: TermEvent) -> Option<(&'static str, EventData)> {
    let (name, data): (&str, EventData) = match evt {
        TermEvent::Key(k) => ("keydown", translate_key_event(k)?),
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

fn translate_key_event(event: crossterm::event::KeyEvent) -> Option<EventData> {
    let (code, key_str);
    if let TermKeyCode::Char(c) = event.code {
        code = match c {
            'A'..='Z' | 'a'..='z' => match c.to_ascii_uppercase() {
                'A' => KeyCode::A,
                'B' => KeyCode::B,
                'C' => KeyCode::C,
                'D' => KeyCode::D,
                'E' => KeyCode::E,
                'F' => KeyCode::F,
                'G' => KeyCode::G,
                'H' => KeyCode::H,
                'I' => KeyCode::I,
                'J' => KeyCode::J,
                'K' => KeyCode::K,
                'L' => KeyCode::L,
                'M' => KeyCode::M,
                'N' => KeyCode::N,
                'O' => KeyCode::O,
                'P' => KeyCode::P,
                'Q' => KeyCode::Q,
                'R' => KeyCode::R,
                'S' => KeyCode::S,
                'T' => KeyCode::T,
                'U' => KeyCode::U,
                'V' => KeyCode::V,
                'W' => KeyCode::W,
                'X' => KeyCode::X,
                'Y' => KeyCode::Y,
                'Z' => KeyCode::Z,
                _ => return None,
            },
            ' ' => KeyCode::Space,
            '[' => KeyCode::OpenBracket,
            '{' => KeyCode::OpenBracket,
            ']' => KeyCode::CloseBraket,
            '}' => KeyCode::CloseBraket,
            ';' => KeyCode::Semicolon,
            ':' => KeyCode::Semicolon,
            ',' => KeyCode::Comma,
            '<' => KeyCode::Comma,
            '.' => KeyCode::Period,
            '>' => KeyCode::Period,
            '1' => KeyCode::Num1,
            '2' => KeyCode::Num2,
            '3' => KeyCode::Num3,
            '4' => KeyCode::Num4,
            '5' => KeyCode::Num5,
            '6' => KeyCode::Num6,
            '7' => KeyCode::Num7,
            '8' => KeyCode::Num8,
            '9' => KeyCode::Num9,
            '0' => KeyCode::Num0,
            '!' => KeyCode::Num1,
            '@' => KeyCode::Num2,
            '#' => KeyCode::Num3,
            '$' => KeyCode::Num4,
            '%' => KeyCode::Num5,
            '^' => KeyCode::Num6,
            '&' => KeyCode::Num7,
            '*' => KeyCode::Num8,
            '(' => KeyCode::Num9,
            ')' => KeyCode::Num0,
            // numpad charicter are ambiguous to tui
            // '*' => KeyCode::Multiply,
            // '/' => KeyCode::Divide,
            // '-' => KeyCode::Subtract,
            // '+' => KeyCode::Add,
            '+' => KeyCode::EqualSign,
            '-' => KeyCode::Dash,
            '_' => KeyCode::Dash,
            '\'' => KeyCode::SingleQuote,
            '"' => KeyCode::SingleQuote,
            '\\' => KeyCode::BackSlash,
            '|' => KeyCode::BackSlash,
            '/' => KeyCode::ForwardSlash,
            '?' => KeyCode::ForwardSlash,
            '=' => KeyCode::EqualSign,
            '`' => KeyCode::GraveAccent,
            '~' => KeyCode::GraveAccent,
            _ => return None,
        };
        key_str = c.to_string();
    } else {
        code = match event.code {
            TermKeyCode::Esc => KeyCode::Escape,
            TermKeyCode::Backspace => KeyCode::Backspace,
            TermKeyCode::Enter => KeyCode::Enter,
            TermKeyCode::Left => KeyCode::LeftArrow,
            TermKeyCode::Right => KeyCode::RightArrow,
            TermKeyCode::Up => KeyCode::UpArrow,
            TermKeyCode::Down => KeyCode::DownArrow,
            TermKeyCode::Home => KeyCode::Home,
            TermKeyCode::End => KeyCode::End,
            TermKeyCode::PageUp => KeyCode::PageUp,
            TermKeyCode::PageDown => KeyCode::PageDown,
            TermKeyCode::Tab => KeyCode::Tab,
            TermKeyCode::Delete => KeyCode::Delete,
            TermKeyCode::Insert => KeyCode::Insert,
            TermKeyCode::F(fn_num) => match fn_num {
                1 => KeyCode::F1,
                2 => KeyCode::F2,
                3 => KeyCode::F3,
                4 => KeyCode::F4,
                5 => KeyCode::F5,
                6 => KeyCode::F6,
                7 => KeyCode::F7,
                8 => KeyCode::F8,
                9 => KeyCode::F9,
                10 => KeyCode::F10,
                11 => KeyCode::F11,
                12 => KeyCode::F12,
                _ => return None,
            },
            TermKeyCode::BackTab => return None,
            TermKeyCode::Null => return None,
            _ => return None,
        };
        key_str = if let KeyCode::BackSlash = code {
            "\\".to_string()
        } else {
            format!("{code:?}")
        }
    };
    // from https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent
    Some(EventData::Keyboard(KeyboardData {
        char_code: code.raw_code(),
        key: key_str.to_string(),
        key_code: code,
        alt_key: event.modifiers.contains(KeyModifiers::ALT),
        ctrl_key: event.modifiers.contains(KeyModifiers::CONTROL),
        meta_key: false,
        shift_key: event.modifiers.contains(KeyModifiers::SHIFT),
        locale: Default::default(),
        location: 0x00,
        repeat: Default::default(),
        which: Default::default(),
    }))
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
