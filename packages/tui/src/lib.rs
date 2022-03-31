// notes:
// mouse events binary search was broken for absolutely positioned elements

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dioxus_core::exports::futures_channel::mpsc::unbounded;
use dioxus_core::*;
use dioxus_native_core::real_dom::RealDom;
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    pin_mut, StreamExt,
};
use layout::StretchLayout;
use std::{io, time::Duration};
use stretch2::{prelude::Size, Stretch};
use style_attributes::StyleModifier;
use tui::{backend::CrosstermBackend, Terminal};

mod config;
mod hooks;
mod layout;
mod render;
mod style;
mod style_attributes;
mod widget;

pub use config::*;
pub use hooks::*;
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

    let cx = dom.base_scope();

    let (handler, state) = RinkInputHandler::new(rx, cx);

    // Setup input handling
    let (event_tx, event_rx) = unbounded();
    let event_tx_clone = event_tx.clone();
    std::thread::spawn(move || {
        let tick_rate = Duration::from_millis(1000);
        loop {
            if crossterm::event::poll(tick_rate).unwrap() {
                // if crossterm::event::poll(timeout).unwrap() {
                let evt = crossterm::event::read().unwrap();
                if event_tx.unbounded_send(InputEvent::UserInput(evt)).is_err() {
                    break;
                }
            }
        }
    });

    cx.provide_root_context(state);
    cx.provide_root_context(TuiContext { tx: event_tx_clone });

    let mut tree: RealDom<StretchLayout, StyleModifier> = RealDom::new();
    let mutations = dom.rebuild();
    let to_update = tree.apply_mutations(vec![mutations]);
    let mut stretch = Stretch::new();
    let _to_rerender = tree
        .update_state(&dom, to_update, &mut stretch, &mut ())
        .unwrap();

    render_vdom(&mut dom, tx, event_rx, handler, cfg, tree, stretch).unwrap();
}

fn render_vdom(
    vdom: &mut VirtualDom,
    crossterm_event_sender: UnboundedSender<TermEvent>,
    mut event_reciever: UnboundedReceiver<InputEvent>,
    handler: RinkInputHandler,
    cfg: Config,
    mut tree: RealDom<StretchLayout, StyleModifier>,
    mut stretch: Stretch,
) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            enable_raw_mode().unwrap();
            let mut stdout = std::io::stdout();
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
            let backend = CrosstermBackend::new(io::stdout());
            let mut terminal = Terminal::new(backend).unwrap();

            terminal.clear().unwrap();
            let mut to_rerender: fxhash::FxHashSet<usize> = vec![0].into_iter().collect();
            let mut redraw = true;

            loop {
                /*
                -> render the nodes in the right place with tui/crossterm
                -> wait for changes
                -> resolve events
                -> lazily update the layout and style based on nodes changed

                use simd to compare lines for diffing?

                todo: lazy re-rendering
                */

                if !to_rerender.is_empty() || redraw {
                    redraw = false;
                    terminal.draw(|frame| {
                        // size is guaranteed to not change when rendering
                        let dims = frame.size();
                        let width = dims.width;
                        let height = dims.height;
                        let root_id = tree.root_id();
                        let root_node = tree[root_id].up_state.node.unwrap();

                        stretch
                            .compute_layout(
                                root_node,
                                Size {
                                    width: stretch2::prelude::Number::Defined((width - 1) as f32),
                                    height: stretch2::prelude::Number::Defined((height - 1) as f32),
                                },
                            )
                            .unwrap();
                        let root = &tree[tree.root_id()];
                        render::render_vnode(frame, &stretch, &tree, &root, cfg);
                    })?;
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
                                        {
                                            break;
                                        }
                                    }
                                    TermEvent::Resize(_, _) => redraw = true,
                                    TermEvent::Mouse(_) => {}
                                },
                                InputEvent::Close => break,
                            };

                            if let InputEvent::UserInput(evt) = evt.unwrap() {
                                crossterm_event_sender.unbounded_send(evt).unwrap();
                            }
                        }
                    }
                }

                // resolve events before rendering
                for e in handler.get_events(&stretch, &mut tree) {
                    vdom.handle_message(SchedulerMsg::Event(e));
                }
                vdom.process_all_messages();
                let mutations = vdom.work_with_deadline(|| false);
                // updates the tree's nodes
                let to_update = tree.apply_mutations(mutations);
                // update the style and layout
                to_rerender = tree
                    .update_state(&vdom, to_update, &mut stretch, &mut ())
                    .unwrap();
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
    #[allow(dead_code)]
    Close,
}
