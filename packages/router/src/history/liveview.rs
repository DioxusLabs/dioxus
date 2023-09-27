use std::{str::FromStr, rc::Rc, sync::Arc};
use dioxus::prelude::*;
use dioxus_liveview::{Window, WindowEvent};
use std::{fmt::Display, sync::{Mutex, RwLock}};
use super::HistoryProvider;
use crate::routable::Routable;

/// A [`HistoryProvider`] that evaluates history through JS.
pub struct LiveviewHistory<R: Routable> {
    action_tx: tokio::sync::mpsc::UnboundedSender<Action<R>>,
    timeline: Arc<Mutex<Timeline<R>>>,
    updater_callback: Arc<RwLock<Arc<dyn Fn() + Send + Sync>>>,
}

struct Timeline<R: Routable> {
    current_route: R,
    history: Vec<R>,
    future: Vec<R>,
}

struct State {
    index: Option<usize>,
}

enum Action<R: Routable> {
    GoBack,
    GoForward,
    Push(R),
    Replace(R),
    External(String),
}

impl<R: Routable> Timeline<R> {
    fn init(
        &mut self,
        route: R,
    ) -> State {
        self.current_route = route;
        State::from_index(0)
    }

    fn update(
        &mut self,
        route: R,
        state: State,
    ) -> State {
        if let Some(index) = state.index {
            // already visited position, shuffle history and future
            let current_index = self.history.len();
            if index > current_index {
                self.history.push(self.current_route.clone());
                for _ in index..current_index {
                    self.history.push(self.future.remove(0));
                }
            } else {
                self.future.push(self.current_route.clone());
                for _ in index..current_index {
                    self.future.push(self.history.remove(0));
                }
            }
            self.current_route = route;
            state
        } else {
            self.push(route)
        }
    }

    fn push(
        &mut self,
        route: R,
    ) -> State {
        // top of stack
        let current_route = self.current_route.clone();
        self.current_route = route;
        self.history.push(current_route);
        self.future.clear();
        State::from_index(self.history.len())
    }

    fn replace(
        &mut self,
        route: R,
    ) -> State {
        self.current_route = route;
        State::from_index(self.history.len())
    }
}

impl State {
    fn from_index(index: usize) -> Self {
        Self {
            index: Some(index),
        }
    }
}

impl From<&str> for State {
    fn from(value: &str) -> Self {
        Self {
            index: match value.parse::<usize>() {
                Ok(index) => Some(index),
                Err(_) => None,
            }
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.index {
            Some(index) => write!(f, r#"
                {index}
            "#),
            None => write!(f, r#"
                null
            "#),
        }
    }
}

impl<R: Routable + std::fmt::Debug> LiveviewHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    /// Create a [`LiveviewHistory`] in the given scope.
    ///
    /// # Panics
    ///
    /// Panics if not in a Liveview context.
    pub fn new(cx: Scope) -> Self {
        Self::new_with_initial_path(
            cx,
            "/".parse().unwrap_or_else(|err| {
                panic!("index route does not exist:\n{}\n use LiveviewHistory::new_with_initial_path to set a custom path", err)
            }),
        )
    }

    /// Create a [`LiveviewHistory`] in the given scope, starting at `initial_path`.
    ///
    /// # Panics
    ///
    /// Panics if not in a Liveview context.
    pub fn new_with_initial_path(cx: Scope, initial_path: R) -> Self {
        let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel::<Action<R>>();
        let action_rx = Arc::new(Mutex::new(action_rx));
        let timeline = Arc::new(Mutex::new(Timeline {
            current_route: initial_path,
            history: Vec::new(),
            future: Vec::new(),
        }));
        let updater_callback: Arc<RwLock<Arc<dyn Fn() + Send + Sync>>> = Arc::new(RwLock::new(Arc::new(|| {})));

        let eval_provider = cx
            .consume_context::<Rc<dyn EvalProvider>>()
            .expect("evaluator not provided");

        let create_eval = Rc::new(move |script: &str| {
            eval_provider
                .new_evaluator(script.to_string())
                .map(UseEval::new)
        }) as Rc<dyn Fn(&str) -> Result<UseEval, EvalError>>;

        // Listen to server actions
        cx.push_future({
            let timeline = timeline.clone();
            let action_rx = action_rx.clone();
            let create_eval = create_eval.clone();
            async move {
                let mut action_rx = action_rx.lock().expect("poisoned mutex");
                loop {
                    let eval = action_rx.recv().await.expect("sender to exist");
                    let _ = match eval {
                        Action::GoBack => {
                            create_eval(r#"
                                // this triggers a PopState event
                                history.back();
                            "#)
                        },
                        Action::GoForward => {
                            create_eval(r#"
                                // this triggers a PopState event
                                history.forward();
                            "#)
                        },
                        Action::Push(route) => {
                            let mut timeline = timeline.lock().expect("poisoned mutex");
                            let state = timeline.push(route.clone());
                            create_eval(&format!(r#"
                                // this does not trigger a PopState event
                                history.pushState({state}, "", "{route}");
                            "#))
                        },
                        Action::Replace(route) => {
                            let mut timeline = timeline.lock().expect("poisoned mutex");
                            let state = timeline.replace(route.clone());
                            create_eval(&format!(r#"
                                // this does not trigger a PopState event
                                history.replaceState({state}, "", "{route}");
                            "#))
                        },
                        Action::External(url) => {
                            create_eval(&format!(r#"
                                location.href = "{url}";
                            "#))
                        },
                    };
                }
            }
        });

        // Listen to browser actions
        let window = cx.consume_context::<Window>().unwrap();
        cx.push_future({
            let mut window_rx = window.subscribe();
            let updater = updater_callback.clone();
            let timeline = timeline.clone();
            let create_eval = create_eval.clone();
            async move {
                loop {
                    let window_event = window_rx.recv().await.expect("sender to exist");
                    match window_event {
                        WindowEvent::Load { location, state: _ } => {
                            let Ok(route) = R::from_str(&location.path) else {
                                return;
                            };
                            let mut timeline = timeline.lock().expect("poisoned mutex");
                            let state = timeline.init(route.clone());
                            let _ = create_eval(&format!(r#"
                                // this does not trigger a PopState event
                                history.replaceState({state}, "", "{route}");
                            "#));
                        },
                        WindowEvent::PopState { location, state } => {
                            let Ok(route) = R::from_str(&location.path) else {
                                return;
                            };
                            let mut timeline = timeline.lock().expect("poisoned mutex");
                            let state = timeline.update(route.clone(), state.as_str().into());
                            let _ = create_eval(&format!(r#"
                                // this does not trigger a PopState event
                                history.replaceState({state}, "", "{route}");
                            "#));
                        },
                    }
                    // Call the updater callback
                    (updater.read().unwrap())();
                }
            }
        });
        
        Self {
            action_tx,
            timeline,
            updater_callback,
        }
    }
}

impl<R: Routable + std::fmt::Debug> HistoryProvider<R> for LiveviewHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display + std::fmt::Debug,
{
    fn go_back(&mut self) {
        let _ = self.action_tx.send(Action::GoBack);
    }

    fn go_forward(&mut self) {
        let _ = self.action_tx.send(Action::GoForward);
    }

    fn push(&mut self, route: R) {
        let _ = self.action_tx.send(Action::Push(route));
    }

    fn replace(&mut self, route: R) {
        let _ = self.action_tx.send(Action::Replace(route));
    }

    fn external(&mut self, url: String) -> bool {
        let _ = self.action_tx.send(Action::External(url));
        true
    }

    fn current_route(&self) -> R {
        let timeline = self.timeline.lock().expect("poisoned mutex");
        timeline.current_route.clone()
    }

    fn can_go_back(&self) -> bool {
        let timeline = self.timeline.lock().expect("poisoned mutex");
        !timeline.history.is_empty()
    }

    fn can_go_forward(&self) -> bool {
        let timeline = self.timeline.lock().expect("poisoned mutex");
        !timeline.future.is_empty()
    }

    fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
        let mut updater_callback = self.updater_callback.write().unwrap();
        *updater_callback = callback;
    }
}
