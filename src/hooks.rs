use anyhow::Result;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyEvent, MouseEvent,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dioxus::{core::exports::futures_channel::mpsc::unbounded, prelude::Props};
use dioxus::{core::*, prelude::*};
use futures::{channel::mpsc::UnboundedReceiver, future::Either, pin_mut, StreamExt};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    io,
    rc::Rc,
    time::{Duration, Instant},
};
use stretch2::{prelude::Size, Stretch};
use tokio::sync::broadcast::Receiver;
use tui::{backend::CrosstermBackend, style::Style as TuiStyle, Terminal};

pub struct RinkContext {
    last_event: Rc<Cell<Option<TermEvent>>>,
    subscribers: Rc<RefCell<HashMap<ScopeId, bool>>>,
}

impl RinkContext {
    pub fn new(mut receiver: UnboundedReceiver<TermEvent>, cx: &ScopeState) -> Self {
        let updater = cx.schedule_update_any();
        let last_event = Rc::new(Cell::new(None));
        let last_event2 = last_event.clone();
        let subscribers = Rc::new(RefCell::new(HashMap::new()));
        let subscribers2 = subscribers.clone();

        cx.push_future(async move {
            while let Some(evt) = receiver.next().await {
                last_event2.replace(Some(evt));
                for (subscriber, received) in subscribers2.borrow_mut().iter_mut() {
                    updater(*subscriber);
                    *received = false;
                }
            }
        });

        Self {
            last_event: last_event,
            subscribers: subscribers,
        }
    }

    pub fn subscribe_to_events(&self, scope: ScopeId) {
        self.subscribers.borrow_mut().insert(scope, false);
    }

    pub fn get_event(&self, scope: ScopeId) -> Option<TermEvent> {
        let mut subscribers = self.subscribers.borrow_mut();
        let received = subscribers.get_mut(&scope)?;
        if !*received {
            *received = true;
            self.last_event.get()
        } else {
            None
        }
    }
}

#[derive(Props)]
pub struct AppHandlerProps<'a> {
    #[props(default)]
    onkeydown: EventHandler<'a, KeyEvent>,

    #[props(default)]
    onmousedown: EventHandler<'a, MouseEvent>,

    #[props(default)]
    onresize: EventHandler<'a, (u16, u16)>,
}

/// This component lets you handle input events
///
/// Once attached to the DOM, it will listen for input events from the terminal
///
///
pub fn InputHandler<'a>(cx: Scope<'a, AppHandlerProps<'a>>) -> Element {
    let rcx = cx.use_hook(|_| {
        let rcx = cx
            .consume_context::<RinkContext>()
            .unwrap_or_else(|| panic!("Rink InputHandlers can only be used in Rink apps!"));

        // our component will only re-render if new events are received ... or if the parent is updated
        // todo: if update was not caused by a new event, we should not re-render
        // perhaps add some tracking to context?
        rcx.subscribe_to_events(cx.scope_id());

        rcx
    });

    {
        if let Some(evet) = rcx.get_event(cx.scope_id()) {
            match evet {
                TermEvent::Key(key) => {
                    cx.props.onkeydown.call(key.clone());
                    // let mut handler = cx.props.keydown.borrow_mut();
                    // handler(*key);
                    // if let Some(handler) = cx.props.onkeydown {
                    //     handler(*key);
                    // }
                }
                TermEvent::Mouse(mouse) => {
                    cx.props.onmousedown.call(mouse.clone());
                }
                TermEvent::Resize(x, y) => {
                    cx.props.onresize.call((x, y));
                }
            }
        }
    }

    None
}
