use super::HistoryProvider;
use crate::routable::Routable;
use dioxus_lib::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, RwLock};
use std::{collections::BTreeMap, rc::Rc, str::FromStr, sync::Arc};

/// A [`HistoryProvider`] that evaluates history through JS.
pub struct LiveviewHistory<R: Routable>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    action_tx: tokio::sync::mpsc::UnboundedSender<Action<R>>,
    timeline: Arc<Mutex<Timeline<R>>>,
    updater_callback: Arc<RwLock<Arc<dyn Fn() + Send + Sync>>>,
}

struct Timeline<R: Routable>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    current_index: usize,
    routes: BTreeMap<usize, R>,
}

#[derive(Serialize, Deserialize, Debug)]
struct State {
    index: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct Session<R: Routable>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    #[serde(with = "routes")]
    routes: BTreeMap<usize, R>,
    last_visited: usize,
}

#[derive(Serialize, Deserialize)]
struct SessionStorage {
    liveview: Option<String>,
}

enum Action<R: Routable> {
    GoBack,
    GoForward,
    Push(R),
    Replace(R),
    External(String),
}

impl<R: Routable> Timeline<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn new(initial_path: R) -> Self {
        Self {
            current_index: 0,
            routes: BTreeMap::from([(0, initial_path)]),
        }
    }

    fn init(
        &mut self,
        route: R,
        state: Option<State>,
        session: Option<Session<R>>,
        depth: usize,
    ) -> State {
        if let Some(session) = session {
            self.routes = session.routes;
            if state.is_none() {
                // top of stack
                let last_visited = session.last_visited;
                self.routes.retain(|&lhs, _| lhs <= last_visited);
            }
        };
        let state = match state {
            Some(state) => {
                self.current_index = state.index;
                state
            }
            None => {
                let index = depth - 1;
                self.current_index = index;
                State { index }
            }
        };
        self.routes.insert(state.index, route);
        state
    }

    fn update(&mut self, route: R, state: Option<State>) -> State {
        if let Some(state) = state {
            self.current_index = state.index;
            self.routes.insert(self.current_index, route);
            state
        } else {
            self.push(route)
        }
    }

    fn push(&mut self, route: R) -> State {
        // top of stack
        let index = self.current_index + 1;
        self.current_index = index;
        self.routes.insert(index, route);
        self.routes.retain(|&rhs, _| index >= rhs);
        State {
            index: self.current_index,
        }
    }

    fn replace(&mut self, route: R) -> State {
        self.routes.insert(self.current_index, route);
        State {
            index: self.current_index,
        }
    }

    fn current_route(&self) -> &R {
        &self.routes[&self.current_index]
    }

    fn session(&self) -> Session<R> {
        Session {
            routes: self.routes.clone(),
            last_visited: self.current_index,
        }
    }
}

impl<R: Routable> Default for LiveviewHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Routable> LiveviewHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    /// Create a [`LiveviewHistory`] in the given scope.
    /// When using a [`LiveviewHistory`] in combination with use_eval, history must be untampered with.
    ///
    /// # Panics
    ///
    /// Panics if the function is not called in a dioxus runtime with a Liveview context.
    pub fn new() -> Self {
        Self::new_with_initial_path(
            "/".parse().unwrap_or_else(|err| {
                panic!("index route does not exist:\n{}\n use LiveviewHistory::new_with_initial_path to set a custom path", err)
            }),
        )
    }

    /// Create a [`LiveviewHistory`] in the given scope, starting at `initial_path`.
    /// When using a [`LiveviewHistory`] in combination with use_eval, history must be untampered with.
    ///
    /// # Panics
    ///
    /// Panics if the function is not called in a dioxus runtime with a Liveview context.
    pub fn new_with_initial_path(initial_path: R) -> Self {
        let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel::<Action<R>>();

        let timeline = Arc::new(Mutex::new(Timeline::new(initial_path)));
        let updater_callback: Arc<RwLock<Arc<dyn Fn() + Send + Sync>>> =
            Arc::new(RwLock::new(Arc::new(|| {})));

        let eval_provider = consume_context::<Rc<dyn EvalProvider>>();

        let create_eval = Rc::new(move |script: &str| {
            UseEval::new(eval_provider.new_evaluator(script.to_string()))
        }) as Rc<dyn Fn(&str) -> UseEval>;

        // Listen to server actions
        spawn({
            let timeline = timeline.clone();
            let create_eval = create_eval.clone();
            async move {
                loop {
                    let eval = action_rx.recv().await.expect("sender to exist");

                    let _ = match eval {
                        Action::GoBack => create_eval(
                            r#"
                                // this triggers a PopState event
                                history.back();
                            "#,
                        ),
                        Action::GoForward => create_eval(
                            r#"
                                // this triggers a PopState event
                                history.forward();
                            "#,
                        ),
                        Action::Push(route) => {
                            let mut timeline = timeline.lock().expect("unpoisoned mutex");
                            let state = timeline.push(route.clone());
                            let state = serde_json::to_string(&state).expect("serializable state");
                            let session = serde_json::to_string(&timeline.session())
                                .expect("serializable session");
                            create_eval(&format!(
                                r#"
                                // this does not trigger a PopState event
                                history.pushState({state}, "", "{route}");
                                sessionStorage.setItem("liveview", '{session}');
                            "#
                            ))
                        }
                        Action::Replace(route) => {
                            let mut timeline = timeline.lock().expect("unpoisoned mutex");
                            let state = timeline.replace(route.clone());
                            let state = serde_json::to_string(&state).expect("serializable state");
                            let session = serde_json::to_string(&timeline.session())
                                .expect("serializable session");
                            create_eval(&format!(
                                r#"
                                // this does not trigger a PopState event
                                history.replaceState({state}, "", "{route}");
                                sessionStorage.setItem("liveview", '{session}');
                            "#
                            ))
                        }
                        Action::External(url) => create_eval(&format!(
                            r#"
                                location.href = "{url}";
                            "#
                        )),
                    };
                }
            }
        });

        // Listen to browser actions
        spawn({
            let updater = updater_callback.clone();
            let timeline = timeline.clone();
            let create_eval = create_eval.clone();
            async move {
                let mut popstate_eval = {
                    let init_eval = create_eval(
                        r#"
                        return [
                          document.location.pathname + "?" + document.location.search + "\#" + document.location.hash,
                          history.state,
                          JSON.parse(sessionStorage.getItem("liveview")),
                          history.length,
                        ];
                    "#,
                    ).await.expect("serializable state");
                    let (route, state, session, depth) = serde_json::from_value::<(
                        String,
                        Option<State>,
                        Option<Session<R>>,
                        usize,
                    )>(init_eval)
                    .expect("serializable state");
                    let Ok(route) = R::from_str(&route.to_string()) else {
                        return;
                    };
                    let mut timeline = timeline.lock().expect("unpoisoned mutex");
                    let state = timeline.init(route.clone(), state, session, depth);
                    let state = serde_json::to_string(&state).expect("serializable state");
                    let session =
                        serde_json::to_string(&timeline.session()).expect("serializable session");

                    // Call the updater callback
                    (updater.read().unwrap())();

                    create_eval(&format!(
                        r#"
                        // this does not trigger a PopState event
                        history.replaceState({state}, "", "{route}");
                        sessionStorage.setItem("liveview", '{session}');

                        window.addEventListener("popstate", (event) => {{
                          dioxus.send([
                            document.location.pathname + "?" + document.location.search + "\#" + document.location.hash,
                            event.state,
                          ]);
                        }});
                    "#
                    ))
                };

                loop {
                    let event = match popstate_eval.recv().await {
                        Ok(event) => event,
                        Err(_) => continue,
                    };
                    let (route, state) = serde_json::from_value::<(String, Option<State>)>(event)
                        .expect("serializable state");
                    let Ok(route) = R::from_str(&route.to_string()) else {
                        return;
                    };
                    let mut timeline = timeline.lock().expect("unpoisoned mutex");
                    let state = timeline.update(route.clone(), state);
                    let state = serde_json::to_string(&state).expect("serializable state");
                    let session =
                        serde_json::to_string(&timeline.session()).expect("serializable session");

                    let _ = create_eval(&format!(
                        r#"
                        // this does not trigger a PopState event
                        history.replaceState({state}, "", "{route}");
                        sessionStorage.setItem("liveview", '{session}');
                    "#
                    ));

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

impl<R: Routable> HistoryProvider<R> for LiveviewHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
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
        let timeline = self.timeline.lock().expect("unpoisoned mutex");
        timeline.current_route().clone()
    }

    fn can_go_back(&self) -> bool {
        let timeline = self.timeline.lock().expect("unpoisoned mutex");
        // Check if the one before is contiguous (i.e., not an external page)
        let visited_indices: Vec<usize> = timeline.routes.keys().cloned().collect();
        visited_indices
            .iter()
            .position(|&rhs| timeline.current_index == rhs)
            .map_or(false, |index| {
                index > 0 && visited_indices[index - 1] == timeline.current_index - 1
            })
    }

    fn can_go_forward(&self) -> bool {
        let timeline = self.timeline.lock().expect("unpoisoned mutex");
        // Check if the one after is contiguous (i.e., not an external page)
        let visited_indices: Vec<usize> = timeline.routes.keys().cloned().collect();
        visited_indices
            .iter()
            .rposition(|&rhs| timeline.current_index == rhs)
            .map_or(false, |index| {
                index < visited_indices.len() - 1
                    && visited_indices[index + 1] == timeline.current_index + 1
            })
    }

    fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
        let mut updater_callback = self.updater_callback.write().unwrap();
        *updater_callback = callback;
    }
}

mod routes {
    use crate::prelude::Routable;
    use core::str::FromStr;
    use serde::de::{MapAccess, Visitor};
    use serde::{ser::SerializeMap, Deserializer, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S, R>(routes: &BTreeMap<usize, R>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        R: Routable,
    {
        let mut map = serializer.serialize_map(Some(routes.len()))?;
        for (index, route) in routes.iter() {
            map.serialize_entry(&index.to_string(), &route.to_string())?;
        }
        map.end()
    }

    pub fn deserialize<'de, D, R>(deserializer: D) -> Result<BTreeMap<usize, R>, D::Error>
    where
        D: Deserializer<'de>,
        R: Routable,
        <R as FromStr>::Err: std::fmt::Display,
    {
        struct BTreeMapVisitor<R> {
            marker: std::marker::PhantomData<R>,
        }

        impl<'de, R> Visitor<'de> for BTreeMapVisitor<R>
        where
            R: Routable,
            <R as FromStr>::Err: std::fmt::Display,
        {
            type Value = BTreeMap<usize, R>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map with indices and routable values")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut routes = BTreeMap::new();
                while let Some((index, route)) = map.next_entry::<String, String>()? {
                    let index = index.parse::<usize>().map_err(serde::de::Error::custom)?;
                    let route = R::from_str(&route).map_err(serde::de::Error::custom)?;
                    routes.insert(index, route);
                }
                Ok(routes)
            }
        }

        deserializer.deserialize_map(BTreeMapVisitor {
            marker: std::marker::PhantomData,
        })
    }
}
