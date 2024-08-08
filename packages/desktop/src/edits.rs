use std::cell::Cell;
use std::{cell::RefCell, collections::VecDeque, rc::Rc, task::Waker};

/// This handles communication between the requests that the webview makes and the interpreter. The interpreter constantly makes long running requests to the webview to get any edits that should be made to the DOM almost like server side events.
/// It will hold onto the requests until the interpreter is ready to handle them and hold onto any pending edits until a new request is made.
#[derive(Default, Clone)]
pub(crate) struct EditQueue {
    queue: Rc<RefCell<VecDeque<Vec<u8>>>>,
    responder: Rc<RefCell<Option<wry::RequestAsyncResponder>>>,
    // Stores any futures waiting for edits to be applied to the webview
    // NOTE: We don't use a Notify here because we need polling the notify to be cancel safe
    waiting_for_edits_flushed: Rc<RefCell<Vec<Waker>>>,
    // If this webview is currently waiting for an edit to be flushed. We don't run the virtual dom while this is true to avoid running effects before the dom has been updated
    edits_in_progress: Rc<Cell<bool>>,
}

impl EditQueue {
    pub fn handle_request(&self, responder: wry::RequestAsyncResponder) {
        let mut queue = self.queue.borrow_mut();
        if let Some(bytes) = queue.pop_back() {
            responder.respond(wry::http::Response::new(bytes));
        } else {
            // There are now no edits that need to be applied to the webview
            self.edits_finished();
            *self.responder.borrow_mut() = Some(responder);
        }
    }

    pub fn add_edits(&self, edits: Vec<u8>) {
        let mut responder = self.responder.borrow_mut();
        // There are pending edits that need to be applied to the webview before we run futures
        self.start_edits();
        if let Some(responder) = responder.take() {
            responder.respond(wry::http::Response::new(edits));
        } else {
            self.queue.borrow_mut().push_front(edits);
        }
    }

    fn start_edits(&self) {
        self.edits_in_progress.set(true);
    }

    fn edits_finished(&self) {
        for waker in self.waiting_for_edits_flushed.borrow_mut().drain(..) {
            waker.wake();
        }
        self.edits_in_progress.set(false);
    }

    fn edits_in_progress(&self) -> bool {
        self.edits_in_progress.get()
    }

    /// Wait until all pending edits have been rendered in the webview
    pub fn poll_edits_flushed(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        if self.edits_in_progress() {
            let waker = cx.waker();
            self.waiting_for_edits_flushed
                .borrow_mut()
                .push(waker.clone());
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    }
}
