use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dioxus::{core::exports::futures_channel::mpsc::unbounded, prelude::Props};
use dioxus::{core::*, prelude::*};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    future::Either,
    pin_mut, StreamExt,
};
use std::{
    cell::Cell,
    collections::HashMap,
    io,
    rc::Rc,
    time::{Duration, Instant},
};
use stretch2::{prelude::Size, Stretch};
use tui::{backend::CrosstermBackend, style::Style as TuiStyle, Terminal};

mod attributes;
mod hooks;
mod layout;
mod render;

pub use attributes::*;
pub use hooks::*;
pub use layout::*;
pub use render::*;

pub fn launch(app: Component<()>) {
    let mut dom = VirtualDom::new(app);
    let (tx, rx) = unbounded();

    let cx = dom.base_scope();

    cx.provide_root_context(RinkContext::new(rx, cx));

    dom.rebuild();

    render_vdom(&mut dom, tx).unwrap();
}

pub struct TuiNode<'a> {
    pub layout: stretch2::node::Node,
    pub block_style: TuiStyle,
    pub node: &'a VNode<'a>,
}

pub fn render_vdom(vdom: &mut VirtualDom, ctx: UnboundedSender<TermEvent>) -> Result<()> {
    // Setup input handling
    let (tx, mut rx) = unbounded();
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
                tx.unbounded_send(InputEvent::UserInput(evt)).unwrap();
            }

            if last_tick.elapsed() >= tick_rate {
                tx.unbounded_send(InputEvent::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            /*
            Get the terminal to calcualte the layout from
            */
            enable_raw_mode().unwrap();
            let mut stdout = std::io::stdout();
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
            let backend = CrosstermBackend::new(io::stdout());
            let mut terminal = Terminal::new(backend).unwrap();

            terminal.clear().unwrap();

            loop {
                let dims = terminal.size().unwrap();
                let width = dims.width;
                let height = dims.height;

                /*
                -> collect all the nodes with their layout
                -> solve their layout
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
                Compute the layout given th terminal size
                */
                let node_id = root_node.try_mounted_id().unwrap();
                let root_layout = nodes[&node_id].layout;
                layout.compute_layout(
                    root_layout,
                    Size {
                        width: stretch2::prelude::Number::Defined(width as f32),
                        height: stretch2::prelude::Number::Defined(height as f32),
                    },
                )?;

                terminal.draw(|frame| {
                    //
                    render::render_vnode(frame, &layout, &mut nodes, vdom, root_node);
                    assert!(nodes.is_empty());
                })?;

                use futures::future::{select, Either};
                {
                    let wait = vdom.wait_for_work();
                    pin_mut!(wait);

                    match select(wait, rx.next()).await {
                        Either::Left((a, b)) => {
                            //
                        }
                        Either::Right((evt, o)) => {
                            //
                            match evt.as_ref().unwrap() {
                                InputEvent::UserInput(event) => match event {
                                    TermEvent::Key(key) => {
                                        match key.code {
                                            KeyCode::Char('q') => break,
                                            _ => {} // handle event
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

            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            Ok(())
        })
}

enum InputEvent {
    UserInput(TermEvent),
    Close,
    Tick,
}
