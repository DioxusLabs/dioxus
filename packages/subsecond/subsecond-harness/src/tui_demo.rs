use color_eyre::Result;
use rand::{rng, Rng};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Bar, BarChart, BarGroup},
    DefaultTerminal, Frame,
};
use std::time::Duration;

pub fn launch() {
    color_eyre::install().unwrap();
    let mut terminal = ratatui::init();
    let app_result = subsecond::call(|| App::new().run(&mut terminal));
    ratatui::restore();
    app_result.unwrap();
}

struct App {
    should_exit: bool,
    temperatures: Vec<u8>,
}

impl App {
    fn new() -> Self {
        let mut rng = rand::rng();
        let temperatures = (0..24).map(|_| rng.random_range(50..90)).collect();

        Self {
            should_exit: false,
            temperatures,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_exit {
            subsecond::call(|| self.tick(terminal))?;
        }
        Ok(())
    }

    fn tick(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        terminal.draw(|frame| self.draw(frame))?;
        self.handle_events()?;
        Ok(())
    }

    // wait 100ms for an event to occur
    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    self.should_exit = true;
                }

                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('t') {
                    let mut rng = rng();
                    self.temperatures = (0..24).map(|_| rng.random_range(50..90)).collect();
                }
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let [title, main] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)])
            .spacing(1)
            .areas(frame.area());

        frame.render_widget(
            "Tui development has never been so easy!"
                .bold()
                .italic()
                .into_centered_line()
                .centered(),
            title,
        );
        frame.render_widget(vertical_barchart(&self.temperatures), main);
    }
}

/// Create a vertical bar chart from the temperatures data.
fn vertical_barchart(temperatures: &[u8]) -> BarChart {
    let bars: Vec<Bar> = temperatures
        .iter()
        .enumerate()
        .map(|(hour, value)| vertical_bar(hour, value))
        .collect();
    BarChart::default()
        .data(BarGroup::default().bars(&bars))
        .bar_width(5)
}

fn vertical_bar(hour: usize, temperature: &u8) -> Bar {
    Bar::default()
        .value(u64::from(*temperature))
        .label(Line::from(format!("{hour:>02}:00")))
        .text_value(format!("{temperature:>3}°"))
        .style(temperature_style(*temperature))
        .value_style(temperature_style(*temperature).reversed())
}

/// create a yellow to red value based on the value (50-90)
fn temperature_style(value: u8) -> Style {
    let green = (255.0 * (1.0 - f64::from(value - 50) / 40.0)) as u8;
    let color = Color::Rgb(255, green, 0);
    Style::new().fg(color)
}
