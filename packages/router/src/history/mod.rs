//! History Integration
//!
//! dioxus-router-core relies on [`HistoryProvider`]s to store the current Route, and possibly a
//! history (i.e. a browsers back button) and future (i.e. a browsers forward button).
//!
//! To integrate dioxus-router with a any type of history, all you have to do is implement the
//! [`HistoryProvider`] trait.
//!
//! dioxus-router contains two built in history providers:
//! 1) [`MemoryHistory`] for desktop/mobile/ssr platforms
//! 2) [`WebHistory`] for web platforms

mod memory;
pub use memory::*;
