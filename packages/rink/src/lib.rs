use crate::focus::Focus;
use anyhow::Result;
use crossterm::{
    cursor::{MoveTo, RestorePosition, SavePosition, Show},
    event::{DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dioxus_native_core::prelude::*;
use dioxus_native_core::{real_dom::RealDom, FxDashSet, NodeId, SendAnyMap};
use focus::FocusState;
use futures::{channel::mpsc::UnboundedSender, pin_mut, Future, StreamExt};
use futures_channel::mpsc::unbounded;
use layout::TaffyLayout;
use prevent_default::PreventDefault;
use std::{io, time::Duration};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
};
use std::{rc::Rc, sync::RwLock};
use style_attributes::StyleModifier;
use taffy::Taffy;
pub use taffy::{geometry::Point, prelude::*};
use tokio::select;
use tui::{backend::CrosstermBackend, layout::Rect, Terminal};
use widgets::{register_widgets, RinkWidgetResponder, RinkWidgetTraitObject};

mod config;
mod focus;
mod hooks;
mod layout;
mod prevent_default;
pub mod query;
mod render;
mod style;
mod style_attributes;
mod widget;
mod widgets;

pub use config::*;
pub use hooks::*;
pub use query::Query;

// the layout space has a multiplier of 10 to minimize rounding errors
pub(crate) fn screen_to_layout_space(screen: u16) -> f32 {
    screen as f32 * 10.0
}

pub(crate) fn unit_to_layout_space(screen: f32) -> f32 {
    screen * 10.0
}

pub(crate) fn layout_to_screen_space(layout: f32) -> f32 {
    layout / 10.0
}

#[derive(Clone)]
pub struct TuiContext {
    tx: UnboundedSender<InputEvent>,
}

impl TuiContext {
    pub fn new(tx: UnboundedSender<InputEvent>) -> Self {
        Self { tx }
    }

    pub fn quit(&self) {
        self.tx.unbounded_send(InputEvent::Close).unwrap();
    }

    pub fn inject_event(&self, event: crossterm::event::Event) {
        self.tx
            .unbounded_send(InputEvent::UserInput(event))
            .unwrap();
    }
}

pub fn render<R: Driver>(
    cfg: Config,
    create_renderer: impl FnOnce(
        &Arc<RwLock<RealDom>>,
        &Arc<Mutex<Taffy>>,
        UnboundedSender<InputEvent>,
    ) -> R,
) -> Result<()> {
    let mut rdom = RealDom::new([
        TaffyLayout::to_type_erased(),
        Focus::to_type_erased(),
        StyleModifier::to_type_erased(),
        PreventDefault::to_type_erased(),
    ]);

    // Setup input handling

    // The event channel for fully resolved events
    let (event_tx, mut event_reciever) = unbounded();

    // The event channel for raw terminal events
    let (raw_event_tx, mut raw_event_reciever) = unbounded();
    let event_tx_clone = raw_event_tx.clone();
    if !cfg.headless {
        std::thread::spawn(move || {
            let tick_rate = Duration::from_millis(1000);
            loop {
                if crossterm::event::poll(tick_rate).unwrap() {
                    let evt = crossterm::event::read().unwrap();
                    if raw_event_tx
                        .unbounded_send(InputEvent::UserInput(evt))
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });
    }

    register_widgets(&mut rdom, event_tx);

    let (handler, mut register_event) = RinkInputHandler::create(&mut rdom);

    let rdom = Arc::new(RwLock::new(rdom));
    let taffy = Arc::new(Mutex::new(Taffy::new()));
    let mut renderer = create_renderer(&rdom, &taffy, event_tx_clone);

    // insert the query engine into the rdom
    let query_engine = Query::new(rdom.clone(), taffy.clone());
    {
        let mut rdom = rdom.write().unwrap();
        rdom.raw_world_mut().add_unique(query_engine);
    }

    {
        renderer.update(&rdom);
        let mut any_map = SendAnyMap::new();
        any_map.insert(taffy.clone());
        let mut rdom = rdom.write().unwrap();
        let _ = rdom.update_state(any_map);
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut terminal = (!cfg.headless).then(|| {
                enable_raw_mode().unwrap();
                let mut stdout = std::io::stdout();
                execute!(
                    stdout,
                    EnterAlternateScreen,
                    EnableMouseCapture,
                    MoveTo(0, 1000)
                )
                .unwrap();
                let backend = CrosstermBackend::new(io::stdout());
                Terminal::new(backend).unwrap()
            });
            if let Some(terminal) = &mut terminal {
                terminal.clear().unwrap();
            }

            let mut to_rerender = FxDashSet::default();
            to_rerender.insert(rdom.read().unwrap().root_id());
            let mut updated = true;

            loop {
                /*
                -> render the nodes in the right place with tui/crossterm
                -> wait for changes
                -> resolve events
                -> lazily update the layout and style based on nodes changed
                use simd to compare lines for diffing?
                todo: lazy re-rendering
                */

                if !to_rerender.is_empty() || updated {
                    updated = false;
                    fn resize(dims: Rect, taffy: &mut Taffy, rdom: &RealDom) {
                        let width = screen_to_layout_space(dims.width);
                        let height = screen_to_layout_space(dims.height);
                        let root_node = rdom
                            .get(rdom.root_id())
                            .unwrap()
                            .get::<TaffyLayout>()
                            .unwrap()
                            .node
                            .unwrap();

                        // the root node fills the entire area
                        let mut style = *taffy.style(root_node).unwrap();
                        let new_size = Size {
                            width: Dimension::Points(width),
                            height: Dimension::Points(height),
                        };
                        if style.size != new_size {
                            style.size = new_size;
                            taffy.set_style(root_node, style).unwrap();
                        }

                        let size = Size {
                            width: AvailableSpace::Definite(width),
                            height: AvailableSpace::Definite(height),
                        };
                        taffy.compute_layout(root_node, size).unwrap();
                    }
                    if let Some(terminal) = &mut terminal {
                        execute!(terminal.backend_mut(), SavePosition).unwrap();
                        terminal.draw(|frame| {
                            let rdom = rdom.write().unwrap();
                            let mut taffy = taffy.lock().expect("taffy lock poisoned");
                            // size is guaranteed to not change when rendering
                            resize(frame.size(), &mut taffy, &rdom);
                            let root = rdom.get(rdom.root_id()).unwrap();
                            render::render_vnode(frame, &taffy, root, cfg, Point::ZERO);
                        })?;
                        execute!(terminal.backend_mut(), RestorePosition, Show).unwrap();
                    } else {
                        let rdom = rdom.read().unwrap();
                        resize(
                            Rect {
                                x: 0,
                                y: 0,
                                width: 1000,
                                height: 1000,
                            },
                            &mut taffy.lock().expect("taffy lock poisoned"),
                            &rdom,
                        );
                    }
                }

                let mut event_recieved = None;
                {
                    let wait = renderer.poll_async();

                    pin_mut!(wait);

                    select! {
                        _ = wait => {

                        },
                        evt = raw_event_reciever.next() => {
                            match evt.as_ref().unwrap() {
                                InputEvent::UserInput(event) => match event {
                                    TermEvent::Key(key) => {
                                        if matches!(key.code, KeyCode::Char('C' | 'c'))
                                        && key.modifiers.contains(KeyModifiers::CONTROL)
                                        && cfg.ctrl_c_quit
                                        {
                                            break;
                                        }
                                    }
                                    TermEvent::Resize(_, _) => updated = true,
                                    TermEvent::Mouse(_) => {}
                                },
                                InputEvent::Close => break,
                            };

                            if let InputEvent::UserInput(evt) = evt.unwrap() {
                                register_event(evt);
                            }
                        },
                        Some(evt) = event_reciever.next() => {
                            event_recieved=Some(evt);
                        }
                    }
                }

                {
                    if let Some(evt) = event_recieved {
                        renderer.handle_event(
                            &rdom,
                            evt.id,
                            evt.name,
                            Rc::new(evt.data),
                            evt.bubbles,
                        );
                    }
                    {
                        let evts = handler.get_events(
                            &taffy.lock().expect("taffy lock poisoned"),
                            &mut rdom.write().unwrap(),
                        );
                        updated |= handler.state().focus_state.clean();

                        for e in evts {
                            bubble_event_to_widgets(&mut rdom.write().unwrap(), &e);
                            renderer.handle_event(&rdom, e.id, e.name, Rc::new(e.data), e.bubbles);
                        }
                    }
                    // updates the dom's nodes
                    renderer.update(&rdom);
                    // update the style and layout
                    let mut rdom = rdom.write().unwrap();
                    let mut any_map = SendAnyMap::new();
                    any_map.insert(taffy.clone());
                    let (new_to_rerender, dirty) = rdom.update_state(any_map);
                    to_rerender = new_to_rerender;
                    let text_mask = NodeMaskBuilder::new().with_text().build();
                    for (id, mask) in dirty {
                        if mask.overlaps(&text_mask) {
                            to_rerender.insert(id);
                        }
                    }
                }
            }

            if let Some(terminal) = &mut terminal {
                disable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )?;
                terminal.show_cursor()?;
            }

            Ok(())
        })
}

#[derive(Debug)]
pub enum InputEvent {
    UserInput(TermEvent),
    Close,
}

pub trait Driver {
    fn update(&mut self, rdom: &Arc<RwLock<RealDom>>);
    fn handle_event(
        &mut self,
        rdom: &Arc<RwLock<RealDom>>,
        id: NodeId,
        event: &str,
        value: Rc<EventData>,
        bubbles: bool,
    );
    fn poll_async(&mut self) -> Pin<Box<dyn Future<Output = ()> + '_>>;
}

/// Before sending the event to drivers, we need to bubble it up the tree to any widgets that are listening
fn bubble_event_to_widgets(rdom: &mut RealDom, event: &Event) {
    let id = event.id;
    let mut node = Some(rdom.get_mut(id).unwrap());

    while let Some(mut node_mut) = node {
        let parent_id = node_mut.parent_id();
        if let Some(mut widget) = node_mut
            .get_mut::<RinkWidgetTraitObject>()
            .map(|w| w.clone())
        {
            widget.handle_event(event, &mut node_mut)
        }
        if !event.bubbles {
            break;
        }
        node = parent_id.map(|id| rdom.get_mut(id).unwrap());
    }
}
