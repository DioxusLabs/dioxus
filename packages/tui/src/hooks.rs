use crossterm::event::{
    Event as TermEvent, KeyCode as TermKeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use dioxus_core::*;
use fxhash::{FxHashMap, FxHashSet};

use dioxus_html::geometry::euclid::{Point2D, Rect, Size2D};
use dioxus_html::geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint};
use dioxus_html::input_data::keyboard_types::Modifiers;
use dioxus_html::input_data::MouseButtonSet as DioxusMouseButtons;
use dioxus_html::input_data::{MouseButton as DioxusMouseButton, MouseButtonSet};
use dioxus_html::{on::*, KeyCode};
use std::{
    any::Any,
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};
use stretch2::geometry::{Point, Size};
use stretch2::{prelude::Layout, Stretch};

use crate::{Dom, Node};

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
//         data.mouse.as_ref().map(|m| m.clone())
//     }

//     pub fn wheel(&self) -> Option<WheelData> {
//         let data = (**self.0).borrow();
//         data.wheel.as_ref().map(|w| w.clone())
//     }

//     pub fn screen(&self) -> Option<(u16, u16)> {
//         let data = (**self.0).borrow();
//         data.screen.as_ref().map(|m| m.clone())
//     }

//     pub fn last_key_pressed(&self) -> Option<KeyboardData> {
//         let data = (**self.0).borrow();
//         data.last_key_pressed
//             .as_ref()
//             .map(|k| &k.0.clone())
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
            // limitations: only two buttons may be held at once
            EventData::Mouse(ref mut m) => {
                let mut held_buttons = match &self.mouse {
                    Some(previous_data) => previous_data.held_buttons(),
                    None => MouseButtonSet::empty(),
                };

                match evt.0 {
                    "mousedown" => {
                        held_buttons.insert(
                            m.trigger_button()
                                .expect("No trigger button for mousedown event"),
                        );
                    }
                    "mouseup" => {
                        held_buttons.remove(
                            m.trigger_button()
                                .expect("No trigger button for mouseup event"),
                        );
                    }
                    _ => {}
                }

                let new_mouse_data = MouseData::new(
                    m.coordinates(),
                    m.trigger_button(),
                    held_buttons,
                    m.modifiers(),
                );

                self.mouse = Some(new_mouse_data.clone());
                *m = new_mouse_data;
            }
            EventData::Wheel(ref w) => self.wheel = Some(w.clone()),
            EventData::Screen(ref s) => self.screen = Some(*s),
            EventData::Keyboard(ref mut k) => {
                let repeat = self
                    .last_key_pressed
                    .as_ref()
                    .filter(|k2| k2.0.key == k.key && k2.1.elapsed() < MAX_REPEAT_TIME)
                    .is_some();
                k.repeat = repeat;
                let new = k.clone();
                self.last_key_pressed = Some((new, Instant::now()));
            }
        }
    }

    fn update(
        &mut self,
        evts: &mut [EventCore],
        resolved_events: &mut Vec<UserEvent>,
        layout: &Stretch,
        dom: &mut Dom,
    ) {
        let previous_mouse = self.mouse.clone();

        self.wheel = None;

        for e in evts.iter_mut() {
            self.apply_event(e);
        }

        self.resolve_mouse_events(previous_mouse, resolved_events, layout, dom);

        // for s in &self.subscribers {
        //     s();
        // }
    }

    fn resolve_mouse_events(
        &self,
        previous_mouse: Option<MouseData>,
        resolved_events: &mut Vec<UserEvent>,
        layout: &Stretch,
        dom: &mut Dom,
    ) {
        fn layout_contains_point(layout: &Layout, point: ScreenPoint) -> bool {
            let Point { x, y } = layout.location;
            let Size { width, height } = layout.size;

            let layout_rect = Rect::new(Point2D::new(x, y), Size2D::new(width, height));
            layout_rect.contains(point.cast())
        }

        fn try_create_event(
            name: &'static str,
            data: Arc<dyn Any + Send + Sync>,
            will_bubble: &mut FxHashSet<ElementId>,
            resolved_events: &mut Vec<UserEvent>,
            node: &Node,
            dom: &Dom,
        ) {
            // only trigger event if the event was not triggered already by a child
            if will_bubble.insert(node.id) {
                let mut parent = node.parent;
                while let Some(parent_id) = parent {
                    will_bubble.insert(parent_id);
                    parent = dom[parent_id.0].parent;
                }
                resolved_events.push(UserEvent {
                    scope_id: None,
                    priority: EventPriority::Medium,
                    name,
                    element: Some(node.id),
                    data,
                })
            }
        }

        fn prepare_mouse_data(mouse_data: &MouseData, layout: &Layout) -> MouseData {
            let Point { x, y } = layout.location;
            let node_origin = ClientPoint::new(x.into(), y.into());

            let new_client_coordinates = (mouse_data.client_coordinates() - node_origin)
                .to_point()
                .cast_unit();

            let coordinates = Coordinates::new(
                mouse_data.screen_coordinates(),
                mouse_data.client_coordinates(),
                new_client_coordinates,
                mouse_data.page_coordinates(),
            );

            MouseData::new(
                coordinates,
                mouse_data.trigger_button(),
                mouse_data.held_buttons(),
                mouse_data.modifiers(),
            )
        }

        if let Some(mouse_data) = &self.mouse {
            let new_pos = mouse_data.screen_coordinates();
            let old_pos = previous_mouse.as_ref().map(|m| m.screen_coordinates());

            // a mouse button is pressed if a button was not down and is now down
            let previous_buttons = previous_mouse
                .map_or(MouseButtonSet::empty(), |previous_data| {
                    previous_data.held_buttons()
                });
            let was_pressed = !(mouse_data.held_buttons() - previous_buttons).is_empty();

            // a mouse button is released if a button was down and is now not down
            let was_released = !(previous_buttons - mouse_data.held_buttons()).is_empty();

            let wheel_delta = self.wheel.as_ref().map_or(0.0, |w| w.delta_y);
            let wheel_data = &self.wheel;

            {
                // mousemove
                if old_pos != Some(new_pos) {
                    let mut will_bubble = FxHashSet::default();
                    for node in dom.get_listening_sorted("mousemove") {
                        let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                        let previously_contained = old_pos
                            .filter(|pos| layout_contains_point(node_layout, *pos))
                            .is_some();
                        let currently_contains = layout_contains_point(node_layout, new_pos);

                        if currently_contains && previously_contained {
                            try_create_event(
                                "mousemove",
                                Arc::new(prepare_mouse_data(mouse_data, node_layout)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                dom,
                            );
                        }
                    }
                }
            }

            {
                // mouseenter
                let mut will_bubble = FxHashSet::default();
                for node in dom.get_listening_sorted("mouseenter") {
                    let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                    let previously_contained = old_pos
                        .filter(|pos| layout_contains_point(node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(node_layout, new_pos);

                    if currently_contains && !previously_contained {
                        try_create_event(
                            "mouseenter",
                            Arc::new(mouse_data.clone()),
                            &mut will_bubble,
                            resolved_events,
                            node,
                            dom,
                        );
                    }
                }
            }

            {
                // mouseover
                let mut will_bubble = FxHashSet::default();
                for node in dom.get_listening_sorted("mouseover") {
                    let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                    let previously_contained = old_pos
                        .filter(|pos| layout_contains_point(node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(node_layout, new_pos);

                    if currently_contains && !previously_contained {
                        try_create_event(
                            "mouseover",
                            Arc::new(prepare_mouse_data(mouse_data, node_layout)),
                            &mut will_bubble,
                            resolved_events,
                            node,
                            dom,
                        );
                    }
                }
            }

            // mousedown
            if was_pressed {
                let mut will_bubble = FxHashSet::default();
                for node in dom.get_listening_sorted("mousedown") {
                    let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                    let currently_contains = layout_contains_point(node_layout, new_pos);

                    if currently_contains {
                        try_create_event(
                            "mousedown",
                            Arc::new(prepare_mouse_data(mouse_data, node_layout)),
                            &mut will_bubble,
                            resolved_events,
                            node,
                            dom,
                        );
                    }
                }
            }

            {
                // mouseup
                if was_released {
                    let mut will_bubble = FxHashSet::default();
                    for node in dom.get_listening_sorted("mouseup") {
                        let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                        let currently_contains = layout_contains_point(node_layout, new_pos);

                        if currently_contains {
                            try_create_event(
                                "mouseup",
                                Arc::new(prepare_mouse_data(mouse_data, node_layout)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                dom,
                            );
                        }
                    }
                }
            }

            {
                // click
                if mouse_data.trigger_button() == Some(DioxusMouseButton::Primary) && was_released {
                    let mut will_bubble = FxHashSet::default();
                    for node in dom.get_listening_sorted("click") {
                        let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                        let currently_contains = layout_contains_point(node_layout, new_pos);

                        if currently_contains {
                            try_create_event(
                                "click",
                                Arc::new(prepare_mouse_data(mouse_data, node_layout)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                dom,
                            );
                        }
                    }
                }
            }

            {
                // contextmenu
                if mouse_data.trigger_button() == Some(DioxusMouseButton::Secondary) && was_released
                {
                    let mut will_bubble = FxHashSet::default();
                    for node in dom.get_listening_sorted("contextmenu") {
                        let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                        let currently_contains = layout_contains_point(node_layout, new_pos);

                        if currently_contains {
                            try_create_event(
                                "contextmenu",
                                Arc::new(prepare_mouse_data(mouse_data, node_layout)),
                                &mut will_bubble,
                                resolved_events,
                                node,
                                dom,
                            );
                        }
                    }
                }
            }

            {
                // wheel
                if let Some(w) = wheel_data {
                    if wheel_delta != 0.0 {
                        let mut will_bubble = FxHashSet::default();
                        for node in dom.get_listening_sorted("wheel") {
                            let node_layout =
                                layout.layout(node.state.layout.node.unwrap()).unwrap();
                            let currently_contains = layout_contains_point(node_layout, new_pos);

                            if currently_contains {
                                try_create_event(
                                    "wheel",
                                    Arc::new(w.clone()),
                                    &mut will_bubble,
                                    resolved_events,
                                    node,
                                    dom,
                                );
                            }
                        }
                    }
                }
            }

            {
                // mouseleave
                let mut will_bubble = FxHashSet::default();
                for node in dom.get_listening_sorted("mouseleave") {
                    let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                    let previously_contained = old_pos
                        .filter(|pos| layout_contains_point(node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(node_layout, new_pos);

                    if !currently_contains && previously_contained {
                        try_create_event(
                            "mouseleave",
                            Arc::new(prepare_mouse_data(mouse_data, node_layout)),
                            &mut will_bubble,
                            resolved_events,
                            node,
                            dom,
                        );
                    }
                }
            }

            {
                // mouseout
                let mut will_bubble = FxHashSet::default();
                for node in dom.get_listening_sorted("mouseout") {
                    let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                    let previously_contained = old_pos
                        .filter(|pos| layout_contains_point(node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(node_layout, new_pos);

                    if !currently_contains && previously_contained {
                        try_create_event(
                            "mouseout",
                            Arc::new(prepare_mouse_data(mouse_data, node_layout)),
                            &mut will_bubble,
                            resolved_events,
                            node,
                            dom,
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
    pub fn new() -> (
        Self,
        Rc<RefCell<InnerInputState>>,
        impl FnMut(crossterm::event::Event),
    ) {
        let queued_events = Rc::new(RefCell::new(Vec::new()));
        let queued_events2 = Rc::downgrade(&queued_events);

        let regester_event = move |evt: crossterm::event::Event| {
            if let Some(evt) = get_event(evt) {
                if let Some(v) = queued_events2.upgrade() {
                    (*v).borrow_mut().push(evt);
                }
            }
        };

        let state = Rc::new(RefCell::new(InnerInputState::new()));

        (
            Self {
                state: state.clone(),
                queued_events,
            },
            state,
            regester_event,
        )
    }

    pub(crate) fn get_events(&self, layout: &Stretch, dom: &mut Dom) -> Vec<UserEvent> {
        let mut resolved_events = Vec::new();

        (*self.state).borrow_mut().update(
            &mut (*self.queued_events).borrow_mut(),
            &mut resolved_events,
            layout,
            dom,
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
        let mut hm: FxHashMap<&'static str, Vec<Arc<dyn Any + Send + Sync>>> = FxHashMap::default();
        for (event, data) in events {
            if let Some(v) = hm.get_mut(event) {
                v.push(data);
            } else {
                hm.insert(event, vec![data]);
            }
        }
        for (event, datas) in hm {
            for node in dom.get_listening_sorted(event) {
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

            let get_mouse_data = |crossterm_button: Option<MouseButton>| {
                let button = crossterm_button.map(|b| match b {
                    MouseButton::Left => DioxusMouseButton::Primary,
                    MouseButton::Right => DioxusMouseButton::Secondary,
                    MouseButton::Middle => DioxusMouseButton::Auxiliary,
                });

                // from https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent

                // The `page` and `screen` coordinates are inconsistent with the MDN definition, as they are relative to the viewport (client), not the target element/page/screen, respectively.
                // todo?
                // But then, MDN defines them in terms of pixels, yet crossterm provides only row/column, and it might not be possible to get pixels. So we can't get 100% consistency anyway.
                let coordinates = Coordinates::new(
                    ScreenPoint::new(x, y),
                    ClientPoint::new(x, y),
                    // offset x/y are set when the origin of the event is assigned to an element
                    ElementPoint::new(0., 0.),
                    PagePoint::new(x, y),
                );

                let mut modifiers = Modifiers::empty();
                if shift {
                    modifiers.insert(Modifiers::SHIFT);
                }
                if ctrl {
                    modifiers.insert(Modifiers::CONTROL);
                }
                if meta {
                    modifiers.insert(Modifiers::META);
                }
                if alt {
                    modifiers.insert(Modifiers::ALT);
                }

                // held mouse buttons get set later by maintaining state, as crossterm does not provide them
                EventData::Mouse(MouseData::new(
                    coordinates,
                    button,
                    DioxusMouseButtons::empty(),
                    modifiers,
                ))
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
        key: key_str,
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
