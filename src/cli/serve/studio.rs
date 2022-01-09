//! It's better to store all the configuration in one spot

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::canvas::{Canvas, Line, Map, MapResolution, Rectangle},
    widgets::{
        Axis, BarChart, Block, BorderType, Borders, Cell, Chart, Dataset, Gauge, LineGauge, List,
        ListItem, Paragraph, Row, Sparkline, Table, Tabs, Wrap,
    },
    Frame, Terminal,
};

use crate::*;
use std::{any::Any, io::Write, path::PathBuf, process::Command};

pub struct Cfg {
    command: LaunchOptions,
    headless: bool,
    example: Option<String>,
    outdir: Option<String>,
    release: bool,
    hydrate: Option<String>,
    template: Option<String>,
    translate_file: Option<String>,
    crate_config: Option<CrateConfig>,
    should_quit: bool,
}

pub async fn start(options: DevelopOptions) -> Result<()> {
    let mut state = Cfg {
        command: todo!(),
        headless: todo!(),
        example: todo!(),
        outdir: todo!(),
        release: todo!(),
        hydrate: todo!(),
        template: todo!(),
        translate_file: todo!(),
        crate_config: todo!(),
        should_quit: false,
    };

    crossterm::terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    // Setup input handling
    // let (tx, rx) = futures::channel::mpsc::unbounded();
    let tick_rate = std::time::Duration::from_millis(100);

    let mut prev_time = std::time::Instant::now();
    while !state.should_quit {
        let next_time = prev_time + tick_rate;
        let now = std::time::Instant::now();

        let diff = next_time - std::time::Instant::now();
    }

    Ok(())
}

struct TuiStudio {
    cfg: Cfg,
    hook_idx: usize,
    hooks: Vec<Box<dyn Any>>,
}
impl TuiStudio {
    fn use_hook<F: 'static>(&mut self, f: impl FnOnce() -> F) -> &mut F {
        if self.hook_idx == self.hooks.len() {
            self.hooks.push(Box::new(f()));
        }
        let idx = self.hook_idx;
        self.hook_idx += 1;
        let hook = self.hooks.get_mut(idx).unwrap();
        let r = hook.downcast_mut::<F>().unwrap();
        r
    }
}

impl TuiStudio {
    fn event_handler(&self, action: crossterm::event::Event) -> anyhow::Result<()> {
        match action {
            crossterm::event::Event::Key(_) => {}
            crossterm::event::Event::Mouse(_) => {}
            crossterm::event::Event::Resize(_, _) => {}
        }
        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {}

    fn tick(&mut self) {}

    fn should_quit(&self) -> bool {
        false
    }

    fn render<B: tui::backend::Backend>(&mut self, f: &mut tui::Frame<B>) {
        self.hook_idx = 0;

        // Wrapping block for a group
        // Just draw the block and the group on the same area and build the group
        // with at least a margin of 1
        let size = f.size();
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Main block with round corners")
            .border_type(BorderType::Rounded);
        f.render_widget(block, size);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(4)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);
        let block = Block::default()
            .title(vec![
                Span::styled("With", Style::default().fg(Color::Yellow)),
                Span::from(" background"),
            ])
            .style(Style::default().bg(Color::Green));
        f.render_widget(block, top_chunks[0]);

        let block = Block::default().title(Span::styled(
            "Styled title",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ));
        f.render_widget(block, top_chunks[1]);

        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[1]);
        let block = Block::default().title("With borders").borders(Borders::ALL);
        f.render_widget(block, bottom_chunks[0]);
        let block = Block::default()
            .title("With styled borders and doubled borders")
            .border_style(Style::default().fg(Color::Cyan))
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Double);
        f.render_widget(block, bottom_chunks[1]);
    }
}

impl TuiStudio {
    fn render_list<B: tui::backend::Backend>(&mut self, f: &mut tui::Frame<B>) {
        let options = [
            "Bundle", "Develop",
            //
        ];
    }
}
