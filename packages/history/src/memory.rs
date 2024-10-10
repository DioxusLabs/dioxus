use std::cell::RefCell;

use crate::History;

struct MemoryHistoryState {
    current: String,
    history: Vec<String>,
    future: Vec<String>,
}

/// A [`History`] provider that stores all navigation information in memory.
pub struct MemoryHistory {
    state: RefCell<MemoryHistoryState>,
}

impl Default for MemoryHistory {
    fn default() -> Self {
        Self {
            state: MemoryHistoryState{
            current: "/".parse().unwrap_or_else(|err| {
                panic!("index route does not exist:\n{err}\n use MemoryHistory::with_initial_path to set a custom path")
            }),
            history: Vec::new(),
            future: Vec::new(),}.into()
        }
    }
}

impl History for MemoryHistory {
    fn current_route(&self) -> String {
        self.state.borrow().current.clone()
    }

    fn can_go_back(&self) -> bool {
        !self.state.borrow().history.is_empty()
    }

    fn go_back(&self) {
        let mut write = self.state.borrow_mut();
        if let Some(last) = write.history.pop() {
            let old = std::mem::replace(&mut write.current, last);
            write.future.push(old);
        }
    }

    fn can_go_forward(&self) -> bool {
        !self.state.borrow().future.is_empty()
    }

    fn go_forward(&self) {
        let mut write = self.state.borrow_mut();
        if let Some(next) = write.future.pop() {
            let old = std::mem::replace(&mut write.current, next);
            write.history.push(old);
        }
    }

    fn push(&self, new: String) {
        let mut write = self.state.borrow_mut();
        // don't push the same route twice
        if write.current == new {
            return;
        }
        let old = std::mem::replace(&mut write.current, new);
        write.history.push(old);
        write.future.clear();
    }

    fn replace(&self, path: String) {
        let mut write = self.state.borrow_mut();
        write.current = path;
    }
}
