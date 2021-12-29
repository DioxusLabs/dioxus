//! It's better to store all the configuration in one spot
//!
use tui_template::tuiapp::TuiApp;

use crate::*;
use std::{any::Any, io::Write, path::PathBuf, process::Command};

pub struct Studio {
    command: LaunchOptions,
    headless: bool,
    example: Option<String>,
    outdir: Option<String>,
    release: bool,
    hydrate: Option<String>,
    template: Option<String>,
    translate_file: Option<String>,
    crate_config: Option<CrateConfig>,
}

impl Studio {
    pub fn new(command: LaunchOptions) -> Self {
        let headless = true;
        let release = false;
        let example = None;
        let outdir = None;
        let hydrate = None;
        let template = None;
        let translate_file = None;
        let crate_config = None;

        match command.command {
            LaunchCommand::Translate(_) => todo!(),
            LaunchCommand::Develop(_) => todo!(),
            LaunchCommand::Build(_) => todo!(),
            LaunchCommand::Test(_) => todo!(),
            LaunchCommand::Publish(_) => todo!(),
            LaunchCommand::Studio(StudioOptions { .. }) => {
                //
            }
        };

        Self {
            command,
            headless,
            example,
            outdir,
            release,
            hydrate,
            template,
            translate_file,
            crate_config,
        }
    }

    pub async fn start(self) -> Result<()> {
        match self.command.command {
            LaunchCommand::Develop(_) => todo!(),
            LaunchCommand::Build(_) => todo!(),
            LaunchCommand::Translate(_) => todo!(),
            LaunchCommand::Test(_) => todo!(),
            LaunchCommand::Publish(_) => todo!(),
            LaunchCommand::Studio(_) => self.launch_studio().await?,
        }
        Ok(())
    }

    pub async fn launch_studio(mut self) -> Result<()> {
        let task = async_std::task::spawn_blocking(|| async move {
            let mut app = TuiStudio {
                cfg: self,
                hooks: vec![],
                hook_idx: 0,
            };
            app.launch(250).expect("tui app crashed :(");
        });
        let r = task.await.await;

        Ok(())
    }
}

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::canvas::{Canvas, Line, Map, MapResolution, Rectangle},
    widgets::{
        Axis, BarChart, Block, BorderType, Borders, Cell, Chart, Dataset, Gauge, LineGauge, List,
        ListItem, Paragraph, Row, Sparkline, Table, Tabs, Wrap,
    },
    Frame,
};

struct TuiStudio {
    cfg: Studio,

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

impl TuiApp for TuiStudio {
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
