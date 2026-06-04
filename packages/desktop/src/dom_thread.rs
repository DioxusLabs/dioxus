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
use crate::file_upload::NativeFileHover;
use crate::ipc::UserWindowEvent;
use crate::shortcut::HotKeyState;
use dioxus_core::{ScopeId, VirtualDom, provide_context};
use dioxus_history::{History, MemoryHistory};
use dioxus_interpreter_js::MutationState;
use futures_channel::oneshot;
use futures_util::FutureExt;
use futures_util::future::OptionFuture;
use slab::Slab;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{any::Any, cell::RefCell, collections::HashMap, future::Future, pin::Pin, rc::Rc};
use tao::{event_loop::EventLoopProxy, window::WindowId};
use tokio::sync::mpsc::{self as tokio_mpsc, UnboundedSender};
use tokio::task::AbortHandle;
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
    RunCallback(Box<dyn FnOnce(&mut DomCallbackRegistry) + Send>),
}

type AssetHandlerCallback = Box<dyn Fn(AssetRequest, RequestAsyncResponder)>;
type ShortcutCallback = Box<dyn FnMut(HotKeyState)>;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
type EventHandlerCallback = Box<dyn FnMut(UserWindowEvent)>;

/// A wry event handler whose closure stays on the DOM thread. It is invoked with a borrowed event
/// while the main thread is blocked, so it never needs to be `Send` or own a `'static` event.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
type WryEventHandlerCallback =
    Box<dyn for<'a> FnMut(tao::event::Event<'a, ()>) -> tao::event::Event<'a, ()>>;

/// Unique identifier for a callback stored on the DOM thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DomCallbackId(pub usize);

pub(crate) type SharedCallbackRegistry = Rc<RefCell<DomCallbackRegistry>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PendingDomId(usize);

thread_local! {
    static PENDING_DOMS: RefCell<HashMap<usize, VirtualDom>> = RefCell::new(HashMap::new());
}

static NEXT_PENDING_DOM_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn reserve_pending_dom(dom: VirtualDom) -> PendingDomId {
    let id = NEXT_PENDING_DOM_ID.fetch_add(1, Ordering::Relaxed);
    PENDING_DOMS.with(|doms| {
        doms.borrow_mut().insert(id, dom);
    });
    PendingDomId(id)
}

fn take_pending_dom(id: PendingDomId) -> VirtualDom {
    PENDING_DOMS
        .with(|doms| doms.borrow_mut().remove(&id.0))
        .expect("Pending VirtualDom should exist on the DOM thread")
}

/// Registry for callbacks that live on the DOM thread.
///
/// This registry stores non-Send closures that are invoked via the inverted
/// callback pattern. The main thread sends requests to invoke these callbacks,
/// and the DOM thread looks them up and executes them.
pub(crate) struct DomCallbackRegistry {
    /// Callback storage for asset handlers, shortcut callbacks, and event handlers.
    callbacks: Slab<Box<dyn Any>>,
    /// Asset handler names point into the shared callback slab.
    asset_handler_names: HashMap<String, DomCallbackId>,
}

impl Default for DomCallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DomCallbackRegistry {
    /// Create a new empty callback registry.
    pub fn new() -> Self {
        Self {
            callbacks: Slab::new(),
            asset_handler_names: HashMap::new(),
        }
    }

    fn insert_callback<T: 'static>(&mut self, callback: T) -> DomCallbackId {
        DomCallbackId(self.callbacks.insert(Box::new(callback)))
    }

    fn remove_callback(&mut self, id: DomCallbackId) -> Option<()> {
        self.callbacks.try_remove(id.0).map(|_| ())
    }

    fn callback_mut<T: 'static>(&mut self, id: DomCallbackId) -> Option<&mut T> {
        self.callbacks.get_mut(id.0)?.downcast_mut::<T>()
    }

    fn invoke<T: 'static, O>(&mut self, id: DomCallbackId, f: impl FnOnce(&mut T) -> O) -> O {
        let value = self
            .callback_mut::<T>(id)
            .expect("type id should match expected type");
        f(value)
    }

    /// Register an asset handler.
    pub fn register_asset_handler(&mut self, name: String, handler: AssetHandlerCallback) {
        let id = self.insert_callback(handler);
        if let Some(old_id) = self.asset_handler_names.insert(name, id) {
            self.remove_callback(old_id);
        }
    }

    /// Remove an asset handler.
    pub fn remove_asset_handler(&mut self, name: &str) -> Option<()> {
        let id = self.asset_handler_names.remove(name)?;
        self.remove_callback(id)
    }

    /// Invoke an asset handler if it exists.
    pub fn invoke_asset_handler(
        &mut self,
        name: &str,
        request: AssetRequest,
        responder: RequestAsyncResponder,
    ) -> bool {
        let Some(id) = self.asset_handler_names.get(name).copied() else {
            return false;
        };

        self.invoke::<AssetHandlerCallback, _>(id, |handler| handler(request, responder));
        true
    }

    /// Register a shortcut callback and return its ID.
    pub fn register_shortcut_callback(&mut self, callback: ShortcutCallback) -> DomCallbackId {
        self.insert_callback(callback)
    }

    /// Remove a shortcut callback.
    pub fn remove_shortcut_callback(&mut self, id: DomCallbackId) -> Option<()> {
        self.remove_callback(id)
    }

    /// Invoke a shortcut callback if it exists.
    pub fn invoke_shortcut_callback(&mut self, id: DomCallbackId, state: HotKeyState) {
        self.invoke::<ShortcutCallback, _>(id, |callback| callback(state))
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn register_event_handler(&mut self, callback: EventHandlerCallback) -> DomCallbackId {
        self.insert_callback(callback)
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn remove_event_handler(&mut self, id: DomCallbackId) -> Option<()> {
        self.remove_callback(id)
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn invoke_event_handler(&mut self, id: DomCallbackId, event: UserWindowEvent) {
        self.invoke::<EventHandlerCallback, _>(id, |callback| callback(event))
    }

    /// Register a wry event handler whose closure stays on the DOM thread.
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn register_wry_event_handler(
        &mut self,
        callback: WryEventHandlerCallback,
    ) -> DomCallbackId {
        self.insert_callback(callback)
    }

    /// Invoke a DOM-thread wry event handler with a borrowed event, if it exists.
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn invoke_wry_event_handler<'a>(
        &mut self,
        id: DomCallbackId,
        event: tao::event::Event<'a, ()>,
    ) -> tao::event::Event<'a, ()> {
        self.invoke::<WryEventHandlerCallback, _>(id, |callback| callback(event))
    }
}

/// Handle to communicate with a VirtualDom running on a dedicated thread.
///
/// This handle only contains the sender for sending events to the VirtualDom. The VirtualDom
/// thread sends its rendered edits straight to the webview's edit websocket, so there is no
/// command channel back to the main thread.
#[derive(Clone)]
pub(crate) struct VirtualDomHandle {
    /// Send events to the VirtualDom thread.
    pub event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>,
}

impl VirtualDomHandle {
    /// Create a new handle with the given event sender.
    pub fn new(event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>) -> Self {
        Self { event_tx }
    }

    /// Send an event to the VirtualDom thread.
    pub fn send_event(&self, event: VirtualDomEvent) {
        let _ = self.event_tx.send(event);
    }
}

/// Run the VirtualDom in the current async context (called from wry-bindgen app thread).
///
/// This creates the VirtualDom and runs its event loop until completion.
/// Also sets up the wasm-bindgen event handler for direct JS->Rust event calls.
pub(crate) async fn run_virtual_dom<F>(
    make_dom: F,
    event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>,
    websocket: EditWebsocket,
    webview_id: u32,
    proxy: EventLoopProxy<UserWindowEvent>,
    window_id: WindowId,
    file_hover: NativeFileHover,
) where
    F: FnOnce() -> VirtualDom + Send + 'static,
{
    let dom = make_dom();
    run_virtual_dom_with_dom(
        dom, event_rx, event_tx, websocket, webview_id, proxy, window_id, file_hover,
    )
    .await;
}

pub(crate) async fn run_pending_virtual_dom(
    pending_dom_id: PendingDomId,
    event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>,
    websocket: EditWebsocket,
    webview_id: u32,
    proxy: EventLoopProxy<UserWindowEvent>,
    window_id: WindowId,
    file_hover: NativeFileHover,
) {
    let dom = take_pending_dom(pending_dom_id);
    run_virtual_dom_with_dom(
        dom, event_rx, event_tx, websocket, webview_id, proxy, window_id, file_hover,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn run_virtual_dom_with_dom(
    dom: VirtualDom,
    event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>,
    websocket: EditWebsocket,
    webview_id: u32,
    proxy: EventLoopProxy<UserWindowEvent>,
    window_id: WindowId,
    file_hover: NativeFileHover,
) {
    crate::wry_bindgen_bridge::setup_event_handler(dom.runtime(), file_hover);
    let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
    let desktop_service_proxy = DesktopContext::new(proxy, window_id, event_tx);

    // Create the callback registry for the inverted callback pattern
    let callback_registry = Rc::new(RefCell::new(DomCallbackRegistry::new()));

    dom.in_scope(ScopeId::ROOT, || {
        provide_context(history_provider);
        provide_context(Rc::new(DesktopDocument::new(desktop_service_proxy.clone()))
            as Rc<dyn dioxus_document::Document>);
        provide_context(desktop_service_proxy);
        provide_context(callback_registry.clone());
    });
    run_virtual_dom_loop(dom, event_rx, websocket, webview_id, callback_registry).await;
}

/// The main event loop for the VirtualDom running on its dedicated thread.
async fn run_virtual_dom_loop(
    mut dom: VirtualDom,
    mut event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    websocket: EditWebsocket,
    webview_id: u32,
    callback_registry: SharedCallbackRegistry,
) {
    let mut mutations = MutationState::default();
    // The receiver for the edits we are currently waiting on the webview to apply. While this is
    // `Some`, we hold off rendering new work so effects don't run before the DOM has updated. The
    // websocket worker resolves it once the webview acks the edits (or drops it if the connection
    // goes away).
    let mut pending_flush: Option<oneshot::Receiver<()>> = None;
    let mut initialized = false;

    loop {
        // Normal operation: wait for work or events
        tokio::select! {
            biased;

            // Check for incoming events from main thread first
            event = event_rx.recv() => {
                let Some(event) = event else {
                    // Channel closed
                    return;
                };
                match event {
                    VirtualDomEvent::Initialize => {
                        if !initialized {
                            initialized = true;
                            // Perform initial rebuild
                            dom.rebuild(&mut mutations);
                            let edits = take_edits(&mut mutations);
                            pending_flush = Some(websocket.send_edits(webview_id, edits));
                        }
                    }
                    #[cfg(all(feature = "devtools", debug_assertions))]
                    VirtualDomEvent::HotReload(msg) => {
                        dioxus_devtools::apply_changes(&dom, &msg);
                    }
                    VirtualDomEvent::RunCallback(callback) => {
                        // Run the callback with access to the registry
                        let mut registry = callback_registry.borrow_mut();
                        callback(&mut registry);
                    }
                }
            }

            // The webview applied the in-flight edits (Ok) or the connection dropped them (Err).
            // Either way we are no longer waiting, so rendering can resume.
            Some(_) = OptionFuture::from(pending_flush.as_mut()) => {
                pending_flush = None;
            }

            // Wait for the VirtualDom to have work ready
            _ = dom.wait_for_work(), if initialized && pending_flush.is_none() => {
                // Render and send mutations straight to the webview's edit websocket
                dom.render_immediate(&mut mutations);
                let edits = take_edits(&mut mutations);
                pending_flush = Some(websocket.send_edits(webview_id, edits));
            }
        }
    }
}

/// Export mutations from the MutationState.
fn take_edits(mutations: &mut MutationState) -> Vec<u8> {
    mutations.export_memory()
}

type SpawnTask = (
    WindowId,
    Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()>>> + Send>,
);
type TaskSender = UnboundedSender<SpawnTask>;

/// Handle to spawn tasks on the dom thread and abort them by window ID.
pub(crate) struct DomThreadHandle {
    /// Channel to send tasks to spawn (with associated window ID).
    pub task_tx: TaskSender,
    /// Channel to request task abortion by window ID.
    pub abort_tx: UnboundedSender<WindowId>,
}

/// Spawn a thread that runs async tasks and supports aborting them by window ID.
pub(crate) fn spawn_dom_thread(proxy: EventLoopProxy<UserWindowEvent>) -> DomThreadHandle {
    let (task_tx, mut task_rx): (TaskSender, _) = tokio::sync::mpsc::unbounded_channel();
    let (abort_tx, mut abort_rx): (UnboundedSender<WindowId>, _) =
        tokio::sync::mpsc::unbounded_channel();

    std::thread::Builder::new()
        .name("dioxus-desktop-dom".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for VirtualDom thread");

            runtime.block_on(async move {
                tokio::task::LocalSet::new()
                    .run_until(async {
                        let mut abort_handles: HashMap<WindowId, AbortHandle> = HashMap::new();

                        loop {
                            tokio::select! {
                                biased;

                                // Handle abort requests with priority
                                Some(window_id) = abort_rx.recv() => {
                                    if let Some(handle) = abort_handles.remove(&window_id) {
                                        handle.abort();
                                    }
                                }

                                // Handle new task spawns
                                spawn_result = task_rx.recv() => {
                                    let Some((window_id, spawn_task)) = spawn_result else {
                                        // Channel closed, exit the loop
                                        break;
                                    };
                                    let fut = spawn_task();
                                    let proxy = proxy.clone();
                                    let join_handle = tokio::task::spawn_local(async move {
                                        _ = AssertUnwindSafe(fut).catch_unwind().await;
                                        // Close the window when the task completes (aborted or finished)
                                        _ = proxy.send_event(UserWindowEvent::close_window(window_id));
                                    });
                                    abort_handles.insert(window_id, join_handle.abort_handle());
                                }
                            }
                        }
                    })
                    .await;
            });
        })
        .expect("Failed to spawn VirtualDom thread");

    DomThreadHandle { task_tx, abort_tx }
}
