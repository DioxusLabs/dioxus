use std::{str::FromStr, sync::Arc};
use dioxus::prelude::*;
use tokio::sync::Mutex;
use super::HistoryProvider;
use crate::routable::Routable;

/// A [`HistoryProvider`] that evaluates history through JS.
pub struct LiveviewHistory<R: Routable> {
    eval_tx: tokio::sync::mpsc::UnboundedSender<Eval<R>>,
    eval_rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Eval<R>>>>,
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
            current_route: "/".parse().unwrap_or_else(|err| {
                panic!("index route does not exist:\n{}\n use MemoryHistory::with_initial_path to set a custom path", err)
            }),
            can_go_back: false,
            can_go_forward: false,
        }
    }
}

impl<R: Routable> LiveviewHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    /// TODO
    pub fn attach(&self, cx: Scope) {
        let create_eval = use_eval(cx);
        let eval_rx = self.eval_rx.clone();
        let _: &Coroutine<()> = use_coroutine(cx, |_| {
            to_owned![create_eval];
            async move {
                loop {
                    let mut eval_rx = eval_rx.lock().await;
                    let eval = eval_rx.recv().await.expect("sender to exist");
                    let _eval = match eval {
                        Eval::GoBack => create_eval(r#"
                            history.back();
                        "#),
                        Eval::GoForward => create_eval(r#"
                            history.forward();
                        "#),
                        Eval::Push(route) => create_eval(&format!(r#"
                            history.pushState("{route}", "", "{route}");
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
    }
}

impl<R: Routable> HistoryProvider<R> for LiveviewHistory<R>
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
        self.current_route.clone()
    }

    fn can_go_back(&self) -> bool {
        self.can_go_back
    }

    fn can_go_forward(&self) -> bool {
        self.can_go_forward
    }
}
