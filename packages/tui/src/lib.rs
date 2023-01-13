use anyhow::Result;
use crossterm::{
    cursor::{MoveTo, RestorePosition, SavePosition, Show},
    event::{DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dioxus_core::*;
use dioxus_native_core::{real_dom::RealDom, FxDashSet, NodeId, NodeMask, SendAnyMap};
use focus::FocusState;
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    pin_mut, StreamExt,
};
use futures_channel::mpsc::unbounded;
use query::Query;
use std::rc::Rc;
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
use std::{io, time::Duration};
use taffy::Taffy;
pub use taffy::{geometry::Point, prelude::*};
use tokio::{select, sync::mpsc::unbounded_channel};
use tui::{backend::CrosstermBackend, layout::Rect, Terminal};

mod config;
mod focus;
mod hooks;
mod layout;
mod node;
pub mod prelude;
pub mod query;
mod render;
mod style;
mod style_attributes;
mod widget;
mod widgets;

pub use config::*;
pub use hooks::*;
pub(crate) use node::*;

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
    launch_cfg_with_props(app, (), cfg);
}

pub fn launch_cfg_with_props<Props: 'static>(app: Component<Props>, props: Props, cfg: Config) {
    let mut dom = VirtualDom::new_with_props(app, props);

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
    let taffy = Arc::new(Mutex::new(Taffy::new()));
    cx.provide_context(state);
    cx.provide_context(TuiContext { tx: event_tx_clone });
    cx.provide_context(Query {
        rdom: rdom.clone(),
        stretch: taffy.clone(),
    });

    {
        let mut rdom = rdom.borrow_mut();
        let mutations = dom.rebuild();
        let (to_update, _) = rdom.apply_mutations(mutations);
        let mut any_map = SendAnyMap::new();
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
    rdom: Rc<RefCell<TuiDom>>,
    taffy: Arc<Mutex<Taffy>>,
    mut register_event: impl FnMut(crossterm::event::Event),
) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            #[cfg(all(feature = "hot-reload", debug_assertions))]
            let mut hot_reload_rx = {
                let (hot_reload_tx, hot_reload_rx) =
                    unbounded_channel::<dioxus_hot_reload::HotReloadMsg>();
                dioxus_hot_reload::connect(move |msg| {
                    let _ = hot_reload_tx.send(msg);
                });
                hot_reload_rx
            };
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
            to_rerender.insert(NodeId(0));
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
                    fn resize(dims: Rect, taffy: &mut Taffy, rdom: &TuiDom) {
                        let width = screen_to_layout_space(dims.width);
                        let height = screen_to_layout_space(dims.height);
                        let root_node = rdom[NodeId(0)].state.layout.node.unwrap();

                        // the root node fills the entire area

                        let mut style = *taffy.style(root_node).unwrap();
                        style.size = Size {
                            width: Dimension::Points(width),
                            height: Dimension::Points(height),
                        };
                        taffy.set_style(root_node, style).unwrap();

                        let size = Size {
                            width: AvailableSpace::Definite(width),
                            height: AvailableSpace::Definite(height),
                        };
                        taffy.compute_layout(root_node, size).unwrap();
                    }
                    if let Some(terminal) = &mut terminal {
                        execute!(terminal.backend_mut(), SavePosition).unwrap();
                        terminal.draw(|frame| {
                            let rdom = rdom.borrow();
                            let mut taffy = taffy.lock().expect("taffy lock poisoned");
                            // size is guaranteed to not change when rendering
                            resize(frame.size(), &mut taffy, &rdom);
                            let root = &rdom[NodeId(0)];
                            render::render_vnode(frame, &taffy, &rdom, root, cfg, Point::ZERO);
                        })?;
                        execute!(terminal.backend_mut(), RestorePosition, Show).unwrap();
                    } else {
                        let rdom = rdom.borrow();
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

                let mut hot_reload_msg = None;
                {
                    let wait = vdom.wait_for_work();
                    #[cfg(all(feature = "hot-reload", debug_assertions))]
                    let hot_reload_wait = hot_reload_rx.recv();
                    #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
                    let hot_reload_wait = std::future::pending();

                    pin_mut!(wait);

                    select! {
                        _ = wait => {

                        },
                        evt = event_reciever.next() => {
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
                        Some(msg) = hot_reload_wait => {
                            hot_reload_msg = Some(msg);
                        }
                    }
                }

                // if we have a new template, replace the old one
                if let Some(msg) = hot_reload_msg {
                    match msg {
                        dioxus_hot_reload::HotReloadMsg::UpdateTemplate(template) => {
                            vdom.replace_template(template);
                        }
                        dioxus_hot_reload::HotReloadMsg::Shutdown => {
                            break;
                        }
                    }
                }

                {
                    let evts = {
                        let mut rdom = rdom.borrow_mut();
                        handler.get_events(&taffy.lock().expect("taffy lock poisoned"), &mut rdom)
                    };
                    {
                        updated |= handler.state().focus_state.clean();
                    }
                    for e in evts {
                        vdom.handle_event(e.name, e.data, e.id, e.bubbles)
                    }
                    let mut rdom = rdom.borrow_mut();
                    let mutations = vdom.render_immediate();
                    handler.prune(&mutations, &rdom);
                    // updates the dom's nodes
                    let (to_update, dirty) = rdom.apply_mutations(mutations);
                    // update the style and layout
                    let mut any_map = SendAnyMap::new();
                    any_map.insert(taffy.clone());
                    to_rerender = rdom.update_state(to_update, any_map);
                    for (id, mask) in dirty {
                        if mask.overlaps(&NodeMask::new().with_text()) {
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
enum InputEvent {
    UserInput(TermEvent),
    Close,
}
