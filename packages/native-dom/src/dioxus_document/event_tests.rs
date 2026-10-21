use super::*;
use blitz_traits::events::{BlitzKeyEvent, KeyState};
use dioxus::prelude::*;
use keyboard_types::{Code, Key, Location, Modifiers};
use std::cell::RefCell;

type Calls = Rc<RefCell<Vec<(&'static str, bool)>>>;

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Bubble,
    Prevent,
    Stop,
    UnmappedTarget,
}

#[derive(Clone, PartialEq)]
struct Config {
    action: Action,
    calls: Calls,
}

fn sample(config: Config) -> Element {
    let outer_calls = config.calls.clone();
    let parent_calls = config.calls.clone();
    let child_calls = config.calls;
    rsx! {
        div {
            onkeydown: move |event| outer_calls.borrow_mut().push(("outer", event.default_action_enabled())),
            div {
                onkeydown: move |event| parent_calls.borrow_mut().push(("parent", event.default_action_enabled())),
                if config.action == Action::UnmappedTarget {
                    button { id: "target", "No listener on this target" }
                } else {
                    button {
                        id: "target",
                        onkeydown: move |event| {
                            child_calls.borrow_mut().push(("child", event.default_action_enabled()));
                            match config.action {
                                Action::Prevent => event.prevent_default(),
                                Action::Stop => event.stop_propagation(),
                                Action::Bubble | Action::UnmappedTarget => {}
                            }
                        },
                        "Target with a listener"
                    }
                }
            }
        }
    }
}

fn dispatch(action: Action, bubbles: bool) -> (Vec<(&'static str, bool)>, EventState) {
    let calls = Calls::default();
    let mut doc = DioxusDocument::new(
        VirtualDom::new_with_props(
            sample,
            Config {
                action,
                calls: calls.clone(),
            },
        ),
        DocumentConfig::default(),
    );
    doc.initial_build();
    let target = doc.inner.query_selector("#target").unwrap().unwrap();
    if action == Action::UnmappedTarget {
        assert!(get_dioxus_id(doc.inner.get_node(target).unwrap()).is_none());
    }
    // Mirror EventDriver's native chain construction, without invoking an unrelated
    // browser default action or relying on keyboard focus/layout to choose a target.
    let chain = if bubbles {
        doc.inner.node_chain(target)
    } else {
        vec![target]
    };
    let mut event = DomEvent::new(
        target,
        DomEventData::KeyDown(BlitzKeyEvent {
            key: Key::Escape,
            code: Code::Escape,
            modifiers: Modifiers::empty(),
            location: Location::Standard,
            is_auto_repeating: false,
            is_composing: false,
            state: KeyState::Pressed,
            text: None,
        }),
    );
    event.bubbles = bubbles;
    let mut state = EventState::default();
    let mut handler = DioxusEventHandler {
        vdom: &mut doc.vdom,
        vdom_state: &mut doc.vdom_state,
    };
    blitz_dom::EventHandler::handle_event(
        &mut handler,
        &chain,
        &mut event,
        &mut doc.inner.mutate(),
        &mut state,
    );
    let result = calls.borrow().clone();
    (result, state)
}

#[test]
fn bubbling_invokes_each_listener_once_in_target_to_root_order() {
    let (calls, state) = dispatch(Action::Bubble, true);
    assert_eq!(calls, [("child", true), ("parent", true), ("outer", true)]);
    assert!(!state.is_cancelled());
    assert!(!state.propagation_is_stopped());
}

#[test]
fn child_default_prevention_survives_bubbling_and_reaches_the_host() {
    let (calls, state) = dispatch(Action::Prevent, true);
    assert_eq!(
        calls,
        [("child", true), ("parent", false), ("outer", false)]
    );
    assert!(state.is_cancelled());
    assert!(!state.propagation_is_stopped());
}

#[test]
fn child_can_stop_propagation_without_preventing_the_default() {
    let (calls, state) = dispatch(Action::Stop, true);
    assert_eq!(calls, [("child", true)]);
    assert!(!state.is_cancelled());
    assert!(state.propagation_is_stopped());
}

#[test]
fn non_bubbling_event_stays_at_the_target() {
    let (calls, _) = dispatch(Action::Bubble, false);
    assert_eq!(calls, [("child", true)]);
}

#[test]
fn unmapped_target_starts_at_the_first_mapped_ancestor() {
    let (calls, _) = dispatch(Action::UnmappedTarget, true);
    assert_eq!(calls, [("parent", true), ("outer", true)]);
}
