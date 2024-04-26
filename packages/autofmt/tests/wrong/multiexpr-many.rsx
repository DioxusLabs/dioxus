#![allow(dead_code, unused)]
use dioxus::desktop::use_window;
use dioxus::prelude::*;
use std::{
    process::exit,
    time::{Duration, Instant},
};
use tokio::time::sleep;

fn main() {
    LaunchBuilder::desktop().launch(app);
}

struct WindowPreferences {
    always_on_top: bool,
    with_decorations: bool,
    exiting: Option<Instant>,
}

impl Default for WindowPreferences {
    fn default() -> Self {
        Self {
            with_decorations: true,
            always_on_top: false,
            exiting: None,
        }
    }
}

impl WindowPreferences {
    fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
struct Timer {
    hours: u8,
    minutes: u8,
    seconds: u8,
    started_at: Option<Instant>,
}

impl Timer {
    fn new() -> Self {
        Self::default()
    }

    fn duration(&self) -> Duration {
        Duration::from_secs(
            (self.hours as u64 * 60 + self.minutes as u64) * 60 + self.seconds as u64,
        )
    }
}

const UPD_FREQ: Duration = Duration::from_millis(100);

fn exit_button(
    delay: Duration,
    label: fn(Signal<Option<Instant>>, Duration) -> Option<VNode>,
) -> Element {
    let mut trigger: Signal<Option<Instant>> = use_signal(|| None);
    use_future(move || async move {
        loop {
            sleep(UPD_FREQ).await;
            if let Some(true) = trigger.read().map(|e| e.elapsed() > delay) {
                exit(0);
            }
        }
    });
    let stuff: Option<VNode> = rsx! {
        button {
            onmouseup: move |_| {
                trigger.set(None);
            },
            onmousedown: move |_| {
                trigger.set(Some(Instant::now()));
            },
            width: 100,
            {label(trigger, delay)}
        }
    };
    stuff
}

fn app() -> Element {
    let mut timer = use_signal(Timer::new);
    let mut window_preferences = use_signal(WindowPreferences::new);

    use_future(move || async move {
        loop {
            sleep(UPD_FREQ).await;
            timer.with_mut(|t| {
                if let Some(started_at) = t.started_at {
                    if t.duration().saturating_sub(started_at.elapsed()) == Duration::ZERO {
                        t.started_at = None;
                    }
                }
            });
        }
    });

    rsx! {
        div {
            {
                let millis = timer.with(|t| t.duration().saturating_sub(t.started_at.map(|x| x.elapsed()).unwrap_or(Duration::ZERO)).as_millis());
                format!("{:02}:{:02}:{:02}.{:01}",
                        millis / 1000 / 3600 % 3600,
                        millis / 1000 / 60 % 60,
                        millis / 1000 % 60,
                        millis / 100 % 10)
            }
        }
        div {
            input {
                r#type: "number",
                min: 0,
                max: 99,
                value: format!("{:02}", timer.read().hours),
                oninput: move |e| {
                    timer.write().hours = e.value().parse().unwrap_or(0);
                }
            }

            input {
                r#type: "number",
                min: 0,
                max: 59,
                value: format!("{:02}", timer.read().minutes),
                oninput: move |e| {
                    timer.write().minutes = e.value().parse().unwrap_or(0);
                }
            }

            input {
                r#type: "number",
                min: 0,
                max: 59,
                value: format!("{:02}", timer.read().seconds),
                oninput: move |e| {
                    timer.write().seconds = e.value().parse().unwrap_or(0);
                }
            }
        }

        button {
            id: "start_stop",
            onclick: move |_| {
                timer
                    .with_mut(|t| {
                        t.started_at = if t.started_at.is_none() {
                            Some(Instant::now())
                        } else {
                            None
                        };
                    })
            },
            { timer.with(|t| if t.started_at.is_none() { "Start" } else { "Stop" }) }
        }
        div { id: "app",
            button {
                onclick: move |_| {
                    let decorations = window_preferences.read().with_decorations;
                    use_window().set_decorations(!decorations);
                    window_preferences.write().with_decorations = !decorations;
                },
                {
                    format!("with decorations{}", if window_preferences.read().with_decorations { " ✓" } else { "" }).to_string()
                }
            }
            button {
                onclick: move |_| {
                    window_preferences
                        .with_mut(|wp| {
                            use_window().set_always_on_top(!wp.always_on_top);
                            wp.always_on_top = !wp.always_on_top;
                        })
                },
                width: 100,
                {
                    format!("always on top{}", if window_preferences.read().always_on_top { " ✓" } else { "" })
                }
            }
        }
        {
            exit_button(
                Duration::from_secs(3),
                |trigger, delay| rsx! {
                    {format!("{:0.1?}", trigger.read().map(|inst| (delay.as_secs_f32() - inst.elapsed().as_secs_f32()))) }
                }
            )
        }
    }
}
