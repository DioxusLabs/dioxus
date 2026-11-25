use crate::Document;
use dioxus_html::events::GlobalEventBinding;
use dioxus_vdom::{Runtime, current_scope_id, queue_effect, try_consume_context, use_hook};
use std::{cell::RefCell, rc::Rc};

pub use dioxus_html::events::{Containment, GlobalListenerContext, ViewportResizeData};
#[doc(hidden)]
pub use dioxus_html::events::{GlobalEventCallback, GlobalEventInput, GlobalEventPayload};

/// The target of a scoped global listener.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalListenerTarget {
    /// The owning window.
    Window,
    /// The owning document.
    Document,
}

/// Options accepted by the global listener macros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalListenerOptions {
    /// Run during capture rather than bubble.
    pub capture: bool,
    /// Disallow cancelling the browser's default action.
    pub passive: bool,
    /// Install the listener only while this is true.
    pub enabled: bool,
    /// An element ID used to compute containment against the original target.
    pub containment_root: Option<String>,
}

impl Default for GlobalListenerOptions {
    fn default() -> Self {
        Self {
            capture: false,
            passive: false,
            enabled: true,
            containment_root: None,
        }
    }
}

/// Whether the current document supports synchronous global listeners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalListenerSupport {
    /// The renderer supports global listeners. Registration happens after rendering.
    Supported,
    /// The renderer does not support global listeners. No listener will be installed.
    Unsupported,
}

/// A renderer registration that detaches when dropped.
#[doc(hidden)]
pub struct GlobalListenerRegistration(Option<Box<dyn FnOnce()>>);

impl GlobalListenerRegistration {
    #[doc(hidden)]
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
        GlobalListenerTarget,
        &'static str,
        GlobalListenerOptions,
        GlobalListenerRegistration,
    )>,
}

#[doc(hidden)]
pub fn use_global_listener(
    target: GlobalListenerTarget,
    binding: GlobalEventBinding,
    options: GlobalListenerOptions,
) -> GlobalListenerSupport {
    let state = use_hook(|| Rc::new(RefCell::new(ListenerState::default())));
    let document = try_consume_context::<Rc<dyn Document>>();
    let document = document.filter(|document| document.supports_global_listeners());
    let support = if document.is_some() {
        GlobalListenerSupport::Supported
    } else {
        GlobalListenerSupport::Unsupported
    };

    let state = Rc::downgrade(&state);
    let runtime = Rc::downgrade(&Runtime::current());
    let scope = current_scope_id();
    queue_effect(move || {
        let Some(state) = state.upgrade() else { return };
        let mut current = state.borrow_mut();
        let document = document.filter(|_| options.enabled);
        current.callback = document.as_ref().map(|_| binding.callback);
        if document.is_some()
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

        if let Some(document) = document {
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
            if let Some(registration) =
                document.add_global_listener(target, binding.name, &options, callback)
            {
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
    support
}

/// Subscribe to an event on the owning window while this component is mounted.
///
/// Use bare identifiers for keyboard, mouse, pointer, focus, drag, wheel, and
/// touch events, such as `keydown` or `pointermove`. The callback receives a typed
/// event. Use `viewportresize` to observe changes in the layout viewport size;
/// it does not emit an initial size. Options are `capture` (default false), `passive` (default false),
/// `enabled` (default true), and `containment_root` (an element ID, enabling a
/// second [`GlobalListenerContext`] argument). Cancellation must happen synchronously.
///
/// Web and Desktop support these listeners. Other renderers return
/// [`GlobalListenerSupport::Unsupported`]; no listener is installed during SSR.
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
#[macro_export]
macro_rules! use_window_listener {
    ($name:ident, $handler:expr $(, $option:ident : $value:expr)* $(,)?) => {
        $crate::__use_global_listener!(Window, window_events, $name, $handler; $($option : $value),*)
    };
}

/// Subscribe to an event on the owning document while this component is mounted.
///
/// Accepts the same options as [`use_window_listener!`](crate::use_window_listener).
/// The `viewportresize` event is only available on the window.
#[macro_export]
macro_rules! use_document_listener {
    ($name:ident, $handler:expr $(, $option:ident : $value:expr)* $(,)?) => {
        $crate::__use_global_listener!(Document, global_events, $name, $handler; $($option : $value),*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __use_global_listener {
    ($target:ident, $module:ident, $name:ident, $handler:expr; $($option:ident : $value:expr),*) => {{
        let options = $crate::GlobalListenerOptions {
            $($option: $crate::__use_global_listener!(@value $option, $value),)*
            ..Default::default()
        };
        let binding = $crate::__use_global_listener!(@bind $module, $name, $handler; $($option),*);
        $crate::__private::use_global_listener($crate::GlobalListenerTarget::$target, binding, options)
    }};
    (@value containment_root, $value:expr) => { Some(::std::convert::Into::<String>::into($value)) };
    (@value $option:ident, $value:expr) => { $value };
    (@bind $module:ident, $name:ident, $handler:expr; containment_root $(, $tail:ident)*) => {
        $crate::__private::$module::contextual::$name($handler)
    };
    (@bind $module:ident, $name:ident, $handler:expr; $option:ident $(, $tail:ident)*) => {
        $crate::__use_global_listener!(@bind $module, $name, $handler; $($tail),*)
    };
    (@bind $module:ident, $name:ident, $handler:expr;) => {
        $crate::__private::$module::plain::$name($handler)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_html::events::GlobalEventResult;
    use dioxus_vdom::{Element, NoOpMutations, ScopeId, VNode, VirtualDom, consume_context};
    use std::cell::Cell;

    #[derive(Default)]
    struct TestDocument {
        attached: Cell<usize>,
        detached: Rc<Cell<usize>>,
        callback: RefCell<Option<GlobalEventCallback>>,
    }

    impl Document for TestDocument {
        fn eval(&self, js: String) -> crate::Eval {
            crate::NoOpDocument.eval(js)
        }

        fn supports_global_listeners(&self) -> bool {
            true
        }

        fn add_global_listener(
            &self,
            _: GlobalListenerTarget,
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
    }

    fn app() -> Element {
        let version = consume_context::<Rc<Cell<u32>>>().get();
        let binding = GlobalEventBinding {
            name: "keydown",
            callback: Rc::new(move |_| GlobalEventResult {
                prevent_default: version == 1,
                ..Default::default()
            }),
        };
        use_global_listener(
            GlobalListenerTarget::Window,
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
        dom.insert_any_root_context(Box::new(document.clone() as Rc<dyn Document>));
        dom.insert_any_root_context(Box::new(version.clone()));
        let emit = || {
            document.callback.borrow().as_ref().unwrap()(GlobalEventInput {
                payload: GlobalEventPayload::Platform(dioxus_html::PlatformEventData::new(
                    Box::new(()),
                )),
                containment: Containment::RootMissing,
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
    fn reports_unsupported_without_a_document() {
        let mut dom = VirtualDom::new(|| {
            let support = crate::use_document_listener!(keydown, move |event| {
                let _ = event.key();
            });
            assert_eq!(support, GlobalListenerSupport::Unsupported);
            VNode::empty()
        });
        dom.rebuild(&mut NoOpMutations);
        dom.process_events();
    }
}
