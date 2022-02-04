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
use tui::{backend::CrosstermBackend, style::Style as TuiStyle, Terminal};

pub struct RinkContext {
    last_event: RefCell<Option<TermEvent>>,
    receiver: Rc<Cell<Option<UnboundedReceiver<TermEvent>>>>,
}

impl RinkContext {
    pub fn new(receiver: UnboundedReceiver<TermEvent>) -> Self {
        Self {
            last_event: RefCell::new(None),
            receiver: Rc::new(Cell::new(Some(receiver))),
        }
    }
    pub fn subscribe_to_events(&self, scope: ScopeId) {
        //
    }
}

#[derive(Props)]
pub struct AppHandlerProps<'a> {
    #[props(default)]
    onkeydown: EventHandler<'a, KeyEvent>,

    #[props(default)]
    onmousedown: EventHandler<'a, MouseEvent>,

    #[props(default)]
    onresize: Option<EventHandler<'a, (u16, u16)>>,
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

        let mut rec = rcx.receiver.take().unwrap();
        let updater = cx.schedule_update();
        let rc2 = rcx.clone();
        cx.push_future(async move {
            while let Some(evt) = rec.next().await {
                rc2.last_event.borrow_mut().replace(evt);
                println!("{:?}", evt);
                updater();
            }
            //
        });

        rcx
    });

    if let Some(evet) = rcx.last_event.borrow().as_ref() {
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
                // if let Some(handler) = cx.props.onresize {
                //     handler((*x, *y));
                // }
            }
        }
    }

    None
}
