use super::HistoryProvider;

/// A [`HistoryProvider`] that stores all information in memory.
pub struct MemoryHistoryProvider {
    current: Vec<String>,
    future: Vec<String>,
}

impl Default for MemoryHistoryProvider {
    fn default() -> Self {
        Self {
            current: vec![String::from("/")],
            future: Default::default(),
        }
    }
}

impl HistoryProvider for MemoryHistoryProvider {
    fn current_path<'a>(&'a self) -> &'a str {
        &self.current.last().unwrap() // memory history always has at least one item
    }

    fn can_go_back(&self) -> bool {
        self.current.len() > 1
    }

    fn can_go_forward(&self) -> bool {
        !self.future.is_empty()
    }

    fn can_handle_external(&self) -> bool {
        false
    }

    fn go_back(&mut self) {
        if self.can_go_back() {
            self.future.push(self.current.pop().unwrap());
        }
    }

    fn go_forward(&mut self) {
        if self.can_go_forward() {
            self.current.push(self.future.pop().unwrap());
        }
    }

    fn push(&mut self, path: String) {
        self.current.push(path);
        self.future.clear();
    }

    fn replace(&mut self, path: String) {
        *self.current.last_mut().unwrap() = path; // memory history always has at least one item
    }
}
