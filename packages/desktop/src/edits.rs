use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use tokio::sync::Notify;

/// This handles communication between the requests that the webview makes and the interpreter. The interpreter constantly makes long running requests to the webview to get any edits that should be made to the DOM almost like server side events.
/// It will hold onto the requests until the interpreter is ready to handle them and hold onto any pending edits until a new request is made.
#[derive(Default, Clone)]
pub(crate) struct EditQueue {
    queue: Rc<RefCell<VecDeque<Vec<u8>>>>,
    responder: Rc<RefCell<Option<wry::RequestAsyncResponder>>>,
    edit_added: Rc<Notify>,
}

impl EditQueue {
    pub fn handle_request(&self, responder: wry::RequestAsyncResponder) {
        let mut queue = self.queue.borrow_mut();
        self.edit_added.notify_waiters();
        if let Some(bytes) = queue.pop_back() {
            responder.respond(wry::http::Response::new(bytes));
        } else {
            *self.responder.borrow_mut() = Some(responder);
        }
    }

    pub fn add_edits(&self, edits: Vec<u8>) {
        let mut responder = self.responder.borrow_mut();
        if let Some(responder) = responder.take() {
            responder.respond(wry::http::Response::new(edits));
        } else {
            self.queue.borrow_mut().push_front(edits);
        }
    }

    fn edits_queued(&self) -> bool {
        self.queue.borrow().len() > 0
    }

    /// Wait until all pending edits have been rendered in the webview
    pub async fn wait_for_edits_flushed(&self) {
        while self.edits_queued() {
            self.edit_added.notified().await;
        }
    }
}
