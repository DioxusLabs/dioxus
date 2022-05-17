use log::error;
use url::Url;

use super::HistoryProvider;

/// A [`HistoryProvider`] that stores all information in memory.
pub struct MemoryHistoryProvider {
    current: Url,
    past: Vec<String>,
    future: Vec<String>,
}

impl Default for MemoryHistoryProvider {
    fn default() -> Self {
        Self {
            current: Url::parse("dioxus://index.html/").unwrap(),
            past: Default::default(),
            future: Default::default(),
        }
    }
}

impl HistoryProvider for MemoryHistoryProvider {
    fn current_path(&self) -> String {
        self.current.path().to_string()
    }

    fn current_query(&self) -> Option<String> {
        self.current.query().map(|q| q.to_string())
    }

    fn can_go_back(&self) -> bool {
        !self.past.is_empty()
    }

    fn can_go_forward(&self) -> bool {
        !self.future.is_empty()
    }

    fn go_back(&mut self) {
        if self.can_go_back() {
            self.future.push(self.current.to_string());
            self.current = Url::parse(&self.past.pop().unwrap()).unwrap();

            // past urls are always valid, they came from the url struct itself
        }
    }

    fn go_forward(&mut self) {
        if self.can_go_forward() {
            self.past.push(self.current.to_string());
            self.current = Url::parse(&self.future.pop().unwrap()).unwrap();

            // future urls are always valid, they came from the url struct itself
        }
    }

    fn push(&mut self, path: String) {
        if path.starts_with("//") {
            error!(r#"cannot navigate to paths starting with "//", path: {path}"#);
            return;
        }

        let previous_path = self.current.to_string();

        if let Ok(url) = self.current.join(&path) {
            self.past.push(previous_path);
            self.current = url;
            self.future.clear();
        }
    }

    fn replace(&mut self, path: String) {
        if path.starts_with("//") {
            error!(r#"cannot navigate to paths starting with "//", path: {path}"#);
            return;
        }

        if let Ok(url) = self.current.join(&path) {
            self.current = url;
        }
    }
}
