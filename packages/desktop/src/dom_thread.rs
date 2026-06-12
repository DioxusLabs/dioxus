//! VirtualDom thread management for desktop.
//!
//! This module handles running the VirtualDom on a dedicated thread to improve
//! responsiveness. The main thread continues to run the tao event loop and manage
//! windows, while VirtualDom polling and rendering happens as separate tasks on
//! a dedicated DOM thread.

use crate::AssetRequest;
use crate::desktop_context::DesktopContext;
use crate::document::DesktopDocument;
use crate::edits::EditWebsocket;
use crate::event_handlers::WryEventHandler;
use crate::file_upload::NativeFileHover;
use crate::ipc::{UserWindowEvent, UserWindowEventVariant, WindowHandle};
use crate::shortcut::ShortcutHandle;
use dioxus_core::{ElementId, ScopeId, VirtualDom, provide_context};
use dioxus_history::{History, MemoryHistory};
use dioxus_interpreter_js::MutationState;
use dioxus_web_sys_events::QueueMountedEvents;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_channel::oneshot;
use futures_util::future::{AbortHandle, Abortable, OptionFuture};
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use slotmap::SlotMap;
use std::panic::AssertUnwindSafe;
use std::{any::Any, cell::RefCell, collections::HashMap, future::Future, pin::Pin, rc::Rc};
use tao::{event_loop::EventLoopProxy, window::WindowId};
use wry::RequestAsyncResponder;

/// Events sent from the main thread to the VirtualDom thread.
pub(crate) enum VirtualDomEvent {
    /// Initialize the VirtualDom (perform initial rebuild).
    Initialize,

    /// Hot reload message from devtools.
    #[cfg(all(feature = "devtools", debug_assertions))]
    HotReload(dioxus_devtools::HotReloadMsg),

    /// Run a callback on the DOM thread.
    ///
    /// This is used for the inverted callback pattern where closures stay on the
    /// DOM thread and the main thread invokes them via message passing.
    RunCallback(Box<dyn FnOnce(&SharedCallbackRegistry) + Send>),
}

/// A type-erased callback stored on the DOM thread. The wrapper created in
/// [`SharedCallbackRegistry::register`] downcasts the argument back to its concrete type.
type StoredCallback = Box<dyn FnMut(Box<dyn Any>)>;

slotmap::new_key_type! {
    /// Unique identifier for a callback stored on the DOM thread. Ids are generational: a
    /// removed callback's id can never dispatch to a different callback reusing its slot.
    pub struct DomCallbackId;
}

/// A shared handle to the DOM thread's [`DomCallbackRegistry`], shared by every window. One
/// generational keyspace means a stale id can never reach another window's callback: removing an
/// already-purged callback is a no-op by construction.
///
/// The registry is owned by the DOM thread's message loop ([`spawn_dom_thread`]); clones are
/// threaded into every window task and [`DesktopContext`]. The `Rc` keeps every handle pinned to
/// the DOM thread, where the `!Send` callbacks live.
///
/// Invocation never holds the inner `RefCell` borrow while user code runs, so callbacks can
/// register/remove callbacks freely.
#[derive(Clone, Default)]
pub(crate) struct SharedCallbackRegistry(Rc<RefCell<DomCallbackRegistry>>);

/// The name of the dedicated DOM thread spawned by [`spawn_dom_thread`].
const DOM_THREAD_NAME: &str = "dioxus-desktop-dom";

/// Registry for callbacks that live on the DOM thread, shared by every window.
///
/// This registry stores non-Send closures that are invoked via the inverted
/// callback pattern. The main thread sends requests to invoke these callbacks,
/// and the DOM thread looks them up and executes them.
#[derive(Default)]
pub(crate) struct DomCallbackRegistry {
    /// Callback storage, tagged with the owning window so
    /// [`SharedCallbackRegistry::remove_window`] can purge a window's callbacks wholesale. A
    /// slot's callback is `None` while it is checked out for invocation; the entry stays in the
    /// map so the id remains reserved until it is restored.
    callbacks: SlotMap<DomCallbackId, (WindowId, Option<StoredCallback>)>,
    /// Asset handler names point into the shared callback map. Names are per window: two windows
    /// may register handlers under the same name without clobbering each other.
    asset_handler_names: HashMap<(WindowId, String), DomCallbackId>,
    /// Shortcut handles point into the shared callback map.
    shortcut_handlers: HashMap<ShortcutHandle, DomCallbackId>,
    /// Wry event handler ids point into the shared callback map. Only handlers with a DOM-thread
    /// callback (created with [`DesktopContext::create_wry_event_handler`] or the wry event
    /// hooks) have an entry here.
    wry_event_handlers: HashMap<WryEventHandler, DomCallbackId>,
}

/// Erase a callback's argument type so every callback kind shares one storage type. The wrapper
/// restores the concrete type on invocation.
fn erase<A: 'static>(mut f: impl FnMut(A) + 'static) -> StoredCallback {
    Box::new(move |arg| match arg.downcast::<A>() {
        Ok(arg) => f(*arg),
        Err(_) => tracing::warn!("dom callback invoked with an unexpected argument type"),
    })
}

impl SharedCallbackRegistry {
    /// Register a callback owned by `window_id` taking an argument of type `A`. Invoke it with
    /// [`Self::invoke`]`(id, arg)`.
    pub fn register<A: 'static>(
        &self,
        window_id: WindowId,
        f: impl FnMut(A) + 'static,
    ) -> DomCallbackId {
        self.0
            .borrow_mut()
            .callbacks
            .insert((window_id, Some(erase(f))))
    }

    /// Remove a callback. Works even while the callback is checked out for invocation; the
    /// in-flight restore will see the missing entry and drop the closure.
    pub fn remove(&self, id: DomCallbackId) -> Option<()> {
        self.0.borrow_mut().callbacks.remove(id).map(|_| ())
    }

    /// Associate a shortcut with its DOM-thread callback. Shortcut handles are generational, so
    /// a handle can never be registered twice.
    pub fn register_shortcut_handler(&self, shortcut: ShortcutHandle, id: DomCallbackId) {
        self.0.borrow_mut().shortcut_handlers.insert(shortcut, id);
    }

    /// Remove the DOM-thread callback associated with a shortcut.
    pub fn remove_shortcut_handler(&self, shortcut: ShortcutHandle) -> Option<()> {
        let mut registry = self.0.borrow_mut();
        let id = registry.shortcut_handlers.remove(&shortcut)?;
        registry.callbacks.remove(id).map(|_| ())
    }

    /// Associate a wry event handler with its DOM-thread callback. Handler ids are generational,
    /// so a handler can never be registered twice.
    pub fn register_wry_event_handler(&self, handler: WryEventHandler, id: DomCallbackId) {
        self.0.borrow_mut().wry_event_handlers.insert(handler, id);
    }

    /// Remove the DOM-thread callback associated with a wry event handler.
    pub fn remove_wry_event_handler(&self, handler: WryEventHandler) -> Option<()> {
        let mut registry = self.0.borrow_mut();
        let id = registry.wry_event_handlers.remove(&handler)?;
        registry.callbacks.remove(id).map(|_| ())
    }

    /// Remove every shortcut callback, matching the main thread's global shortcut registry.
    pub fn remove_all_shortcut_handlers(&self) {
        let mut registry = self.0.borrow_mut();
        let shortcut_handlers = std::mem::take(&mut registry.shortcut_handlers);
        for id in shortcut_handlers.values() {
            registry.callbacks.remove(*id);
        }
    }

    /// Drop every callback `window_id` registered and prune the indexes that pointed at them.
    ///
    /// A window's callbacks intentionally outlive its VirtualDom task: hook cleanup runs while
    /// the task is being dropped, so this runs after the abort message for the window, and
    /// removing an already-purged callback is a no-op.
    pub fn remove_window(&self, window_id: WindowId) {
        let mut registry = self.0.borrow_mut();
        let DomCallbackRegistry {
            callbacks,
            asset_handler_names,
            shortcut_handlers,
            wry_event_handlers,
        } = &mut *registry;
        callbacks.retain(|_, (owner, _)| *owner != window_id);
        asset_handler_names.retain(|_, id| callbacks.contains_key(*id));
        shortcut_handlers.retain(|_, id| callbacks.contains_key(*id));
        wry_event_handlers.retain(|_, id| callbacks.contains_key(*id));
    }

    /// Invoke a callback without holding the registry borrow while user code runs.
    ///
    /// The callback is taken out of the registry, invoked, and restored afterwards, so the
    /// callback itself may register or remove callbacks (including itself). Returns `false` if
    /// the callback is missing (already removed, or currently running reentrantly).
    pub fn invoke<A: 'static>(&self, id: DomCallbackId, arg: A) -> bool {
        let taken = self
            .0
            .borrow_mut()
            .callbacks
            .get_mut(id)
            .and_then(|(_, slot)| slot.take());
        let Some(mut callback) = taken else {
            tracing::warn!(
                "dropping invocation of dom callback {id:?}: it was removed or is already running"
            );
            return false;
        };
        callback(Box::new(arg));
        // Put the callback back, unless it was removed (or replaced) while it ran.
        if let Some((_, slot @ None)) = self.0.borrow_mut().callbacks.get_mut(id) {
            *slot = Some(callback);
        }
        true
    }

    /// Register an asset handler under a name, replacing any previous handler the window
    /// registered with that name. Returns the callback id the main-thread forwarder invokes
    /// (with an `(AssetRequest, RequestAsyncResponder)` argument).
    pub fn register_asset_handler(
        &self,
        window_id: WindowId,
        name: String,
        mut handler: impl FnMut(AssetRequest, RequestAsyncResponder) + 'static,
    ) -> DomCallbackId {
        let id = self.register(window_id, move |(request, responder)| {
            handler(request, responder)
        });
        let mut registry = self.0.borrow_mut();
        if let Some(old_id) = registry.asset_handler_names.insert((window_id, name), id) {
            registry.callbacks.remove(old_id);
        }
        id
    }

    /// Remove an asset handler.
    pub fn remove_asset_handler(&self, window_id: WindowId, name: &str) -> Option<()> {
        let id = self
            .0
            .borrow_mut()
            .asset_handler_names
            .remove(&(window_id, name.to_string()))?;
        self.remove(id)
    }
}

/// Run the VirtualDom's event loop until completion in the current async context (called from
/// the wry-bindgen app task on the DOM thread). Also sets up the wasm-bindgen event handler for
/// direct JS->Rust event calls.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_virtual_dom_with_dom(
    dom: VirtualDom,
    event_rx: UnboundedReceiver<VirtualDomEvent>,
    event_tx: UnboundedSender<VirtualDomEvent>,
    websocket: EditWebsocket,
    webview_id: u32,
    window_handle: WindowHandle,
    file_hover: NativeFileHover,
    callbacks: SharedCallbackRegistry,
) {
    crate::wry_bindgen_bridge::setup_event_handler(dom.runtime(), file_hover);
    let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
    let desktop_service_proxy = DesktopContext::new(event_tx, window_handle, callbacks.clone());

    dom.in_scope(ScopeId::ROOT, || {
        provide_context(history_provider);
        provide_context(Rc::new(DesktopDocument::new(desktop_service_proxy.clone()))
            as Rc<dyn dioxus_document::Document>);
        provide_context(desktop_service_proxy);
    });
    run_virtual_dom_loop(dom, event_rx, websocket, webview_id, callbacks).await;
}

/// The main event loop for the VirtualDom running on its dedicated thread.
async fn run_virtual_dom_loop(
    mut dom: VirtualDom,
    mut event_rx: UnboundedReceiver<VirtualDomEvent>,
    websocket: EditWebsocket,
    webview_id: u32,
    callback_registry: SharedCallbackRegistry,
) {
    let mut mutations = QueueMountedEvents::new(MutationState::default());
    // The receiver for the edits we are currently waiting on the webview to apply. While this is
    // `Some`, we hold off rendering new work so effects don't run before the DOM has updated. The
    // websocket worker resolves it once the webview acks the edits (or drops it if the connection
    // goes away).
    let mut pending_flush: Option<PendingFlush> = None;
    let mut initialized = false;

    loop {
        // Normal operation: wait for work or events. `select_biased!` polls the arms in
        // priority order: main-thread events first, then the in-flight edit ack, then
        // VirtualDom work (only while initialized and not waiting on a flush — an arm whose
        // `OptionFuture` is `None` counts as terminated, so it is never polled).
        //
        // The arm futures must be written inline: the macro binds inline expressions in a
        // scope that ends before the arm bodies run, so their borrows of `dom` and
        // `pending_flush` are released by the time the bodies use them.
        let waiting_for_work = initialized && pending_flush.is_none();

        futures_util::select_biased! {
            // Check for incoming events from main thread first
            event = event_rx.next() => {
                let Some(event) = event else {
                    // Channel closed
                    return;
                };
                match event {
                    // The webview sends initialize on every page load, so a reload triggers a
                    // fresh rebuild for the new document
                    VirtualDomEvent::Initialize => {
                        initialized = true;
                        // Perform initial rebuild
                        dom.rebuild(&mut mutations);
                        pending_flush = Some(send_edits(&websocket, webview_id, &mut mutations));
                    }
                    #[cfg(all(feature = "devtools", debug_assertions))]
                    VirtualDomEvent::HotReload(msg) => {
                        dioxus_devtools::apply_changes(&dom, &msg);
                    }
                    VirtualDomEvent::RunCallback(callback) => {
                        // Run the callback with access to the registry. The registry handle is
                        // passed by reference (not a borrow of its RefCell) so the callback can
                        // register and remove other callbacks while it runs.
                        callback(&callback_registry);
                    }
                }
            }

            // The webview applied the in-flight edits (Ok) or the connection dropped them (Err).
            // Either way we are no longer waiting, so rendering can resume.
            applied = OptionFuture::from(pending_flush.as_mut().map(|flush| &mut flush.applied)) => {
                let applied = applied.expect("this arm only fires while a flush is pending");
                let flush = pending_flush
                    .take()
                    .expect("this arm only fires while a flush is pending");
                if applied.is_ok() {
                    let runtime = dom.runtime();
                    for id in flush.mounted_events {
                        dioxus_web_sys_events::handle_mounted_event(&runtime, id);
                    }
                }
            }

            // Wait for the VirtualDom to have work ready
            _ = OptionFuture::from(waiting_for_work.then(|| dom.wait_for_work().fuse())) => {
                // Render and send mutations straight to the webview's edit websocket
                dom.render_immediate(&mut mutations);
                pending_flush = Some(send_edits(&websocket, webview_id, &mut mutations));
            }
        }
    }
}

struct PendingFlush {
    applied: oneshot::Receiver<()>,
    mounted_events: Vec<ElementId>,
}

fn send_edits(
    websocket: &EditWebsocket,
    webview_id: u32,
    mutations: &mut QueueMountedEvents<MutationState>,
) -> PendingFlush {
    let edits = mutations.export_memory();
    let mounted_events = mutations.take_mounted_events();
    PendingFlush {
        applied: websocket.send_edits(webview_id, edits),
        mounted_events,
    }
}

/// Messages for the dom thread. Spawns and aborts share one channel so they arrive in the order
/// the main thread sent them: an abort can never overtake its window's spawn message and get
/// dropped, which would leave an unabortable task (and its window) alive forever.
/// Builds a window's VirtualDom task. Built on the main thread but run on the DOM thread, which
/// hands it the callback registry it owns.
pub(crate) type SpawnTask =
    Box<dyn FnOnce(SharedCallbackRegistry) -> Pin<Box<dyn Future<Output = ()>>> + Send>;

pub(crate) enum DomThreadMessage {
    /// Spawn the VirtualDom task for a window.
    Spawn(WindowId, SpawnTask),
    /// Abort the VirtualDom task for a window.
    Abort(WindowId),
    /// Drop a window's callbacks once the window has closed.
    RemoveWindowCallbacks(WindowId),
}

/// Handle to spawn tasks on the dom thread and abort them by window ID.
pub(crate) struct DomThreadHandle {
    pub tx: UnboundedSender<DomThreadMessage>,
}

/// The DOM thread's main loop as a future, handed to a [`Config::new_with_dom_thread_driver`]
/// closure to drive on the executor of its choice. The future is `!Send`: it must be driven on
/// the thread the closure is called on, to completion.
///
/// [`Config::new_with_dom_thread_driver`]: crate::Config::new_with_dom_thread_driver
pub type DomThreadFuture = Pin<Box<dyn Future<Output = ()>>>;

/// The executor for the DOM thread (see [`DomThreadFuture`]).
pub(crate) type DomThreadDriver = Box<dyn FnOnce(DomThreadFuture) + Send>;

/// The default DOM thread driver: a dedicated current-thread Tokio runtime, so component
/// futures can use Tokio timers and IO.
#[cfg(feature = "tokio_runtime")]
pub(crate) fn default_tokio_driver() -> DomThreadDriver {
    Box::new(|main_loop| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build the Tokio runtime for the DOM thread")
            .block_on(main_loop)
    })
}

/// Spawn a thread that runs async tasks and supports aborting them by window ID.
///
/// The thread's whole workload is a single future: the message loop multiplexes the window
/// VirtualDom tasks itself instead of spawning them on an executor, so `driver` can run it on
/// any runtime that blocks the thread on a `!Send` future.
pub(crate) fn spawn_dom_thread(
    proxy: EventLoopProxy<UserWindowEvent>,
    driver: DomThreadDriver,
) -> DomThreadHandle {
    let (tx, mut rx) = unbounded();

    std::thread::Builder::new()
        .name(DOM_THREAD_NAME.into())
        .spawn(move || {
            let main_loop: DomThreadFuture = Box::pin(async move {
                // The callback registry for every window, owned by this loop and handed
                // to each spawned task (see [`SharedCallbackRegistry`]).
                let callbacks = SharedCallbackRegistry::default();
                let abort_handles: Rc<RefCell<HashMap<WindowId, AbortHandle>>> =
                    Rc::new(RefCell::new(HashMap::new()));
                // The window VirtualDom tasks.
                let mut tasks = FuturesUnordered::new();

                loop {
                    futures_util::select_biased! {
                        message = rx.next() => {
                            let Some(message) = message else {
                                // Channel closed: the app is shutting down.
                                return;
                            };
                            match message {
                                DomThreadMessage::Abort(window_id) => {
                                    // A missing handle means the task already finished naturally.
                                    let handle = abort_handles.borrow_mut().remove(&window_id);
                                    if let Some(handle) = handle {
                                        handle.abort();
                                    }
                                }

                                DomThreadMessage::Spawn(window_id, spawn_task) => {
                                    let fut = spawn_task(callbacks.clone());
                                    let proxy = proxy.clone();
                                    let handles = abort_handles.clone();
                                    let (abort_handle, abort_registration) = AbortHandle::new_pair();
                                    tasks.push(
                                        async move {
                                            let result = Abortable::new(
                                                AssertUnwindSafe(fut).catch_unwind(),
                                                abort_registration,
                                            )
                                            .await;
                                            // Runs when the VirtualDom task finishes or panics (never
                                            // on abort). Start tearing down the window: hiding a
                                            // window whose VirtualDom is gone would leave a zombie.
                                            if result.is_ok() {
                                                handles.borrow_mut().remove(&window_id);
                                                _ = proxy.send_event(
                                                    UserWindowEventVariant::DestroyWindow(window_id).into(),
                                                );
                                            }
                                        }
                                        .boxed_local(),
                                    );
                                    abort_handles.borrow_mut().insert(window_id, abort_handle);
                                }

                                DomThreadMessage::RemoveWindowCallbacks(window_id) => {
                                    callbacks.remove_window(window_id);
                                }
                            }
                        }

                        // A window task finished; its wrapper above already queued the window
                        // teardown. An empty `FuturesUnordered` counts as terminated, so this
                        // arm is skipped while no window is running.
                        _ = tasks.next() => {}
                    }
                }
            });

            driver(main_loop);
        })
        .expect("Failed to spawn VirtualDom thread");

    DomThreadHandle { tx }
}
