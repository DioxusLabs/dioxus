use std::{str::FromStr, sync::Arc};
use dioxus::prelude::*;
use dioxus_liveview::{Window, WindowEvent};
use std::sync::{Mutex, RwLock};
use super::HistoryProvider;
use crate::routable::Routable;

/// A [`HistoryProvider`] that evaluates history through JS.
pub struct LiveviewHistory<R: Routable> {
    eval_tx: tokio::sync::mpsc::UnboundedSender<Eval<R>>,
    eval_rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Eval<R>>>>,
    state: Arc<Mutex<State<R>>>,
    updater_callback: Arc<RwLock<Arc<dyn Fn() + Send + Sync>>>,
}

struct State<R: Routable> {
    current_route: R,
    can_go_back: bool,
    can_go_forward: bool,
}

enum Eval<R: Routable> {
    GoBack,
    GoForward,
    Push(R),
    Replace(R),
    External(String),
}

impl<R: Routable> Default for LiveviewHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        let (eval_tx, eval_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            eval_tx,
            eval_rx: Arc::new(Mutex::new(eval_rx)),
            state: Arc::new(Mutex::new(State {
                current_route: "/".parse().unwrap_or_else(|err| {
                    panic!("index route does not exist:\n{}\n use MemoryHistory::with_initial_path to set a custom path", err)
                }),
                can_go_back: false,
                can_go_forward: false,
            })),
            updater_callback: Arc::new(RwLock::new(Arc::new(|| { panic!("NOOOO!!!!") }))),
        }
    }
}

impl<R: Routable + std::fmt::Debug> LiveviewHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    /// TODO
    pub fn attach(&self, cx: Scope) {
        let create_eval = use_eval(cx);
        let _: &Coroutine<()> = use_coroutine(cx, |_| {
            let eval_rx = self.eval_rx.clone();
            to_owned![create_eval];
            async move {
                let mut eval_rx = eval_rx.lock().expect("poisoned mutex");
                loop {
                    let eval = eval_rx.recv().await.expect("sender to exist");
                    let _ = match eval {
                        Eval::GoBack => create_eval(r#"
                            history.back();
                        "#),
                        Eval::GoForward => create_eval(r#"
                            history.forward();
                        "#),
                        Eval::Push(route) => create_eval(&format!(r#"
                            history.pushState(null, "", "{route}");
                        "#)),
                        Eval::Replace(route) => create_eval(&format!(r#"
                            history.replaceState(null, "", "{route}");
                        "#)),
                        Eval::External(url) => create_eval(&format!(r#"
                            location.href = "{url}";
                        "#)),
                    };
                }
            }
        });

        let window = cx.consume_context::<Window>().unwrap();
        let _: &Coroutine<()> = use_coroutine(cx, |_| {
            let mut window_rx = window.subscribe();
            let updater = self.updater_callback.clone();
            let state = self.state.clone();
            async move {
                loop {
                    let window_event = window_rx.recv().await.expect("sender to exist");
                    match window_event {
                        WindowEvent::Load { location } => {
                            let Ok(route) = R::from_str(&location.path) else {
                                continue;
                            };
                            let mut state = state.lock().expect("poisoned mutex");
                            state.current_route = route;
                        },
                        WindowEvent::PopState { location, state: new_state } => {
                            let Ok(route) = R::from_str(&location.path) else {
                                continue;
                            };
                            let mut state = state.lock().expect("poisoned mutex");
                            state.current_route = route;
                        },
                    }
                    // Call the updater callback
                    (updater.read().unwrap())();
                }
            }
        });
    }
}

impl<R: Routable + std::fmt::Debug> HistoryProvider<R> for LiveviewHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display + std::fmt::Debug,
{
    fn go_back(&mut self) {
        let _ = self.eval_tx.send(Eval::GoBack);
    }

    fn go_forward(&mut self) {
        let _ = self.eval_tx.send(Eval::GoForward);
    }

    fn push(&mut self, new: R) {
        let _ = self.eval_tx.send(Eval::Push(new));
    }

    fn replace(&mut self, path: R) {
        let _ = self.eval_tx.send(Eval::Replace(path));
    }

    fn external(&mut self, url: String) -> bool {
        let _ = self.eval_tx.send(Eval::External(url));
        true
    }

    fn current_route(&self) -> R {
        let state = self.state.lock().expect("poisoned mutex");
        state.current_route.clone()
    }

    fn can_go_back(&self) -> bool {
        let state = self.state.lock().expect("poisoned mutex");
        state.can_go_back
    }

    fn can_go_forward(&self) -> bool {
        let state = self.state.lock().expect("poisoned mutex");
        state.can_go_forward
    }

    fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
        let mut updater_callback = self.updater_callback.write().unwrap();
        *updater_callback = callback;
    }
}
