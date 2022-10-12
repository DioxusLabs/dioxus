use anyhow::Result;
use anymap::AnyMap;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dioxus_core::exports::futures_channel::mpsc::unbounded;
use dioxus_core::*;
use dioxus_native_core::real_dom::RealDom;
use focus::FocusState;
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    pin_mut, StreamExt,
};
use query::Query;
use std::cell::RefCell;
use std::rc::Rc;
use std::{io, time::Duration};
use taffy::Taffy;
pub use taffy::{geometry::Point, prelude::Size};
use tui::{backend::CrosstermBackend, layout::Rect, Terminal};

mod config;
mod focus;
mod hooks;
mod layout;
mod node;
pub mod query;
mod render;
mod style;
mod style_attributes;
mod widget;

pub use config::*;
pub use hooks::*;
pub(crate) use node::*;

#[derive(Clone)]
pub struct TuiContext {
    tx: UnboundedSender<InputEvent>,
}
impl TuiContext {
    pub fn quit(&self) {
        self.tx.unbounded_send(InputEvent::Close).unwrap();
    }

    pub fn inject_event(&self, event: crossterm::event::Event) {
        self.tx
            .unbounded_send(InputEvent::UserInput(event))
            .unwrap();
    }
}

pub fn launch(app: Component<()>) {
    launch_cfg(app, Config::default())
}

pub fn launch_cfg(app: Component<()>, cfg: Config) {
    let mut dom = VirtualDom::new(app);

    let (handler, state, register_event) = RinkInputHandler::new();

    // Setup input handling
    let (event_tx, event_rx) = unbounded();
    let event_tx_clone = event_tx.clone();
    if !cfg.headless {
        std::thread::spawn(move || {
            let tick_rate = Duration::from_millis(1000);
            loop {
                if crossterm::event::poll(tick_rate).unwrap() {
                    let evt = crossterm::event::read().unwrap();
                    if event_tx.unbounded_send(InputEvent::UserInput(evt)).is_err() {
                        break;
                    }
                }
            }
        });
    }

    let cx = dom.base_scope();
    let rdom = Rc::new(RefCell::new(RealDom::new()));
    let taffy = Rc::new(RefCell::new(Taffy::new()));
    cx.provide_root_context(state);
    cx.provide_root_context(TuiContext { tx: event_tx_clone });
    cx.provide_root_context(Query {
        rdom: rdom.clone(),
        stretch: taffy.clone(),
    });

    {
        let mut rdom = rdom.borrow_mut();
        let mutations = dom.rebuild();
        let to_update = rdom.apply_mutations(vec![mutations]);
        let mut any_map = AnyMap::new();
        any_map.insert(taffy.clone());
        let _to_rerender = rdom.update_state(to_update, any_map);
    }

    render_vdom(
        &mut dom,
        event_rx,
        handler,
        cfg,
        rdom,
        taffy,
        register_event,
    )
    .unwrap();
}

fn render_vdom(
    vdom: &mut VirtualDom,
    mut event_reciever: UnboundedReceiver<InputEvent>,
    handler: RinkInputHandler,
    cfg: Config,
    rdom: Rc<RefCell<Dom>>,
    taffy: Rc<RefCell<Taffy>>,
    mut register_event: impl FnMut(crossterm::event::Event),
) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut terminal = (!cfg.headless).then(|| {
                enable_raw_mode().unwrap();
                let mut stdout = std::io::stdout();
                execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
                let backend = CrosstermBackend::new(io::stdout());
                Terminal::new(backend).unwrap()
            });
            if let Some(terminal) = &mut terminal {
                terminal.clear().unwrap();
            }

            let mut to_rerender: rustc_hash::FxHashSet<GlobalNodeId> =
                vec![GlobalNodeId::VNodeId(ElementId(0))]
                    .into_iter()
                    .collect();
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
                    fn resize(dims: Rect, taffy: &mut Taffy, rdom: &Dom) {
                        let width = dims.width;
                        let height = dims.height;
                        let root_node = rdom[ElementId(0)].state.layout.node.unwrap();

                        taffy
                            .compute_layout(
                                root_node,
                                Size {
                                    width: taffy::prelude::Number::Defined((width - 1) as f32),
                                    height: taffy::prelude::Number::Defined((height - 1) as f32),
                                },
                            )
                            .unwrap();
                    }
                    if let Some(terminal) = &mut terminal {
                        terminal.draw(|frame| {
                            let rdom = rdom.borrow();
                            // size is guaranteed to not change when rendering
                            resize(frame.size(), &mut taffy.borrow_mut(), &rdom);
                            let root = &rdom[ElementId(0)];
                            render::render_vnode(
                                frame,
                                &taffy.borrow(),
                                &rdom,
                                root,
                                cfg,
                                Point::zero(),
                            );
                        })?;
                    } else {
                        let rdom = rdom.borrow();
                        resize(
                            Rect {
                                x: 0,
                                y: 0,
                                width: 100,
                                height: 100,
                            },
                            &mut taffy.borrow_mut(),
                            &rdom,
                        );
                    }
                }

                use futures::future::{select, Either};
                {
                    let wait = vdom.wait_for_work();
                    pin_mut!(wait);

                    match select(wait, event_reciever.next()).await {
                        Either::Left((_a, _b)) => {
                            //
                        }
                        Either::Right((evt, _o)) => {
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
                        }
                    }
                }

                {
                    let evts = {
                        let mut rdom = rdom.borrow_mut();
                        handler.get_events(&taffy.borrow(), &mut rdom)
                    };
                    {
                        updated |= handler.state().focus_state.clean();
                    }
                    for e in evts {
                        vdom.handle_message(SchedulerMsg::Event(e));
                    }
                    let mut rdom = rdom.borrow_mut();
                    let mutations = vdom.work_with_deadline(|| false);
                    for m in &mutations {
                        handler.prune(m, &rdom);
                    }
                    // updates the dom's nodes
                    let to_update = rdom.apply_mutations(mutations);
                    // update the style and layout
                    let mut any_map = AnyMap::new();
                    any_map.insert(taffy.clone());
                    to_rerender = rdom.update_state(to_update, any_map);
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
enum InputEvent {
    UserInput(TermEvent),
    Close,
}
