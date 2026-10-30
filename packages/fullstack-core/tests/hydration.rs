#![cfg(any(feature = "web", feature = "server"))]

use base64::Engine;
use dioxus_fullstack_core::{
    HydrationContext, is_hydrating, serialize_context, use_loader, use_server_cached,
    use_server_future,
};
use dioxus_vdom::{CapturedError, Element, ScopeId, VNode, VirtualDom, has_context};

fn round_trip(context: &HydrationContext) -> HydrationContext {
    let serialized = context.serialized();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(serialized.data)
        .unwrap();
    #[cfg(debug_assertions)]
    let (debug_types, debug_locations) = (
        Some(serde_json::from_str(&serialized.debug_types).unwrap()),
        Some(serde_json::from_str(&serialized.debug_locations).unwrap()),
    );
    #[cfg(not(debug_assertions))]
    let (debug_types, debug_locations) = (None, None);
    HydrationContext::from_serialized(&bytes, debug_types, debug_locations)
}

#[cfg(feature = "server")]
#[test]
fn server_context_is_available_to_serialization_walk() {
    fn app() -> Element {
        serialize_context()
            .create_entry()
            .insert(&123_u32, std::panic::Location::caller());
        serialize_context()
            .create_entry()
            .insert(&"second".to_string(), std::panic::Location::caller());
        VNode::empty()
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    // Use the same scope lookup and aggregation as SsrRendererPool::take_from_scope.
    let data = HydrationContext::default();
    dom.in_scope(ScopeId::APP, || {
        let scope_data = has_context::<HydrationContext>()
            .expect("SSR must be able to find the component's hydration data");
        data.extend(&scope_data);
    });
    let restored = round_trip(&data);
    assert_eq!(restored.create_entry::<u32>().get().unwrap(), 123);
    assert_eq!(restored.create_entry::<String>().get().unwrap(), "second");
    assert!(dom.in_scope(ScopeId::APP, is_hydrating));
}

#[cfg(feature = "server")]
#[test]
fn server_hooks_resolve_and_serialize() {
    fn app() -> Element {
        assert_eq!(use_server_cached(|| 123_u32), 123);
        let loader = use_loader(|| async { Ok::<_, CapturedError>(456_u32) })
            .expect("server loader must not wait for client hydration data");
        assert_eq!(loader(), 456);
        let future = use_server_future(|| async { 789_u32 })
            .expect("server future must not wait for client hydration data");
        assert_eq!(future(), Some(789));
        VNode::empty()
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let data = dom.in_scope(ScopeId::APP, || {
        has_context::<HydrationContext>().expect("missing server hydration context")
    });
    let restored = round_trip(&data);
    assert_eq!(restored.create_entry::<u32>().get().unwrap(), 123);
    assert_eq!(
        restored
            .create_entry::<Result<u32, CapturedError>>()
            .get()
            .unwrap()
            .unwrap(),
        456
    );
    assert_eq!(restored.create_entry::<u32>().get().unwrap(), 789);
}

#[cfg(feature = "web")]
#[test]
fn client_hooks_use_serialized_values() {
    let data = HydrationContext::default();
    data.create_entry()
        .insert(&123_u32, std::panic::Location::caller());
    data.create_entry().insert(
        &Ok::<_, CapturedError>(456_u32),
        std::panic::Location::caller(),
    );
    data.create_entry()
        .insert(&789_u32, std::panic::Location::caller());

    fn app() -> Element {
        assert!(is_hydrating());
        assert_eq!(
            use_server_cached::<u32, _>(|| panic!("cached value was ignored")),
            123_u32
        );
        let loader = use_loader(unexpected_future::<Result<u32, CapturedError>>).unwrap();
        assert_eq!(loader(), 456);
        let future = use_server_future(unexpected_future::<u32>).unwrap();
        assert_eq!(future(), Some(789));
        assert!(has_context::<HydrationContext>().is_none());
        dioxus_vdom::provide_context(Rendered);
        VNode::empty()
    }

    let mut dom = VirtualDom::new(app);
    round_trip(&data).in_context(|| dom.rebuild_in_place());
    assert!(
        dom.in_scope(ScopeId::APP, has_context::<Rendered>)
            .is_some()
    );
}

#[cfg(feature = "web")]
#[test]
fn client_pending_data_does_not_run_futures() {
    let data = HydrationContext::default();
    data.create_entry::<Result<u32, CapturedError>>();
    data.create_entry::<u32>();

    fn app() -> Element {
        assert!(use_loader(unexpected_future::<Result<u32, CapturedError>>).is_err());
        assert!(use_server_future(unexpected_future::<u32>).is_err());
        dioxus_vdom::provide_context(Rendered);
        VNode::empty()
    }

    let mut dom = VirtualDom::new(app);
    round_trip(&data).in_context(|| dom.rebuild_in_place());
    assert!(
        dom.in_scope(ScopeId::APP, has_context::<Rendered>)
            .is_some()
    );
}

#[cfg(all(feature = "web", not(feature = "server")))]
#[test]
fn client_after_hydration_has_no_data() {
    assert!(!is_hydrating());
    let context = serialize_context();
    let entry = context.create_entry::<u32>();
    entry.clone().insert(&123, std::panic::Location::caller());
    assert!(matches!(
        entry.get(),
        Err(dioxus_fullstack_core::TakeDataError::DataNotAvailable)
    ));
}

#[cfg(feature = "web")]
#[test]
fn client_route_finalization_preserves_server_route() {
    let data = HydrationContext::default();
    data.create_entry()
        .insert(&"/server-route".to_string(), std::panic::Location::caller());
    let client_data = round_trip(&data);
    let before = client_data.serialized().data;

    fn app() -> Element {
        dioxus_fullstack_core::history::provide_fullstack_history_context(
            dioxus_history::MemoryHistory::with_initial_path("/client-route"),
        );
        dioxus_fullstack_core::commit_initial_chunk();
        dioxus_vdom::provide_context(Rendered);
        VNode::empty()
    }

    let mut dom = VirtualDom::new(app);
    client_data.in_context(|| dom.rebuild_in_place());
    assert!(
        dom.in_scope(ScopeId::APP, has_context::<Rendered>)
            .is_some()
    );
    assert_eq!(client_data.serialized().data, before);
}

#[cfg(feature = "web")]
async fn unexpected_future<T>() -> T {
    panic!("client must use the server hydration data instead of polling this future")
}

#[cfg(feature = "web")]
#[derive(Clone)]
struct Rendered;
