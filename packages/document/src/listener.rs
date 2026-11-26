use dioxus_html::events::{
    EventTargetKind, GlobalEventBinding, GlobalEventCallback, GlobalEventTarget,
};
use dioxus_vdom::{Runtime, current_scope_id, queue_effect, try_consume_context, use_hook};
use std::{cell::RefCell, rc::Rc};

/// Options accepted by the global listener macros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalListenerOptions {
    /// Run during capture rather than bubble.
    pub capture: bool,
    /// Disallow cancelling the browser's default action.
    pub passive: bool,
    /// Install the listener only while this is true.
    pub enabled: bool,
}

impl Default for GlobalListenerOptions {
    fn default() -> Self {
        Self {
            capture: false,
            passive: false,
            enabled: true,
        }
    }
}

/// A renderer's support for listeners on the window and the document.
///
/// Renderers provide an `Rc<dyn GlobalListeners>` as a root context. Where none is
/// provided, [`use_window_listener!`](crate::use_window_listener) and
/// [`use_document_listener!`](crate::use_document_listener) install nothing.
///
/// The callback must be called synchronously while the event is being dispatched, and
/// the default action and propagation updated from its result before dispatch continues.
pub trait GlobalListeners: 'static {
    /// Attach a listener to the window that owns the current component.
    fn add_window_listener(
        &self,
        name: &'static str,
        options: &GlobalListenerOptions,
        callback: GlobalEventCallback,
    ) -> Option<GlobalListenerRegistration>;

    /// Attach a listener to the document that owns the current component.
    fn add_document_listener(
        &self,
        name: &'static str,
        options: &GlobalListenerOptions,
        callback: GlobalEventCallback,
    ) -> Option<GlobalListenerRegistration>;
}

/// A renderer registration that detaches when dropped.
pub struct GlobalListenerRegistration(Option<Box<dyn FnOnce()>>);

impl GlobalListenerRegistration {
    /// Create a registration that runs `detach` when it is dropped.
    pub fn new(detach: impl FnOnce() + 'static) -> Self {
        Self(Some(Box::new(detach)))
    }
}

impl Drop for GlobalListenerRegistration {
    fn drop(&mut self) {
        if let Some(detach) = self.0.take() {
            detach();
        }
    }
}

#[derive(Default)]
struct ListenerState {
    callback: Option<GlobalEventCallback>,
    registration: Option<(
        EventTargetKind,
        &'static str,
        GlobalListenerOptions,
        GlobalListenerRegistration,
    )>,
}

pub fn use_global_listener<Target: GlobalEventTarget>(
    binding: GlobalEventBinding<Target>,
    options: GlobalListenerOptions,
) {
    let state = use_hook(|| Rc::new(RefCell::new(ListenerState::default())));
    let listeners = try_consume_context::<Rc<dyn GlobalListeners>>();

    let target = Target::KIND;
    let state = Rc::downgrade(&state);
    let runtime = Rc::downgrade(&Runtime::current());
    let scope = current_scope_id();
    queue_effect(move || {
        let Some(state) = state.upgrade() else { return };
        let mut current = state.borrow_mut();
        let listeners = listeners.filter(|_| options.enabled);
        current.callback = listeners.as_ref().map(|_| binding.callback);
        if listeners.is_some()
            && current
                .registration
                .as_ref()
                .is_some_and(|(t, name, opts, _)| {
                    *t == target && *name == binding.name && *opts == options
                })
        {
            return;
        }
        let previous = current.registration.take();
        drop(current);
        drop(previous);

        if let Some(listeners) = listeners {
            let weak_state = Rc::downgrade(&state);
            let callback: GlobalEventCallback = Rc::new(move |input| {
                let callback = weak_state
                    .upgrade()
                    .and_then(|state| state.borrow().callback.clone());
                match (runtime.upgrade(), callback) {
                    (Some(runtime), Some(callback)) => runtime.in_scope(scope, || callback(input)),
                    _ => Default::default(),
                }
            });
            let registration = match target {
                EventTargetKind::Window => {
                    listeners.add_window_listener(binding.name, &options, callback)
                }
                _ => listeners.add_document_listener(binding.name, &options, callback),
            };
            if let Some(registration) = registration {
                state.borrow_mut().registration =
                    Some((target, binding.name, options, registration));
            } else {
                tracing::warn!(
                    event = binding.name,
                    "Could not install a global event listener"
                );
            }
        }
    });
}

/// Subscribe to an event on the owning window while this component is mounted.
///
/// The event is named as in `addEventListener`, such as `keydown` or `pointermove`,
/// and the callback receives the same typed event as the matching element handler.
/// `resize` reports the layout viewport size as
/// [`WindowResizeData`](dioxus_html::events::WindowResizeData); it does not emit an
/// initial size.
///
/// Options are `capture` (default false), `passive` (default false) and `enabled`
/// (default true).
/// Events that do not bubble only reach the window with `capture: true`.
/// Cancellation must happen synchronously.
///
/// Event names are methods of
/// [`GlobalEventsExtension`](dioxus_html::events::GlobalEventsExtension) and
/// [`WindowEventsExtension`](dioxus_html::events::WindowEventsExtension), which must
/// be in scope; the prelude exports them.
///
/// Web and Desktop support these listeners. On other renderers, and during SSR, no
/// listener is installed.
///
/// ```no_run
/// use dioxus::prelude::*;
///
/// fn App() -> Element {
///     document::use_window_listener!(keydown, move |event| {
///         if event.key() == Key::Named(dioxus::html::NamedKey::Escape) {
///             event.prevent_default();
///         }
///     }, capture: true);
///     rsx! { "Press Escape" }
/// }
/// ```
///
/// ```compile_fail
/// use dioxus_document::use_global_listener;
/// ```
#[macro_export]
macro_rules! use_window_listener {
    ($name:ident, $handler:expr $(, $option:ident : $value:expr)* $(,)?) => {
        $crate::use_window_listener!(@register WindowTarget, $name, $handler; $($option : $value),*)
    };
    // Expand while only the event name has been typed so that editors can complete it.
    ($name:ident $(,)?) => {{
        let _ = $crate::__private::WindowTarget.$name();
    }};
    (@register $target:ident, $name:ident, $handler:expr; $($option:ident : $value:expr),*) => {{
        let options = $crate::GlobalListenerOptions {
            $($option: $value,)*
            ..Default::default()
        };
        let event = $crate::__private::$target.$name();
        $crate::__private::use_global_listener(event.bind($handler), options)
    }};
}

/// Subscribe to an event on the owning document while this component is mounted.
///
/// Accepts the same options as [`use_window_listener!`](crate::use_window_listener).
/// Event names are methods of
/// [`GlobalEventsExtension`](dioxus_html::events::GlobalEventsExtension) and
/// [`DocumentEventsExtension`](dioxus_html::events::DocumentEventsExtension); window-only
/// events such as `resize` are a compile error here.
///
/// ```compile_fail
/// use dioxus::prelude::*;
///
/// fn App() -> Element {
///     document::use_document_listener!(resize, move |_| {});
///     rsx! {}
/// }
/// ```
#[macro_export]
macro_rules! use_document_listener {
    ($name:ident, $handler:expr $(, $option:ident : $value:expr)* $(,)?) => {
        $crate::use_window_listener!(@register DocumentTarget, $name, $handler; $($option : $value),*)
    };
    ($name:ident $(,)?) => {{
        let _ = $crate::__private::DocumentTarget.$name();
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_html::events::{
        GlobalEventInput, GlobalEventResult, GlobalEventsExtension, WindowEventsExtension,
        WindowTarget,
    };
    use dioxus_vdom::{Element, NoOpMutations, ScopeId, VNode, VirtualDom, consume_context};
    use std::cell::Cell;

    #[derive(Default)]
    struct TestDocument {
        attached: Cell<usize>,
        document_attached: Cell<usize>,
        detached: Rc<Cell<usize>>,
        callback: RefCell<Option<GlobalEventCallback>>,
    }

    impl GlobalListeners for TestDocument {
        fn add_window_listener(
            &self,
            _: &'static str,
            _: &GlobalListenerOptions,
            callback: GlobalEventCallback,
        ) -> Option<GlobalListenerRegistration> {
            self.attached.set(self.attached.get() + 1);
            *self.callback.borrow_mut() = Some(callback);
            let detached = self.detached.clone();
            Some(GlobalListenerRegistration::new(move || {
                detached.set(detached.get() + 1);
            }))
        }

        fn add_document_listener(
            &self,
            name: &'static str,
            _: &GlobalListenerOptions,
            _: GlobalEventCallback,
        ) -> Option<GlobalListenerRegistration> {
            assert_eq!(name, "keydown");
            self.document_attached.set(self.document_attached.get() + 1);
            Some(GlobalListenerRegistration::new(|| {}))
        }
    }

    fn app() -> Element {
        let version = consume_context::<Rc<Cell<u32>>>().get();
        let binding = GlobalEventBinding::<WindowTarget>::new(
            "keydown",
            Rc::new(move |_| GlobalEventResult {
                prevent_default: version == 1,
                ..Default::default()
            }),
        );
        use_global_listener(
            binding,
            GlobalListenerOptions {
                enabled: version != 2,
                ..Default::default()
            },
        );
        VNode::empty()
    }

    #[test]
    fn attaches_after_commit_updates_without_reinstall_and_detaches() {
        let mut dom = VirtualDom::new(app);
        let document = Rc::new(TestDocument::default());
        let version = Rc::new(Cell::new(0u32));
        dom.insert_any_root_context(Box::new(document.clone() as Rc<dyn GlobalListeners>));
        dom.insert_any_root_context(Box::new(version.clone()));
        let emit = || {
            document.callback.borrow().as_ref().unwrap()(GlobalEventInput {
                payload: Rc::new(dioxus_html::PlatformEventData::new(Box::new(()))),
                bubbles: true,
                cancelable: true,
                default_prevented: false,
            })
            .prevent_default
        };

        dom.rebuild(&mut NoOpMutations);
        assert_eq!(document.attached.get(), 0);
        dom.process_events();
        assert_eq!(document.attached.get(), 1);
        assert!(!emit());

        for version_value in 1..=3 {
            version.set(version_value);
            dom.mark_dirty(ScopeId::APP);
            dom.render_immediate(&mut NoOpMutations);
            dom.process_events();
            match version_value {
                1 => {
                    assert_eq!(document.attached.get(), 1);
                    assert!(emit());
                }
                2 => {
                    assert_eq!(document.detached.get(), 1);
                    assert!(!emit());
                }
                _ => assert_eq!(document.attached.get(), 2),
            }
        }
        drop(dom);
        assert_eq!(document.detached.get(), 2);
        assert!(!emit());
    }

    #[test]
    fn document_macro_registers_on_document() {
        let mut dom = VirtualDom::new(|| {
            crate::use_document_listener!(keydown, move |_| {});
            VNode::empty()
        });
        let document = Rc::new(TestDocument::default());
        dom.insert_any_root_context(Box::new(document.clone() as Rc<dyn GlobalListeners>));
        dom.rebuild(&mut NoOpMutations);
        dom.process_events();
        assert_eq!(document.document_attached.get(), 1);
        assert_eq!(document.attached.get(), 0);
    }

    #[test]
    fn installs_nothing_without_a_provider() {
        let mut dom = VirtualDom::new(|| {
            crate::use_window_listener!(resize, move |event| {
                let _ = event.width();
            });
            crate::use_document_listener!(
                scroll,
                move |_| {},
                capture: true,
            );
            VNode::empty()
        });
        dom.rebuild(&mut NoOpMutations);
        dom.process_events();
    }
}
