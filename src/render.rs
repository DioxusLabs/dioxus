use anyhow::Result;
use crossterm::{
    event,
    event::{DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dioxus::core::*;
use std::{
    collections::HashMap,
    io::{self, Stdout},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use stretch2::{
    geometry::Point,
    prelude::{Layout, Size},
    style::Style as StretchStyle,
    Stretch,
};
use tui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::Rect,
    style::Style as TuiStyle,
    text::Span,
    widgets::{canvas::Label, Block, BorderType, Borders, Widget},
    Terminal,
};

use crate::TuiNode;

pub fn render_vnode<'a>(
    frame: &mut tui::Frame<CrosstermBackend<Stdout>>,
    layout: &Stretch,
    layouts: &mut HashMap<ElementId, TuiNode<'a>>,
    vdom: &'a VirtualDom,
    node: &'a VNode<'a>,
) {
    match node {
        VNode::Fragment(f) => {
            for child in f.children {
                render_vnode(frame, layout, layouts, vdom, child);
            }
            return;
        }

        VNode::Component(vcomp) => {
            let idx = vcomp.scope.get().unwrap();
            let new_node = vdom.get_scope(idx).unwrap().root_node();
            render_vnode(frame, layout, layouts, vdom, new_node);
            return;
        }

        VNode::Placeholder(_) => return,

        VNode::Element(_) | VNode::Text(_) => {}
    }

    let id = node.try_mounted_id().unwrap();
    let node = layouts.remove(&id).unwrap();

    let Layout { location, size, .. } = layout.layout(node.layout).unwrap();

    let Point { x, y } = location;
    let Size { width, height } = size;

    match node.node {
        VNode::Text(t) => {
            #[derive(Default)]
            struct Label<'a> {
                text: &'a str,
            }

            impl<'a> Widget for Label<'a> {
                fn render(self, area: Rect, buf: &mut Buffer) {
                    buf.set_string(area.left(), area.top(), self.text, TuiStyle::default());
                }
            }

            // let s = Span::raw(t.text);

            // Block::default().

            let label = Label { text: t.text };
            let area = Rect::new(*x as u16, *y as u16, *width as u16, *height as u16);

            // the renderer will panic if a node is rendered out of range even if the size is zero
            if area.width > 0 && area.height > 0 {
                frame.render_widget(label, area);
            }
        }
        VNode::Element(el) => {
            let block = Block::default().style(node.block_style);
            let area = Rect::new(*x as u16, *y as u16, *width as u16, *height as u16);

            // the renderer will panic if a node is rendered out of range even if the size is zero
            if area.width > 0 && area.height > 0 {
                frame.render_widget(block, area);
            }

            for el in el.children {
                render_vnode(frame, layout, layouts, vdom, el);
            }
        }
        VNode::Fragment(_) => todo!(),
        VNode::Component(_) => todo!(),
        VNode::Placeholder(_) => todo!(),
    }
}
