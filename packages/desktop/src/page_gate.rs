use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::protocol::BASE_URI;

/// WebKit clears its recent-crash count this long after a page finished loading.
const CRASH_COUNT_RESET: Duration = Duration::from_secs(30);

/// Which of the app's own pages a webview may load. After a web content process terminates it
/// follows WebKit's own reload policy, which wry's navigation delegate switches off.
#[derive(Default)]
pub(crate) struct PageLoadGate(Mutex<GateState>);

#[derive(Default)]
struct GateState {
    page_loaded: bool,
    recovery_expected: bool,
    recent_crashes: u32,
    initialized_at: Option<Instant>,
}

impl PageLoadGate {
    /// Only the first load passes, as another one would replace the app. The index may load again
    /// after a recovery and on Android, which loads it into a recreated activity's webview.
    pub(crate) fn allow_app_page(&self, url: &str) -> bool {
        let mut state = self.state();
        let first_load = !std::mem::replace(&mut state.page_loaded, true);
        first_load
            || (url == BASE_URI
                && (std::mem::take(&mut state.recovery_expected) || cfg!(target_os = "android")))
    }

    /// The current page reported `initialize` at `now`.
    pub(crate) fn page_initialized(&self, now: Instant) {
        self.state().initialized_at = Some(now);
    }

    /// The web content process terminated at `now`. Returns whether the index should be loaded again.
    #[cfg_attr(
        not(any(test, target_os = "macos", target_os = "ios")),
        expect(
            dead_code,
            reason = "only WebKit reports a terminated web content process"
        )
    )]
    pub(crate) fn web_content_terminated(&self, now: Instant) -> bool {
        let mut state = self.state();
        if state
            .initialized_at
            .take()
            .is_some_and(|at| now.saturating_duration_since(at) >= CRASH_COUNT_RESET)
        {
            state.recent_crashes = 0;
        }
        state.recent_crashes += 1;
        if state.recent_crashes > 1 {
            state.recent_crashes = 0;
            return false;
        }
        state.recovery_expected = true;
        true
    }

    fn state(&self) -> MutexGuard<'_, GateState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::BASE_URI;

    const OTHER_APP_PAGE: &str = "dioxus://index.html/form-target";

    #[test]
    fn index_loads_once_without_a_recovery() {
        let gate = PageLoadGate::default();
        assert!(gate.allow_app_page(BASE_URI));
        assert!(!gate.allow_app_page(BASE_URI));
        assert!(!gate.allow_app_page(OTHER_APP_PAGE));
    }

    #[test]
    fn a_recovery_allows_one_index_load() {
        let gate = PageLoadGate::default();
        let t0 = Instant::now();
        assert!(gate.allow_app_page(BASE_URI));
        gate.page_initialized(t0);

        assert!(gate.web_content_terminated(t0 + Duration::from_secs(60)));
        assert!(!gate.allow_app_page(OTHER_APP_PAGE));
        assert!(gate.allow_app_page(BASE_URI));
        assert!(!gate.allow_app_page(BASE_URI));
    }

    #[test]
    fn a_crash_before_the_recovered_page_initializes_is_refused() {
        let gate = PageLoadGate::default();
        let t0 = Instant::now();
        gate.allow_app_page(BASE_URI);
        gate.page_initialized(t0);

        assert!(gate.web_content_terminated(t0 + Duration::from_secs(60)));
        assert!(gate.allow_app_page(BASE_URI));
        assert!(!gate.web_content_terminated(t0 + Duration::from_secs(61)));
        assert!(!gate.allow_app_page(BASE_URI));
    }

    #[test]
    fn a_refusal_clears_the_count() {
        let gate = PageLoadGate::default();
        let t0 = Instant::now();
        assert!(gate.web_content_terminated(t0));
        assert!(!gate.web_content_terminated(t0));
        assert!(gate.web_content_terminated(t0));
    }

    #[test]
    fn the_count_clears_thirty_seconds_after_initialize() {
        let gate = PageLoadGate::default();
        let t0 = Instant::now();
        assert!(gate.web_content_terminated(t0));
        gate.page_initialized(t0);
        assert!(gate.web_content_terminated(t0 + Duration::from_secs(30)));
    }

    #[test]
    fn a_crash_within_thirty_seconds_of_initialize_is_refused() {
        let gate = PageLoadGate::default();
        let t0 = Instant::now();
        assert!(gate.web_content_terminated(t0));
        gate.page_initialized(t0);
        assert!(!gate.web_content_terminated(t0 + Duration::from_secs(29)));
    }
}
