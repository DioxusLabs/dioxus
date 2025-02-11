use dioxus_core::prelude::spawn;
use dioxus_document::Eval;
use dioxus_history::History;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::sync::{Mutex, RwLock};
use std::{collections::BTreeMap, sync::Arc};

/// A [`HistoryProvider`] that evaluates history through JS.
pub(crate) struct LiveviewHistory {
    action_tx: tokio::sync::mpsc::UnboundedSender<Action>,
    timeline: Arc<Mutex<Timeline>>,
    updater_callback: Arc<RwLock<Arc<dyn Fn() + Send + Sync>>>,
}

struct Timeline {
    current_index: usize,
    routes: BTreeMap<usize, String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct State {
    index: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct Session {
    #[serde(with = "routes")]
    routes: BTreeMap<usize, String>,
    last_visited: usize,
}

enum Action {
    GoBack,
    GoForward,
    Push(String),
    Replace(String),
    External(String),
}

impl Timeline {
    fn new(initial_path: String) -> Self {
        Self {
            current_index: 0,
            routes: BTreeMap::from([(0, initial_path)]),
        }
    }

    fn init(
        &mut self,
        route: String,
        state: Option<State>,
        session: Option<Session>,
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

    fn update(&mut self, route: String, state: Option<State>) -> State {
        if let Some(state) = state {
            self.current_index = state.index;
            self.routes.insert(self.current_index, route);
            state
        } else {
            self.push(route)
        }
    }

    fn push(&mut self, route: String) -> State {
        // top of stack
        let index = self.current_index + 1;
        self.current_index = index;
        self.routes.insert(index, route);
        self.routes.retain(|&rhs, _| index >= rhs);
        State {
            index: self.current_index,
        }
    }

    fn replace(&mut self, route: String) -> State {
        self.routes.insert(self.current_index, route);
        State {
            index: self.current_index,
        }
    }

    fn current_route(&self) -> &str {
        &self.routes[&self.current_index]
    }

    fn session(&self) -> Session {
        Session {
            routes: self.routes.clone(),
            last_visited: self.current_index,
        }
    }
}

impl LiveviewHistory {
    /// Create a [`LiveviewHistory`] in the given scope.
    /// When using a [`LiveviewHistory`] in combination with use_eval, history must be untampered with.
    ///
    /// # Panics
    ///
    /// Panics if the function is not called in a dioxus runtime with a Liveview context.
    pub(crate) fn new(eval: Rc<dyn Fn(&str) -> Eval>) -> Self {
        Self::new_with_initial_path(
            "/".parse().unwrap_or_else(|err| {
                panic!("index route does not exist:\n{}\n use LiveviewHistory::new_with_initial_path to set a custom path", err)
            }),
            eval
        )
    }

    /// Create a [`LiveviewHistory`] in the given scope, starting at `initial_path`.
    /// When using a [`LiveviewHistory`] in combination with use_eval, history must be untampered with.
    ///
    /// # Panics
    ///
    /// Panics if the function is not called in a dioxus runtime with a Liveview context.
    fn new_with_initial_path(initial_path: String, eval: Rc<dyn Fn(&str) -> Eval>) -> Self {
        let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel::<Action>();

        let timeline = Arc::new(Mutex::new(Timeline::new(initial_path)));
        let updater_callback: Arc<RwLock<Arc<dyn Fn() + Send + Sync>>> =
            Arc::new(RwLock::new(Arc::new(|| {})));

        // Listen to server actions
        spawn({
            let timeline = timeline.clone();
            let create_eval = eval.clone();
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
            let create_eval = eval.clone();
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
                    let (route, state, session, depth) =
                        serde_json::from_value::<(String, Option<State>, Option<Session>, usize)>(
                            init_eval,
                        )
                        .expect("serializable state");
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

impl History for LiveviewHistory {
    fn go_back(&self) {
        let _ = self.action_tx.send(Action::GoBack);
    }

    fn go_forward(&self) {
        let _ = self.action_tx.send(Action::GoForward);
    }

    fn push(&self, route: String) {
        let _ = self.action_tx.send(Action::Push(route));
    }

    fn replace(&self, route: String) {
        let _ = self.action_tx.send(Action::Replace(route));
    }

    fn external(&self, url: String) -> bool {
        let _ = self.action_tx.send(Action::External(url));
        true
    }

    fn current_route(&self) -> String {
        let timeline = self.timeline.lock().expect("unpoisoned mutex");
        timeline.current_route().to_string()
    }

    fn can_go_back(&self) -> bool {
        let timeline = self.timeline.lock().expect("unpoisoned mutex");
        // Check if the one before is contiguous (i.e., not an external page)
        let visited_indices: Vec<usize> = timeline.routes.keys().cloned().collect();
        visited_indices
            .iter()
            .position(|&rhs| timeline.current_index == rhs)
            .is_some_and(|index| {
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
            .is_some_and(|index| {
                index < visited_indices.len() - 1
                    && visited_indices[index + 1] == timeline.current_index + 1
            })
    }

    fn updater(&self, callback: Arc<dyn Fn() + Send + Sync>) {
        let mut updater_callback = self.updater_callback.write().unwrap();
        *updater_callback = callback;
    }

    fn include_prevent_default(&self) -> bool {
        true
    }
}

mod routes {
    use serde::de::{MapAccess, Visitor};
    use serde::{ser::SerializeMap, Deserializer, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S>(routes: &BTreeMap<usize, String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(routes.len()))?;
        for (index, route) in routes.iter() {
            map.serialize_entry(&index.to_string(), &route.to_string())?;
        }
        map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<usize, String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BTreeMapVisitor {}

        impl<'de> Visitor<'de> for BTreeMapVisitor {
            type Value = BTreeMap<usize, String>;

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
                    routes.insert(index, route);
                }
                Ok(routes)
            }
        }

        deserializer.deserialize_map(BTreeMapVisitor {})
    }
}
