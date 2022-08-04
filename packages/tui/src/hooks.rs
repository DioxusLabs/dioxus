use crossterm::event::{
    Event as TermEvent, KeyCode as TermKeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use dioxus_core::*;
use fxhash::{FxHashMap, FxHashSet};

use dioxus_html::geometry::euclid::{Point2D, Rect, Size2D};
use dioxus_html::geometry::{
    ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint, WheelDelta,
};
use dioxus_html::input_data::keyboard_types::{Code, Key, Location, Modifiers};
use dioxus_html::input_data::MouseButtonSet as DioxusMouseButtons;
use dioxus_html::input_data::{MouseButton as DioxusMouseButton, MouseButtonSet};
use dioxus_html::{event_bubbles, on::*};
use std::{
    any::Any,
    cell::{RefCell, RefMut},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};
use taffy::geometry::{Point, Size};
use taffy::{prelude::Layout, Taffy};

use crate::FocusState;
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
//         mouse.as_ref().map(|m| m.clone())
//     }

//     pub fn wheel(&self) -> Option<WheelData> {
//         let data = (**self.0).borrow();
//         wheel.as_ref().map(|w| w.clone())
//     }

//     pub fn screen(&self) -> Option<(u16, u16)> {
//         let data = (**self.0).borrow();
//         screen.as_ref().map(|m| m.clone())
//     }

//     pub fn last_key_pressed(&self) -> Option<KeyboardData> {
//         let data = (**self.0).borrow();
//         last_key_pressed
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
    pub(crate) focus_state: FocusState,
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
            focus_state: FocusState::default(),
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
                let is_repeating = self
                    .last_key_pressed
                    .as_ref()
                    // heuristic for guessing which presses are auto-repeating. not necessarily accurate
                    .filter(|(last_data, last_instant)| {
                        last_data.key() == k.key() && last_instant.elapsed() < MAX_REPEAT_TIME
                    })
                    .is_some();

                if is_repeating {
                    *k = KeyboardData::new(k.key(), k.code(), k.location(), true, k.modifiers());
                }

                self.last_key_pressed = Some((k.clone(), Instant::now()));
            }
        }
    }

    fn update(
        &mut self,
        evts: &mut Vec<EventCore>,
        resolved_events: &mut Vec<UserEvent>,
        layout: &Taffy,
        dom: &mut Dom,
    ) {
        let previous_mouse = self.mouse.clone();

        self.wheel = None;

        let old_focus = self.focus_state.last_focused_id;

        evts.retain(|e| match &e.1 {
            EventData::Keyboard(k) => match k.code() {
                Code::Tab => !self
                    .focus_state
                    .progress(dom, !k.modifiers().contains(Modifiers::SHIFT)),
                _ => true,
            },
            _ => true,
        });

        for e in evts.iter_mut() {
            self.apply_event(e);
        }

        self.resolve_mouse_events(previous_mouse, resolved_events, layout, dom);

        if old_focus != self.focus_state.last_focused_id {
            if let Some(id) = self.focus_state.last_focused_id {
                resolved_events.push(UserEvent {
                    scope_id: None,
                    priority: EventPriority::Medium,
                    name: "focus",
                    element: Some(id),
                    data: Arc::new(FocusData {}),
                    bubbles: event_bubbles("focus"),
                });
                resolved_events.push(UserEvent {
                    scope_id: None,
                    priority: EventPriority::Medium,
                    name: "focusin",
                    element: Some(id),
                    data: Arc::new(FocusData {}),
                    bubbles: event_bubbles("focusin"),
                });
            }
            if let Some(id) = old_focus {
                resolved_events.push(UserEvent {
                    scope_id: None,
                    priority: EventPriority::Medium,
                    name: "focusout",
                    element: Some(id),
                    data: Arc::new(FocusData {}),
                    bubbles: event_bubbles("focusout"),
                });
            }
        }

        // for s in &self.subscribers {
        //     s();
        // }
    }

    fn resolve_mouse_events(
        &mut self,
        previous_mouse: Option<MouseData>,
        resolved_events: &mut Vec<UserEvent>,
        layout: &Taffy,
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
                    parent = dom[parent_id].parent;
                }
                resolved_events.push(UserEvent {
                    scope_id: None,
                    priority: EventPriority::Medium,
                    name,
                    element: Some(node.id),
                    data,
                    bubbles: event_bubbles(name),
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

            let was_scrolled = self
                .wheel
                .as_ref()
                .map_or(false, |data| !data.delta().is_zero());
            let wheel_data = &self.wheel;

            {
                // mousemove
                if old_pos != Some(new_pos) {
                    let mut will_bubble = FxHashSet::default();
                    for node in dom.get_listening_sorted("mousemove") {
                        let node_layout = get_abs_layout(node, dom, layout);
                        let previously_contained = old_pos
                            .filter(|pos| layout_contains_point(&node_layout, *pos))
                            .is_some();
                        let currently_contains = layout_contains_point(&node_layout, new_pos);

                        if currently_contains && previously_contained {
                            try_create_event(
                                "mousemove",
                                Arc::new(prepare_mouse_data(mouse_data, &node_layout)),
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
                    let node_layout = get_abs_layout(node, dom, layout);
                    let previously_contained = old_pos
                        .filter(|pos| layout_contains_point(&node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(&node_layout, new_pos);

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
                    let node_layout = get_abs_layout(node, dom, layout);
                    let previously_contained = old_pos
                        .filter(|pos| layout_contains_point(&node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(&node_layout, new_pos);

                    if currently_contains && !previously_contained {
                        try_create_event(
                            "mouseover",
                            Arc::new(prepare_mouse_data(mouse_data, &node_layout)),
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
                    let node_layout = get_abs_layout(node, dom, layout);
                    let currently_contains = layout_contains_point(&node_layout, new_pos);

                    if currently_contains {
                        try_create_event(
                            "mousedown",
                            Arc::new(prepare_mouse_data(mouse_data, &node_layout)),
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
                        let node_layout = get_abs_layout(node, dom, layout);
                        let currently_contains = layout_contains_point(&node_layout, new_pos);

                        if currently_contains {
                            try_create_event(
                                "mouseup",
                                Arc::new(prepare_mouse_data(mouse_data, &node_layout)),
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
                        let node_layout = get_abs_layout(node, dom, layout);
                        let currently_contains = layout_contains_point(&node_layout, new_pos);

                        if currently_contains {
                            try_create_event(
                                "click",
                                Arc::new(prepare_mouse_data(mouse_data, &node_layout)),
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
                        let node_layout = get_abs_layout(node, dom, layout);
                        let currently_contains = layout_contains_point(&node_layout, new_pos);

                        if currently_contains {
                            try_create_event(
                                "contextmenu",
                                Arc::new(prepare_mouse_data(mouse_data, &node_layout)),
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
                    if was_scrolled {
                        let mut will_bubble = FxHashSet::default();
                        for node in dom.get_listening_sorted("wheel") {
                            let node_layout = get_abs_layout(node, dom, layout);

                            let currently_contains = layout_contains_point(&node_layout, new_pos);

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
                    let node_layout = get_abs_layout(node, dom, layout);
                    let previously_contained = old_pos
                        .filter(|pos| layout_contains_point(&node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(&node_layout, new_pos);

                    if !currently_contains && previously_contained {
                        try_create_event(
                            "mouseleave",
                            Arc::new(prepare_mouse_data(mouse_data, &node_layout)),
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
                    let node_layout = get_abs_layout(node, dom, layout);
                    let previously_contained = old_pos
                        .filter(|pos| layout_contains_point(&node_layout, *pos))
                        .is_some();
                    let currently_contains = layout_contains_point(&node_layout, new_pos);

                    if !currently_contains && previously_contained {
                        try_create_event(
                            "mouseout",
                            Arc::new(prepare_mouse_data(mouse_data, &node_layout)),
                            &mut will_bubble,
                            resolved_events,
                            node,
                            dom,
                        );
                    }
                }
            }

            // update focus
            if was_released {
                let mut focus_id = None;
                dom.traverse_depth_first(|node| {
                    let node_layout = layout.layout(node.state.layout.node.unwrap()).unwrap();
                    let currently_contains = layout_contains_point(node_layout, new_pos);

                    if currently_contains && node.state.focus.level.focusable() {
                        focus_id = Some(node.id);
                    }
                });
                if let Some(id) = focus_id {
                    self.focus_state.set_focus(dom, id);
                }
            }
        }
    }

    // fn subscribe(&mut self, f: Rc<dyn Fn() + 'static>) {
    //     self.subscribers.push(f)
    // }
}

fn get_abs_layout(node: &Node, dom: &Dom, taffy: &Taffy) -> Layout {
    let mut node_layout = *taffy.layout(node.state.layout.node.unwrap()).unwrap();
    let mut current = node;

    while let Some(parent_id) = current.parent {
        let parent = &dom[parent_id];
        current = parent;
        let parent_layout = taffy.layout(parent.state.layout.node.unwrap()).unwrap();
        node_layout.location.x += parent_layout.location.x;
        node_layout.location.y += parent_layout.location.y;
    }
    node_layout
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

    pub(crate) fn prune(&self, mutations: &dioxus_core::Mutations, rdom: &Dom) {
        self.state.borrow_mut().focus_state.prune(mutations, rdom);
    }

    pub(crate) fn get_events(&self, layout: &Taffy, dom: &mut Dom) -> Vec<UserEvent> {
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
                    if node.state.focused {
                        resolved_events.push(UserEvent {
                            scope_id: None,
                            priority: EventPriority::Medium,
                            name: event,
                            element: Some(node.id),
                            data: data.clone(),
                            bubbles: event_bubbles(event),
                        });
                    }
                }
            }
        }

        resolved_events
    }

    pub(crate) fn state(&self) -> RefMut<InnerInputState> {
        self.state.borrow_mut()
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
                let y = if up { -1.0 } else { 1.0 };
                EventData::Wheel(WheelData::new(WheelDelta::lines(0., y, 0.)))
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
    let key = key_from_crossterm_key_code(event.code);
    // crossterm does not provide code. we make a guess as to which key might have been pressed
    // this is probably garbage if the user has a custom keyboard layout
    let code = guess_code_from_crossterm_key_code(event.code)?;
    let modifiers = modifiers_from_crossterm_modifiers(event.modifiers);

    Some(EventData::Keyboard(KeyboardData::new(
        key,
        code,
        Location::Standard,
        false,
        modifiers,
    )))
}

/// The crossterm key_code nicely represents the meaning of the key and we can mostly convert it without any issues
///
/// Exceptions:
/// BackTab is converted to Key::Tab, and Null is converted to Key::Unidentified
fn key_from_crossterm_key_code(key_code: TermKeyCode) -> Key {
    match key_code {
        TermKeyCode::Backspace => Key::Backspace,
        TermKeyCode::Enter => Key::Enter,
        TermKeyCode::Left => Key::ArrowLeft,
        TermKeyCode::Right => Key::ArrowRight,
        TermKeyCode::Up => Key::ArrowUp,
        TermKeyCode::Down => Key::ArrowDown,
        TermKeyCode::Home => Key::Home,
        TermKeyCode::End => Key::End,
        TermKeyCode::PageUp => Key::PageUp,
        TermKeyCode::PageDown => Key::PageDown,
        TermKeyCode::Tab => Key::Tab,
        // ? no corresponding Key
        TermKeyCode::BackTab => Key::Tab,
        TermKeyCode::Delete => Key::Delete,
        TermKeyCode::Insert => Key::Insert,
        TermKeyCode::F(1) => Key::F1,
        TermKeyCode::F(2) => Key::F2,
        TermKeyCode::F(3) => Key::F3,
        TermKeyCode::F(4) => Key::F4,
        TermKeyCode::F(5) => Key::F5,
        TermKeyCode::F(6) => Key::F6,
        TermKeyCode::F(7) => Key::F7,
        TermKeyCode::F(8) => Key::F8,
        TermKeyCode::F(9) => Key::F9,
        TermKeyCode::F(10) => Key::F10,
        TermKeyCode::F(11) => Key::F11,
        TermKeyCode::F(12) => Key::F12,
        TermKeyCode::F(13) => Key::F13,
        TermKeyCode::F(14) => Key::F14,
        TermKeyCode::F(15) => Key::F15,
        TermKeyCode::F(16) => Key::F16,
        TermKeyCode::F(17) => Key::F17,
        TermKeyCode::F(18) => Key::F18,
        TermKeyCode::F(19) => Key::F19,
        TermKeyCode::F(20) => Key::F20,
        TermKeyCode::F(21) => Key::F21,
        TermKeyCode::F(22) => Key::F22,
        TermKeyCode::F(23) => Key::F23,
        TermKeyCode::F(24) => Key::F24,
        TermKeyCode::F(other) => {
            panic!("Unexpected function key: {other:?}")
        }
        TermKeyCode::Char(c) => Key::Character(c.to_string()),
        TermKeyCode::Null => Key::Unidentified,
        TermKeyCode::Esc => Key::Escape,
    }
}

// Crossterm does not provide a way to get the `code` (physical key on keyboard)
// So we make a guess based on their `key_code`, but this is probably going to break on anything other than a very standard european keyboard
// It may look fine, but it's a horrible hack. But there's nothing better we can do.
fn guess_code_from_crossterm_key_code(key_code: TermKeyCode) -> Option<Code> {
    let code = match key_code {
        TermKeyCode::Backspace => Code::Backspace,
        TermKeyCode::Enter => Code::Enter,
        TermKeyCode::Left => Code::ArrowLeft,
        TermKeyCode::Right => Code::ArrowRight,
        TermKeyCode::Up => Code::ArrowUp,
        TermKeyCode::Down => Code::ArrowDown,
        TermKeyCode::Home => Code::Home,
        TermKeyCode::End => Code::End,
        TermKeyCode::PageUp => Code::PageUp,
        TermKeyCode::PageDown => Code::PageDown,
        TermKeyCode::Tab => Code::Tab,
        // ? Apparently you get BackTab by pressing Tab
        TermKeyCode::BackTab => Code::Tab,
        TermKeyCode::Delete => Code::Delete,
        TermKeyCode::Insert => Code::Insert,
        TermKeyCode::F(1) => Code::F1,
        TermKeyCode::F(2) => Code::F2,
        TermKeyCode::F(3) => Code::F3,
        TermKeyCode::F(4) => Code::F4,
        TermKeyCode::F(5) => Code::F5,
        TermKeyCode::F(6) => Code::F6,
        TermKeyCode::F(7) => Code::F7,
        TermKeyCode::F(8) => Code::F8,
        TermKeyCode::F(9) => Code::F9,
        TermKeyCode::F(10) => Code::F10,
        TermKeyCode::F(11) => Code::F11,
        TermKeyCode::F(12) => Code::F12,
        TermKeyCode::F(13) => Code::F13,
        TermKeyCode::F(14) => Code::F14,
        TermKeyCode::F(15) => Code::F15,
        TermKeyCode::F(16) => Code::F16,
        TermKeyCode::F(17) => Code::F17,
        TermKeyCode::F(18) => Code::F18,
        TermKeyCode::F(19) => Code::F19,
        TermKeyCode::F(20) => Code::F20,
        TermKeyCode::F(21) => Code::F21,
        TermKeyCode::F(22) => Code::F22,
        TermKeyCode::F(23) => Code::F23,
        TermKeyCode::F(24) => Code::F24,
        TermKeyCode::F(other) => {
            panic!("Unexpected function key: {other:?}")
        }
        // this is a horrible way for crossterm to represent keys but we have to deal with it
        TermKeyCode::Char(c) => match c {
            'A'..='Z' | 'a'..='z' => match c.to_ascii_uppercase() {
                'A' => Code::KeyA,
                'B' => Code::KeyB,
                'C' => Code::KeyC,
                'D' => Code::KeyD,
                'E' => Code::KeyE,
                'F' => Code::KeyF,
                'G' => Code::KeyG,
                'H' => Code::KeyH,
                'I' => Code::KeyI,
                'J' => Code::KeyJ,
                'K' => Code::KeyK,
                'L' => Code::KeyL,
                'M' => Code::KeyM,
                'N' => Code::KeyN,
                'O' => Code::KeyO,
                'P' => Code::KeyP,
                'Q' => Code::KeyQ,
                'R' => Code::KeyR,
                'S' => Code::KeyS,
                'T' => Code::KeyT,
                'U' => Code::KeyU,
                'V' => Code::KeyV,
                'W' => Code::KeyW,
                'X' => Code::KeyX,
                'Y' => Code::KeyY,
                'Z' => Code::KeyZ,
                _ => unreachable!("Exhaustively checked all characters in range A..Z"),
            },
            ' ' => Code::Space,
            '[' | '{' => Code::BracketLeft,
            ']' | '}' => Code::BracketRight,
            ';' => Code::Semicolon,
            ':' => Code::Semicolon,
            ',' => Code::Comma,
            '<' => Code::Comma,
            '.' => Code::Period,
            '>' => Code::Period,
            '1' => Code::Digit1,
            '2' => Code::Digit2,
            '3' => Code::Digit3,
            '4' => Code::Digit4,
            '5' => Code::Digit5,
            '6' => Code::Digit6,
            '7' => Code::Digit7,
            '8' => Code::Digit8,
            '9' => Code::Digit9,
            '0' => Code::Digit0,
            '!' => Code::Digit1,
            '@' => Code::Digit2,
            '#' => Code::Digit3,
            '$' => Code::Digit4,
            '%' => Code::Digit5,
            '^' => Code::Digit6,
            '&' => Code::Digit7,
            '*' => Code::Digit8,
            '(' => Code::Digit9,
            ')' => Code::Digit0,
            // numpad characters are ambiguous; we don't know which key was really pressed
            // it could be also:
            // '*' => Code::Multiply,
            // '/' => Code::Divide,
            // '-' => Code::Subtract,
            // '+' => Code::Add,
            '+' => Code::Equal,
            '-' | '_' => Code::Minus,
            '\'' => Code::Quote,
            '"' => Code::Quote,
            '\\' => Code::Backslash,
            '|' => Code::Backslash,
            '/' => Code::Slash,
            '?' => Code::Slash,
            '=' => Code::Equal,
            '`' => Code::Backquote,
            '~' => Code::Backquote,
            _ => return None,
        },
        TermKeyCode::Null => return None,
        TermKeyCode::Esc => Code::Escape,
    };

    Some(code)
}

fn modifiers_from_crossterm_modifiers(src: KeyModifiers) -> Modifiers {
    let mut modifiers = Modifiers::empty();

    if src.contains(KeyModifiers::SHIFT) {
        modifiers.insert(Modifiers::SHIFT);
    }

    if src.contains(KeyModifiers::ALT) {
        modifiers.insert(Modifiers::ALT);
    }

    if src.contains(KeyModifiers::CONTROL) {
        modifiers.insert(Modifiers::CONTROL);
    }

    modifiers
}
