use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dioxus_core::exports::futures_channel::mpsc::unbounded;
use dioxus_core::*;
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    pin_mut, StreamExt,
};
use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
};
use stretch2::{
    prelude::{Node, Size},
    Stretch,
};
use style::RinkStyle;
use tui::{backend::CrosstermBackend, Terminal};

mod attributes;
mod config;
mod hooks;
mod layout;
mod render;
mod style;
mod widget;

pub use attributes::*;
pub use config::*;
pub use hooks::*;
pub use layout::*;
pub use render::*;

#[derive(Clone)]
pub struct TuiContext {
    tx: UnboundedSender<InputEvent>,
}
impl TuiContext {
    pub fn quit(&self) {
        self.tx.unbounded_send(InputEvent::Close).unwrap();
    }
}

pub fn launch(app: Component<()>) {
    launch_cfg(app, Config::default())
}

pub fn launch_cfg(app: Component<()>, cfg: Config) {
    let mut dom = VirtualDom::new(app);
    let (tx, rx) = unbounded();
    // Setup input handling
    let (event_tx, event_rx) = unbounded();
    let event_tx_clone = event_tx.clone();
    if !cfg.headless {
        std::thread::spawn(move || {
            let tick_rate = Duration::from_millis(100);
            let mut last_tick = Instant::now();
            loop {
                // poll for tick rate duration, if no events, sent tick event.
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if crossterm::event::poll(timeout).unwrap() {
                    let evt = crossterm::event::read().unwrap();
                    event_tx.unbounded_send(InputEvent::UserInput(evt)).unwrap();
                }

                if last_tick.elapsed() >= tick_rate {
                    event_tx.unbounded_send(InputEvent::Tick).unwrap();
                    last_tick = Instant::now();
                }
            }
        });
    }

    let cx = dom.base_scope();
    cx.provide_root_context(TuiContext { tx: event_tx_clone });

    let (handler, state) = RinkInputHandler::new(rx, cx);

    cx.provide_root_context(state);

    dom.rebuild();

    render_vdom(&mut dom, event_rx, tx, handler, cfg).unwrap();
}

pub struct TuiNode<'a> {
    pub layout: stretch2::node::Node,
    pub block_style: RinkStyle,
    pub tui_modifier: TuiModifier,
    pub node: &'a VNode<'a>,
}

fn render_vdom(
    vdom: &mut VirtualDom,
    mut event_reciever: UnboundedReceiver<InputEvent>,
    ctx: UnboundedSender<TermEvent>,
    handler: RinkInputHandler,
    cfg: Config,
) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            /*
            Get the terminal to calcualte the layout from
            */
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

            loop {
                /*
                -> collect all the nodes with their layout
                -> solve their layout
                -> resolve events
                -> render the nodes in the right place with tui/crosstream
                -> while rendering, apply styling

                use simd to compare lines for diffing?


                todo: reuse the layout and node objects.
                our work_with_deadline method can tell us which nodes are dirty.
                */
                let mut layout = Stretch::new();
                let mut nodes = HashMap::new();

                let root_node = vdom.base_scope().root_node();
                layout::collect_layout(&mut layout, &mut nodes, vdom, root_node);
                /*
                Compute the layout given the terminal size
                */
                let node_id = root_node.try_mounted_id().unwrap();
                let root_layout = nodes[&node_id].layout;
                let mut events = Vec::new();

                fn resize(dims: tui::layout::Rect, stretch: &mut Stretch, root_layout: Node) {
                    let width = dims.width;
                    let height = dims.height;

                    stretch
                        .compute_layout(
                            root_layout,
                            Size {
                                width: stretch2::prelude::Number::Defined((width - 1) as f32),
                                height: stretch2::prelude::Number::Defined((height - 1) as f32),
                            },
                        )
                        .unwrap();
                }

                if let Some(terminal) = &mut terminal {
                    terminal.draw(|frame| {
                        // size is guaranteed to not change when rendering
                        resize(frame.size(), &mut layout, root_layout);

                        // resolve events before rendering
                        events = handler.get_events(vdom, &layout, &mut nodes, root_node);
                        render::render_vnode(
                            frame,
                            &layout,
                            &mut nodes,
                            vdom,
                            root_node,
                            &RinkStyle::default(),
                            cfg,
                        );
                        assert!(nodes.is_empty());
                    })?;
                } else {
                    resize(
                        tui::layout::Rect {
                            x: 0,
                            y: 0,
                            width: 100,
                            height: 100,
                        },
                        &mut layout,
                        root_layout,
                    );
                }

                for e in events {
                    vdom.handle_message(SchedulerMsg::Event(e));
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
                                    TermEvent::Resize(_, _) | TermEvent::Mouse(_) => {}
                                },
                                InputEvent::Tick => {} // tick
                                InputEvent::Close => break,
                            };

                            if let InputEvent::UserInput(evt) = evt.unwrap() {
                                ctx.unbounded_send(evt).unwrap();
                            }
                        }
                    }
                }

                vdom.work_with_deadline(|| false);
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

enum InputEvent {
    UserInput(TermEvent),
    Tick,

    #[allow(dead_code)]
    Close,
}
