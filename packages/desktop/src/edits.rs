use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    task::Waker,
};

use dioxus_interpreter_js::MutationState;

/// This handles communication between the requests that the webview makes and the interpreter. The interpreter constantly makes long running requests to the webview to get any edits that should be made to the DOM almost like server side events.
/// It will hold onto the requests until the interpreter is ready to handle them and hold onto any pending edits until a new request is made.
#[derive(Default, Clone)]
pub(crate) struct WryQueue {
    inner: Arc<RwLock<WryQueueInner>>,
}

#[derive(Default)]
pub(crate) struct WryQueueInner {
    edit_queue: VecDeque<Vec<u8>>,
    edit_responder: Option<wry::RequestAsyncResponder>,
    // Stores any futures waiting for edits to be applied to the webview
    // NOTE: We don't use a Notify here because we need polling the notify to be cancel safe
    waiting_for_edits_flushed: Vec<Waker>,
    // If this webview is currently waiting for an edit to be flushed. We don't run the virtual dom while this is true to avoid running effects before the dom has been updated
    edits_in_progress: bool,
    mutation_state: MutationState,
}

impl WryQueue {
    pub fn handle_request(&self, responder: wry::RequestAsyncResponder) {
        let mut myself = self.inner.write().unwrap();
        if let Some(bytes) = myself.edit_queue.pop_back() {
            responder.respond(wry::http::Response::new(bytes));
        } else {
            // There are now no edits that need to be applied to the webview
            for waker in myself.waiting_for_edits_flushed.drain(..) {
                waker.wake();
            }
            myself.edits_in_progress = false;
            myself.edit_responder = Some(responder);
        }
    }

    pub fn with_mutation_state_mut<O: 'static>(
        &self,
        f: impl FnOnce(&mut MutationState) -> O,
    ) -> O {
        let mut inner = self.inner.write().unwrap();
        f(&mut inner.mutation_state)
    }

    /// Send a list of mutations to the webview
    pub(crate) fn send_edits(&self) {
        let mut myself = self.inner.write().unwrap();
        let serialized_edits = myself.mutation_state.export_memory();
        // There are pending edits that need to be applied to the webview before we run futures
        myself.edits_in_progress = true;
        if let Some(responder) = myself.edit_responder.take() {
            responder.respond(wry::http::Response::new(serialized_edits));
        } else {
            myself.edit_queue.push_front(serialized_edits);
        }
    }

    fn edits_in_progress(&self) -> bool {
        self.inner.read().unwrap().edits_in_progress
    }

    /// Wait until all pending edits have been rendered in the webview
    pub fn poll_edits_flushed(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        if self.edits_in_progress() {
            let mut myself = self.inner.write().unwrap();
            let waker = cx.waker();
            myself.waiting_for_edits_flushed.push(waker.clone());
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    }
}
