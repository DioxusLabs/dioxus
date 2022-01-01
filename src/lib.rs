use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dioxus::core::*;
use std::{collections::HashMap, io};
use stretch2::{prelude::Size, Stretch};
use tui::{backend::CrosstermBackend, style::Style as TuiStyle, Terminal};

mod attributes;
mod layout;
mod render;

pub use attributes::*;
pub use layout::*;
pub use render::*;

pub struct TuiNode<'a> {
    pub layout: stretch2::node::Node,
    pub block_style: TuiStyle,
    pub node: &'a VNode<'a>,
}

pub fn render_vdom(vdom: &VirtualDom) -> Result<()> {
    /*
     -> collect all the nodes with their layout
     -> solve their layout
     -> render the nodes in the right place with tui/crosstream
     -> while rendering, apply styling

     use simd to compare lines for diffing?

    */
    let mut layout = Stretch::new();
    let mut nodes = HashMap::new();

    let root_node = vdom.base_scope().root_node();
    layout::collect_layout(&mut layout, &mut nodes, vdom, root_node);

    /*
    Get the terminal to calcualte the layout from
    */
    enable_raw_mode().unwrap();
    ctrlc::set_handler(move || {
        disable_raw_mode().unwrap();
    })
    .expect("Error setting Ctrl-C handler");
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    let dims = terminal.size().unwrap();
    let width = dims.width;
    let height = dims.height;

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

    std::thread::sleep(std::time::Duration::from_millis(5000));

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

// enum InputEvent {
//     UserInput(KeyEvent),
//     Close,
//     Tick,
// }

// // Setup input handling
// let (tx, rx) = mpsc::channel();
// let tick_rate = Duration::from_millis(100);
// thread::spawn(move || {
//     let mut last_tick = Instant::now();
//     loop {
//         // poll for tick rate duration, if no events, sent tick event.
//         let timeout = tick_rate
//             .checked_sub(last_tick.elapsed())
//             .unwrap_or_else(|| Duration::from_secs(0));

//         if event::poll(timeout).unwrap() {
//             if let TermEvent::Key(key) = event::read().unwrap() {
//                 tx.send(InputEvent::UserInput(key)).unwrap();
//             }
//         }

//         // if last_tick.elapsed() >= tick_rate {
//         //     tx.send(InputEvent::Tick).unwrap();
//         //     last_tick = Instant::now();
//         // }
//     }
// });

// terminal.clear()?;

// // loop {
// terminal.draw(|frame| {
//     // draw the frame
//     let size = frame.size();
//     let root_node = vdom.base_scope().root_node();
//     let block = render_vnode(frame, vdom, root_node);

//     frame.render_widget(block, size);
// })?;

// // // terminal.draw(|f| ui::draw(f, &mut app))?;
// match rx.recv()? {
//     InputEvent::UserInput(event) => match event.code {
//         KeyCode::Char('q') => {
//             disable_raw_mode()?;
//             execute!(
//                 terminal.backend_mut(),
//                 LeaveAlternateScreen,
//                 DisableMouseCapture
//             )?;
//             terminal.show_cursor()?;
//             // break;
//         }
//         _ => {} // handle event
//     },
//     InputEvent::Tick => {} // tick
//     InputEvent::Close => {
//         // break;
//     }
// };
