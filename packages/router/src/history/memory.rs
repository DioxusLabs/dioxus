use std::str::FromStr;

use crate::routable::Routable;

use super::HistoryProvider;

/// A [`HistoryProvider`] that stores all navigation information in memory.
pub struct MemoryHistory<R: Routable> {
    current: R,
    history: Vec<R>,
    future: Vec<R>,
}

impl<R: Routable> MemoryHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    /// Create a [`MemoryHistory`] starting at `path`.
    ///
    /// ```rust
    /// # use dioxus_router_core::history::{HistoryProvider, MemoryHistory};
    /// let mut history = MemoryHistory::with_initial_path("/some/path").unwrap();
    /// assert_eq!(history.current_path(), "/some/path");
    /// assert_eq!(history.can_go_back(), false);
    /// ```
    pub fn with_initial_path(path: impl Into<String>) -> Result<Self, <R as FromStr>::Err> {
        let path = path.into();

        Ok(Self {
            current: R::from_str(&path)?,
            ..Default::default()
        })
    }
}

impl<R: Routable> Default for MemoryHistory<R>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            current: "/".parse().unwrap_or_else(|err| panic!("{}", err)),
            history: Vec::new(),
            future: Vec::new(),
        }
    }
}

impl<R: Routable> HistoryProvider<R> for MemoryHistory<R> {
    fn current_route(&self) -> R {
        self.current.clone()
    }

    fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    fn go_back(&mut self) {
        if let Some(last) = self.history.pop() {
            let old = std::mem::replace(&mut self.current, last);
            self.future.push(old);
        }
    }

    fn can_go_forward(&self) -> bool {
        !self.future.is_empty()
    }

    fn go_forward(&mut self) {
        if let Some(next) = self.future.pop() {
            let old = std::mem::replace(&mut self.current, next);
            self.history.push(old);
        }
    }

    fn push(&mut self, new: R) {
        let old = std::mem::replace(&mut self.current, new);
        self.history.push(old);
        self.future.clear();
    }

    fn replace(&mut self, path: R) {
        self.current = path;
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn default() {
//         let mem = MemoryHistory::default();
//         assert_eq!(mem.current, Url::parse(INITIAL_URL).unwrap());
//         assert_eq!(mem.history, Vec::<String>::new());
//         assert_eq!(mem.future, Vec::<String>::new());
//     }

//     #[test]
//     fn with_initial_path() {
//         let mem = MemoryHistory::with_initial_path("something").unwrap();
//         assert_eq!(
//             mem.current,
//             Url::parse(&format!("{INITIAL_URL}something")).unwrap()
//         );
//         assert_eq!(mem.history, Vec::<String>::new());
//         assert_eq!(mem.future, Vec::<String>::new());
//     }

//     #[test]
//     fn with_initial_path_with_leading_slash() {
//         let mem = MemoryHistory::with_initial_path("/something").unwrap();
//         assert_eq!(
//             mem.current,
//             Url::parse(&format!("{INITIAL_URL}something")).unwrap()
//         );
//         assert_eq!(mem.history, Vec::<String>::new());
//         assert_eq!(mem.future, Vec::<String>::new());
//     }

//     #[test]
//     fn can_go_back() {
//         let mut mem = MemoryHistory::default();
//         assert!(!mem.can_go_back());

//         mem.push(String::from("/test"));
//         assert!(mem.can_go_back());
//     }

//     #[test]
//     fn go_back() {
//         let mut mem = MemoryHistory::default();
//         mem.push(String::from("/test"));
//         mem.go_back();

//         assert_eq!(mem.current, Url::parse(INITIAL_URL).unwrap());
//         assert!(mem.history.is_empty());
//         assert_eq!(mem.future, vec![format!("{INITIAL_URL}test")]);
//     }

//     #[test]
//     fn can_go_forward() {
//         let mut mem = MemoryHistory::default();
//         assert!(!mem.can_go_forward());

//         mem.push(String::from("/test"));
//         mem.go_back();

//         assert!(mem.can_go_forward());
//     }

//     #[test]
//     fn go_forward() {
//         let mut mem = MemoryHistory::default();
//         mem.push(String::from("/test"));
//         mem.go_back();
//         mem.go_forward();

//         assert_eq!(
//             mem.current,
//             Url::parse(&format!("{INITIAL_URL}test")).unwrap()
//         );
//         assert_eq!(mem.history, vec![INITIAL_URL.to_string()]);
//         assert!(mem.future.is_empty());
//     }

//     #[test]
//     fn push() {
//         let mut mem = MemoryHistory::default();
//         mem.push(String::from("/test"));

//         assert_eq!(
//             mem.current,
//             Url::parse(&format!("{INITIAL_URL}test")).unwrap()
//         );
//         assert_eq!(mem.history, vec![INITIAL_URL.to_string()]);
//         assert!(mem.future.is_empty());
//     }

//     #[test]
//     #[should_panic = r#"cannot navigate to paths starting with "//": //test"#]
//     #[cfg(debug_assertions)]
//     fn push_debug() {
//         let mut mem = MemoryHistory::default();
//         mem.push(String::from("//test"));
//     }

//     #[test]
//     #[cfg(not(debug_assertions))]
//     fn push_release() {
//         let mut mem = MemoryHistory::default();
//         mem.push(String::from("//test"));

//         assert_eq!(mem.current, Url::parse(INITIAL_URL).unwrap());
//         assert!(mem.history.is_empty())
//     }

//     #[test]
//     fn replace() {
//         let mut mem = MemoryHistory::default();
//         mem.push(String::from("/test"));
//         mem.push(String::from("/other"));
//         mem.go_back();
//         mem.replace(String::from("/replace"));

//         assert_eq!(
//             mem.current,
//             Url::parse(&format!("{INITIAL_URL}replace")).unwrap()
//         );
//         assert_eq!(mem.history, vec![INITIAL_URL.to_string()]);
//         assert_eq!(mem.future, vec![format!("{INITIAL_URL}other")]);
//     }

//     #[test]
//     #[should_panic = r#"cannot navigate to paths starting with "//": //test"#]
//     #[cfg(debug_assertions)]
//     fn replace_debug() {
//         let mut mem = MemoryHistory::default();
//         mem.replace(String::from("//test"));
//     }

//     #[test]
//     #[cfg(not(debug_assertions))]
//     fn replace_release() {
//         let mut mem = MemoryHistory::default();
//         mem.replace(String::from("//test"));

//         assert_eq!(mem.current, Url::parse(INITIAL_URL).unwrap());
//     }
// }
