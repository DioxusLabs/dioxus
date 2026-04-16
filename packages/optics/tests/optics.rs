use std::{
    collections::{BTreeMap, HashMap},
    future::{ready, Future},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Wake, Waker},
};

use dioxus_core::{ScopeId, VNode, VirtualDom};
use dioxus_optics::{
    AsFuture, AwaitTransform, FlattenSomeOp, FutureAccess, FutureProject, OkPrism, OptPrismOp,
    Optic, Prism, PrismOp, Resource, ResourceFuture, SomePrism, ValueAccess,
};
use dioxus_signals::{CopyValue, Memo, ReadableExt, Signal, SyncSignal};
use dioxus_stores::Store;

#[derive(Debug, Clone)]
struct App {
    user: Option<User>,
    todos: Vec<Todo>,
}

#[derive(Debug, Clone, PartialEq)]
struct User {
    active: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Todo {
    done: bool,
    title: String,
}

fn app_user(app: &App) -> &Option<User> {
    &app.user
}

fn app_user_mut(app: &mut App) -> &mut Option<User> {
    &mut app.user
}

fn app_todos(app: &App) -> &Vec<Todo> {
    &app.todos
}

fn app_todos_mut(app: &mut App) -> &mut Vec<Todo> {
    &mut app.todos
}

fn todo_done(todo: &Todo) -> &bool {
    &todo.done
}

fn todo_done_mut(todo: &mut Todo) -> &mut bool {
    &mut todo.done
}

fn todo_title(todo: &Todo) -> &String {
    &todo.title
}

fn todo_title_mut(todo: &mut Todo) -> &mut String {
    &mut todo.title
}

fn user_active(user: &User) -> &bool {
    &user.active
}

fn user_active_mut(user: &mut User) -> &mut bool {
    &mut user.active
}

#[derive(Clone)]
struct NestedOptionCarrier(Option<Option<i32>>);

impl ValueAccess<Option<Option<i32>>> for NestedOptionCarrier {
    fn value(&self) -> Option<Option<i32>> {
        self.0
    }
}

#[derive(Clone)]
struct NestedOptionFutureCarrier(Option<Option<i32>>);

impl FutureAccess<AsFuture<std::future::Ready<Option<Option<i32>>>>>
    for NestedOptionFutureCarrier
{
    fn future(&self) -> AsFuture<std::future::Ready<Option<Option<i32>>>> {
        AsFuture(ready(self.0))
    }
}

#[derive(Clone)]
struct ResultFutureCarrier<T, E> {
    cell: CopyValue<Result<T, E>>,
}

impl<T: 'static, E: 'static> ResultFutureCarrier<T, E> {
    fn new(value: Result<T, E>) -> Self {
        Self {
            cell: CopyValue::new(value),
        }
    }
}

impl<T: Clone + 'static, E: Clone + 'static>
    FutureAccess<AsFuture<std::future::Ready<Result<T, E>>>> for ResultFutureCarrier<T, E>
{
    fn future(&self) -> AsFuture<std::future::Ready<Result<T, E>>> {
        let value = self.cell.read_unchecked().clone();
        AsFuture(ready(value))
    }
}

// NB: map_ok now requires `A: Access<Target = Result<T, E>>`, so the old
// trick of impling `ResultSource` on a carrier that doesn't have an `Access`
// impl for `Result<T, E>` no longer applies; the future-carrier tests below
// reach for the stdlib `OkPrism` / `map_variant` path directly.
impl<T: 'static, E: 'static> ValueAccess<Result<T, E>> for ResultFutureCarrier<T, E>
where
    T: Clone,
    E: Clone,
{
    fn value(&self) -> Result<T, E> {
        self.cell.read_unchecked().clone()
    }
}

#[test]
fn tuple_lens_supports_read_write() {
    with_runtime(|| {
        let app = Optic::new(App {
            user: Some(User { active: true }),
            todos: vec![Todo {
                done: false,
                title: "write code".into(),
            }],
        });

        let todos = app.clone().map_ref_mut(app_todos, app_todos_mut);
        assert_eq!(todos.read().len(), 1);

        todos.write().push(Todo {
            done: true,
            title: "ship".into(),
        });
        assert_eq!(todos.read().len(), 2);
    });
}

#[test]
fn optional_projection_supports_read_and_write() {
    with_runtime(|| {
        let app = Optic::new(App {
            user: Some(User { active: true }),
            todos: vec![],
        });

        let active = app
            .map_ref_mut(app_user, app_user_mut)
            .map_some()
            .map_ref_mut(user_active, user_active_mut);

        assert_eq!(*active.read_opt().unwrap(), true);
        *active.write_opt().unwrap() = false;
        assert_eq!(*active.read_opt().unwrap(), false);
    });
}

#[test]
fn optional_path_can_be_materialized_as_option_of_required_optic() {
    with_runtime(|| {
        let app = Optic::new(App {
            user: Some(User { active: true }),
            todos: vec![],
        });

        let active = app
            .clone()
            .map_ref_mut(app_user, app_user_mut)
            .map_some()
            .map_ref_mut(user_active, user_active_mut)
            .to_option()
            .unwrap();

        assert!(*active.read());

        let missing = Optic::new(App {
            user: None,
            todos: vec![],
        })
        .map_ref_mut(app_user, app_user_mut)
        .map_some()
        .to_option();

        assert!(missing.is_none());
    });
}

#[test]
fn result_projection_supports_read_write_and_future() {
    with_runtime(|| {
        let ok = Optic::new(Ok::<User, String>(User { active: true }));
        let active = ok.map_ok().map_ref_mut(user_active, user_active_mut);

        assert_eq!(*active.read_opt().unwrap(), true);
        *active.write_opt().unwrap() = false;
        assert_eq!(*active.read_opt().unwrap(), false);

        let err = Optic::new(Err::<User, String>("offline".to_string())).map_err();
        assert_eq!(&*err.read_opt().unwrap(), "offline");

        let future_carrier =
            Optic::from_access(ResultFutureCarrier::new(Ok::<User, String>(User {
                active: true,
            })));
        let fut: AsFuture<
            AwaitTransform<
                std::future::Ready<Result<User, String>>,
                PrismOp<OkPrism<User, String>>,
                Option<User>,
            >,
        > = future_carrier.map_ok().future();
        assert_eq!(block_on(fut).map(|user| user.active), Some(true));
    });
}

#[test]
fn nested_shape_projection_composes() {
    with_runtime(|| {
        let option_result = Optic::new(Some(Ok::<User, String>(User { active: true })));
        let active = option_result
            .map_some()
            .map_ok()
            .map_ref_mut(user_active, user_active_mut);

        assert_eq!(*active.read_opt().unwrap(), true);
        *active.write_opt().unwrap() = false;
        assert_eq!(*active.read_opt().unwrap(), false);

        let result_option = Optic::new(Ok::<Option<User>, String>(Some(User { active: true })));
        let active = result_option
            .map_ok()
            .map_some()
            .map_ref_mut(user_active, user_active_mut);

        assert_eq!(*active.read_opt().unwrap(), true);
        *active.write_opt().unwrap() = false;
        assert_eq!(*active.read_opt().unwrap(), false);

        let future_carrier = Optic::from_access(ResultFutureCarrier::new(
            Ok::<Option<User>, String>(Some(User { active: true })),
        ));
        let fut: AsFuture<
            AwaitTransform<
                AwaitTransform<
                    std::future::Ready<Result<Option<User>, String>>,
                    PrismOp<OkPrism<Option<User>, String>>,
                    Option<Option<User>>,
                >,
                OptPrismOp<SomePrism<User>>,
                Option<User>,
            >,
        > = future_carrier.map_ok().map_some().future();
        assert_eq!(block_on(fut).map(|user| user.active), Some(true));
    });
}

#[test]
fn owned_value_projection_composes_through_fields_and_shapes() {
    with_runtime(|| {
        let user = Optic::new(User { active: true });
        let active: bool = user.map_ref_mut(user_active, user_active_mut).value();
        assert!(active);

        let option_result = Optic::new(Some(Ok::<User, String>(User { active: true })));
        let active: Option<bool> = option_result
            .map_some()
            .map_ok()
            .map_ref_mut(user_active, user_active_mut)
            .value();
        assert_eq!(active, Some(true));

        let result_option = Optic::new(Ok::<Option<User>, String>(Some(User { active: false })));
        let active: Option<bool> = result_option
            .map_ok()
            .map_some()
            .map_ref_mut(user_active, user_active_mut)
            .value();
        assert_eq!(active, Some(false));

        let resource = Optic::from_access(Resource::resolved(User { active: true }));
        let active: Option<bool> = resource.map_ref_mut(user_active, user_active_mut).value();
        assert_eq!(active, Some(true));
    });
}

#[test]
fn each_projects_vec_children() {
    with_runtime(|| {
        let app = Optic::new(App {
            user: None,
            todos: vec![
                Todo {
                    done: false,
                    title: "write code".into(),
                },
                Todo {
                    done: false,
                    title: "ship".into(),
                },
            ],
        });

        let each_todo = app.map_ref_mut(app_todos, app_todos_mut).each::<Todo>();
        for todo in each_todo.iter() {
            *todo.map_ref_mut(todo_done, todo_done_mut).write() = true;
        }

        let titles: Vec<String> = each_todo
            .iter()
            .map(|todo| todo.map_ref_mut(todo_title, todo_title_mut).read().clone())
            .collect();
        assert_eq!(titles, vec!["write code".to_string(), "ship".to_string()]);

        for todo in each_todo.iter() {
            assert!(*todo.map_ref_mut(todo_done, todo_done_mut).read());
        }
    });
}

#[test]
fn vec_collection_supports_lookup_and_mutation_helpers() {
    with_runtime(|| {
        let todos = Optic::new(vec![
            Todo {
                done: false,
                title: "write code".into(),
            },
            Todo {
                done: false,
                title: "ship".into(),
            },
        ])
        .each::<Todo>();

        assert_eq!(todos.len(), 2);
        assert!(!todos.is_empty());

        let second = todos.get(1);
        assert_eq!(
            &*second
                .map_ref_mut(todo_title, todo_title_mut)
                .read_opt()
                .unwrap(),
            "ship"
        );

        assert!(todos.get(99).read_opt().is_none());

        let second_title: String = todos
            .get(1)
            .map_ref_mut(todo_title, todo_title_mut)
            .value::<Option<String>>()
            .unwrap();
        assert_eq!(second_title, "ship");

        *todos.index(0).map_ref_mut(todo_done, todo_done_mut).write() = true;
        assert!(*todos.index(0).map_ref_mut(todo_done, todo_done_mut).read());

        todos.push(Todo {
            done: false,
            title: "review".into(),
        });
        assert_eq!(todos.len(), 3);

        let removed = todos.remove(1);
        assert_eq!(removed.title, "ship");

        todos.insert(
            1,
            Todo {
                done: false,
                title: "test".into(),
            },
        );
        assert_eq!(
            &*todos
                .index(1)
                .map_ref_mut(todo_title, todo_title_mut)
                .read(),
            "test"
        );

        todos.retain(|todo| !todo.done);
        assert_eq!(todos.len(), 2);

        todos.clear();
        assert!(todos.is_empty());
    });
}

#[test]
fn optional_vec_lookup_chains_without_turbofish() {
    with_runtime(|| {
        let nested = Optic::new(vec![vec![10, 20], vec![30]]).each::<Vec<i32>>();

        let second = nested.get(0).get(1).to_option().unwrap();
        assert_eq!(*second.read(), 20);

        assert!(nested.get(0).get(99).to_option().is_none());
        assert!(nested.get(99).get(0).to_option().is_none());
    });
}

#[test]
fn optional_hash_map_lookup_chains_without_special_traits() {
    with_runtime(|| {
        let nested = Optic::new(HashMap::from([(
            "outer".to_string(),
            HashMap::from([("inner".to_string(), 10)]),
        )]))
        .each_hash_map::<String, HashMap<String, i32>, std::collections::hash_map::RandomState>();

        let value = nested.get("outer").get("inner").to_option().unwrap();
        assert_eq!(*value.read(), 10);

        assert!(nested.get("outer").get("missing").to_option().is_none());
        assert!(nested.get("missing").get("inner").to_option().is_none());
    });
}

#[test]
fn optional_btree_map_lookup_chains_without_special_traits() {
    with_runtime(|| {
        let nested = Optic::new(BTreeMap::from([(
            "outer".to_string(),
            BTreeMap::from([("inner".to_string(), 10)]),
        )]))
        .each_btree_map::<String, BTreeMap<String, i32>>();

        let value = nested.get("outer").get("inner").to_option().unwrap();
        assert_eq!(*value.read(), 10);

        assert!(nested.get("outer").get("missing").to_option().is_none());
        assert!(nested.get("missing").get("inner").to_option().is_none());
    });
}

#[test]
fn hash_map_projection_supports_lookup_iteration_and_mutation() {
    with_runtime(|| {
        let users = Optic::new(HashMap::from([
            ("alice".to_string(), User { active: true }),
            ("bob".to_string(), User { active: false }),
        ]))
        .each_hash_map::<String, User, std::collections::hash_map::RandomState>();

        assert_eq!(users.len(), 2);
        assert!(users.contains_key("alice"));

        let alice = users.get("alice");
        assert_eq!(
            *alice
                .clone()
                .map_ref_mut(user_active, user_active_mut)
                .read_opt()
                .unwrap(),
            true
        );
        *alice
            .clone()
            .map_ref_mut(user_active, user_active_mut)
            .write_opt()
            .unwrap() = false;
        assert_eq!(
            *users
                .get("alice")
                .map_ref_mut(user_active, user_active_mut)
                .read_opt()
                .unwrap(),
            false
        );

        assert!(users
            .get("nobody")
            .map_ref_mut(user_active, user_active_mut)
            .read_opt()
            .is_none());

        let mut keys = users.iter().map(|(key, _)| key).collect::<Vec<_>>();
        keys.sort();
        assert_eq!(keys, vec!["alice".to_string(), "bob".to_string()]);

        let active_count = users
            .values()
            .filter(|user| {
                *user
                    .clone()
                    .map_ref_mut(user_active, user_active_mut)
                    .read()
            })
            .count();
        assert_eq!(active_count, 0);

        users.insert("cora".to_string(), User { active: true });
        assert!(users.contains_key("cora"));
        assert!(users.remove("bob").is_some());
        assert!(!users.contains_key("bob"));

        users.retain(|key, _| key != "alice");
        assert!(!users.contains_key("alice"));

        users.clear();
        assert!(users.is_empty());
    });
}

#[test]
fn btree_map_projection_supports_lookup_iteration_and_mutation() {
    with_runtime(|| {
        let users = Optic::new(BTreeMap::from([
            ("alice".to_string(), User { active: true }),
            ("bob".to_string(), User { active: false }),
        ]))
        .each_btree_map::<String, User>();

        assert_eq!(users.len(), 2);
        assert!(users.contains_key("alice"));

        let bob = users.get("bob");
        assert_eq!(
            *bob.clone()
                .map_ref_mut(user_active, user_active_mut)
                .read_opt()
                .unwrap(),
            false
        );
        *bob.clone()
            .map_ref_mut(user_active, user_active_mut)
            .write_opt()
            .unwrap() = true;
        assert_eq!(
            *users
                .get("bob")
                .map_ref_mut(user_active, user_active_mut)
                .read_opt()
                .unwrap(),
            true
        );

        assert!(users
            .get("nobody")
            .map_ref_mut(user_active, user_active_mut)
            .read_opt()
            .is_none());

        let keys = users.iter().map(|(key, _)| key).collect::<Vec<_>>();
        assert_eq!(keys, vec!["alice".to_string(), "bob".to_string()]);

        let active_values = users
            .values()
            .map(|user| *user.map_ref_mut(user_active, user_active_mut).read())
            .collect::<Vec<_>>();
        assert_eq!(active_values, vec![true, true]);

        users.insert("cora".to_string(), User { active: false });
        assert!(users.contains_key("cora"));
        assert!(users.remove("alice").is_some());
        assert!(!users.contains_key("alice"));

        users.retain(|key, _| key != "bob");
        assert!(!users.contains_key("bob"));

        users.clear();
        assert!(users.is_empty());
    });
}

#[test]
fn flatten_some_collapses_nested_option() {
    let nested = Optic::from_access(NestedOptionCarrier(Some(Some(10))));
    assert_eq!(nested.flatten_some().value::<Option<i32>>(), Some(10));
}

#[test]
fn flatten_some_composes_separately_with_future_access() {
    let nested = Optic::from_access(NestedOptionFutureCarrier(Some(Some(10))));
    let fut: AsFuture<
        AwaitTransform<std::future::Ready<Option<Option<i32>>>, FlattenSomeOp, Option<i32>>,
    > = nested.flatten_some().future();
    assert_eq!(block_on(fut), Some(10));
}

#[test]
fn resource_projection_composes_with_future_projection() {
    with_runtime(|| {
        let resource = Optic::from_access(Resource::resolved(User { active: true }));
        let projected = resource.map_ref_mut(user_active, user_active_mut);

        assert_eq!(*projected.read_opt().unwrap(), true);

        let fut: FutureProject<ResourceFuture<User>, User, bool> = projected.future();
        assert_eq!(block_on(fut), true);
    });
}

#[test]
#[should_panic(expected = "optics: required path produced no value")]
fn asserted_present_optic_panics_if_shape_changes_later() {
    with_runtime(|| {
        let app = Optic::new(App {
            user: Some(User { active: true }),
            todos: vec![],
        });

        let active = app
            .clone()
            .map_ref_mut(app_user, app_user_mut)
            .map_some()
            .map_ref_mut(user_active, user_active_mut)
            .to_option()
            .unwrap();

        *app.map_ref_mut(app_user, app_user_mut).write() = None;

        let _ = *active.read();
    });
}

#[test]
fn pending_resource_future_wakes_when_resolved() {
    with_runtime(|| {
        let resource = Resource::pending();
        let mut future = Box::pin(resource.clone().future().0);

        struct CounterWake(Arc<AtomicUsize>);

        impl Wake for CounterWake {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let wake_count = Arc::new(AtomicUsize::new(0));
        let waker = Waker::from(Arc::new(CounterWake(wake_count.clone())));
        let mut cx = Context::from_waker(&waker);

        assert!(matches!(future.as_mut().poll(&mut cx), Poll::Pending));
        assert_eq!(wake_count.load(Ordering::SeqCst), 0);

        resource.resolve(42);
        assert_eq!(wake_count.load(Ordering::SeqCst), 1);
        assert!(matches!(future.as_mut().poll(&mut cx), Poll::Ready(42)));
    });
}

fn block_on<F: Future>(fut: F) -> F::Output {
    struct Noop;

    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }

    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut fut = Box::pin(fut);

    loop {
        if let Poll::Ready(value) = fut.as_mut().poll(&mut cx) {
            return value;
        }
    }
}

fn with_runtime<R>(f: impl FnOnce() -> R) -> R {
    let mut dom = VirtualDom::new(VNode::empty);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, f)
}

#[test]
fn optics_compose_over_signal_root() {
    with_runtime(|| {
        let signal = Signal::new(App {
            user: Some(User { active: true }),
            todos: vec![Todo {
                done: false,
                title: "write code".into(),
            }],
        });
        let optic = Optic::from_access(signal);

        let active = optic
            .clone()
            .map_ref_mut(app_user, app_user_mut)
            .map_some()
            .map_ref_mut(user_active, user_active_mut);
        assert!(*active.read_opt().unwrap());
        *active.write_opt().unwrap() = false;
        assert!(!*active.read_opt().unwrap());

        let todos = optic.map_ref_mut(app_todos, app_todos_mut).each::<Todo>();
        todos.push(Todo {
            done: true,
            title: "ship".into(),
        });
        assert_eq!(todos.len(), 2);
    });
}

#[test]
fn optics_compose_over_sync_signal_root() {
    with_runtime(|| {
        let signal: SyncSignal<User> = SyncSignal::new_maybe_sync(User { active: true });
        let optic = Optic::from_access(signal);

        let active = optic.map_ref_mut(user_active, user_active_mut);
        assert!(*active.read());
        *active.write() = false;
        assert!(!*active.read());
    });
}

#[test]
fn optics_compose_over_memo_root_read_only() {
    with_runtime(|| {
        let source = Signal::new(User { active: true });
        let memo = Memo::new(move || User {
            active: source.read().active,
        });

        // map_ref yields a read-only projection — suitable for Memo.
        let active_via_ref = Optic::from_access(memo).map_ref(user_active);
        assert!(*active_via_ref.read());
        assert!(active_via_ref.value::<bool>());
    });
}

#[test]
fn optics_compose_over_copy_value_root() {
    with_runtime(|| {
        let cell = CopyValue::new(User { active: true });
        let active = Optic::from_access(cell).map_ref_mut(user_active, user_active_mut);

        *active.write() = false;
        assert!(!*active.read());
    });
}

#[test]
fn optics_compose_over_store_root() {
    with_runtime(|| {
        let store: Store<User> = Store::new(User { active: true });
        let optic = Optic::from_access(store);

        let active = optic.map_ref_mut(user_active, user_active_mut);
        assert!(*active.read());
        *active.write() = false;
        assert!(!*active.read());
    });
}

// A 4-variant user enum — the scenario the `Prism` primitive exists for.
#[derive(Debug, Clone, PartialEq)]
enum Page {
    Home,
    Profile(ProfileData),
    Settings(Settings),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
struct ProfileData {
    name: String,
    followers: u32,
}

#[derive(Debug, Clone, PartialEq)]
struct Settings {
    dark_mode: bool,
}

// Named prism for the `Profile` variant — the `#[derive(Store)]`-style path.
#[derive(Default, Clone)]
struct ProfilePrism;

impl Prism for ProfilePrism {
    type Source = Page;
    type Variant = ProfileData;

    fn try_ref<'a>(&self, p: &'a Page) -> Option<&'a ProfileData> {
        if let Page::Profile(x) = p {
            Some(x)
        } else {
            None
        }
    }
    fn try_mut<'a>(&self, p: &'a mut Page) -> Option<&'a mut ProfileData> {
        if let Page::Profile(x) = p {
            Some(x)
        } else {
            None
        }
    }
    fn try_into_variant(&self, p: Page) -> Option<ProfileData> {
        if let Page::Profile(x) = p {
            Some(x)
        } else {
            None
        }
    }
}

fn page_error_ref(p: &Page) -> Option<&String> {
    if let Page::Error(m) = p { Some(m) } else { None }
}
fn page_error_mut(p: &mut Page) -> Option<&mut String> {
    if let Page::Error(m) = p { Some(m) } else { None }
}
fn page_error_into(p: Page) -> Option<String> {
    if let Page::Error(m) = p { Some(m) } else { None }
}

fn profile_name(p: &ProfileData) -> &String { &p.name }
fn profile_name_mut(p: &mut ProfileData) -> &mut String { &mut p.name }
fn profile_followers(p: &ProfileData) -> &u32 { &p.followers }
fn profile_followers_mut(p: &mut ProfileData) -> &mut u32 { &mut p.followers }

#[test]
fn prism_projects_into_named_variant_of_user_enum() {
    with_runtime(|| {
        let page = Optic::new(Page::Profile(ProfileData {
            name: "Alice".into(),
            followers: 7,
        }));

        // Named prism: project Page -> ProfileData -> String
        let profile = page.clone().map_variant::<ProfilePrism>();
        let name = profile
            .clone()
            .map_ref_mut(profile_name, profile_name_mut);

        assert_eq!(&*name.read_opt().unwrap(), "Alice");
        *name.write_opt().unwrap() = "Alicia".into();
        assert_eq!(&*name.read_opt().unwrap(), "Alicia");

        // Number projection via the same prism.
        let followers = profile.map_ref_mut(profile_followers, profile_followers_mut);
        *followers.write_opt().unwrap() = 9;
        assert_eq!(*followers.read_opt().unwrap(), 9);

        // Switching variant: projections through the old prism become absent.
        *page.write() = Page::Home;
        assert!(name.read_opt().is_none());
        assert!(followers.read_opt().is_none());
    });
}

#[test]
fn prism_projects_inline_via_map_variant_with() {
    with_runtime(|| {
        let page = Optic::new(Page::Error("offline".to_string()));

        // Inline prism: no named struct needed.
        let message = page.clone().map_variant_with(
            page_error_ref,
            page_error_mut,
            page_error_into,
        );
        assert_eq!(&*message.read_opt().unwrap(), "offline");
        *message.write_opt().unwrap() = "recovered".into();
        assert_eq!(&*message.read_opt().unwrap(), "recovered");

        // Home variant does not match the Error prism.
        *page.write() = Page::Home;
        assert!(message.read_opt().is_none());
    });
}

#[test]
fn prism_value_projection_yields_optional_clone() {
    with_runtime(|| {
        let page = Optic::new(Page::Settings(Settings { dark_mode: true }));
        let dark_mode_value: Option<Settings> = page
            .map_variant_with(
                |p: &Page| if let Page::Settings(s) = p { Some(s) } else { None },
                |p: &mut Page| if let Page::Settings(s) = p { Some(s) } else { None },
                |p: Page| if let Page::Settings(s) = p { Some(s) } else { None },
            )
            .value::<Option<Settings>>();
        assert_eq!(dark_mode_value, Some(Settings { dark_mode: true }));
    });
}

#[test]
fn prism_chains_through_already_optional_paths() {
    with_runtime(|| {
        // Option<Page> -> Page via map_some, then Page::Profile via the prism.
        let root = Optic::new(Some(Page::Profile(ProfileData {
            name: "Bob".into(),
            followers: 0,
        })));
        let profile = root
            .map_some()
            .map_variant::<ProfilePrism>()
            .map_ref_mut(profile_name, profile_name_mut);
        assert_eq!(&*profile.read_opt().unwrap(), "Bob");

        let missing = Optic::new(Option::<Page>::None)
            .map_some()
            .map_variant::<ProfilePrism>();
        assert!(missing.read_opt().is_none());

        let wrong_variant = Optic::new(Some(Page::Home))
            .map_some()
            .map_variant::<ProfilePrism>();
        assert!(wrong_variant.read_opt().is_none());
    });
}

#[test]
fn optics_each_over_signal_vec() {
    with_runtime(|| {
        let signal = Signal::new(vec![
            Todo {
                done: false,
                title: "write code".into(),
            },
            Todo {
                done: false,
                title: "ship".into(),
            },
        ]);
        let todos = Optic::from_access(signal).each::<Todo>();
        for todo in todos.iter() {
            *todo.map_ref_mut(todo_done, todo_done_mut).write() = true;
        }
        for todo in todos.iter() {
            assert!(*todo.map_ref_mut(todo_done, todo_done_mut).read());
        }
    });
}
