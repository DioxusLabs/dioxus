//! Example: Antipatterns
//! ---------------------
//!
//! This example shows what *not* to do and provides a reason why a given pattern is considered an "AntiPattern". Most
//! anti-patterns are considered wrong to due performance reasons or violate the "rules" of Dioxus. These rules are
//! borrowed from other successful UI frameworks, and Dioxus is more focused on providing a familiar, ergonomic interface
//! rather than building new harder-to-misuse patterns.
use std::collections::HashMap;

use dioxus::prelude::*;
fn main() {}

/// Antipattern: Iterators without keys
/// -----------------------------------
///
/// This is considered an anti-pattern for performance reasons. Dioxus must diff your current and old layout and must
/// take a slower path if it can't correlate old elements with new elements. Lists are particularly susceptible to the
/// "slow" path, so you're strongly encouraged to provide a unique ID stable between renders.
///
/// Dioxus will log an error in the console if it detects that your iterator does not properly generate keys
#[derive(PartialEq, Props)]
struct NoKeysProps {
    data: HashMap<u32, String>,
}
static AntipatternNoKeys: FC<NoKeysProps> = |cx| {
    // WRONG: Make sure to add keys!
    rsx!(in cx, ul {
        {cx.data.iter().map(|(k, v)| rsx!(li { "List item: {v}" }))}
    });
    // Like this:
    rsx!(in cx, ul {
        {cx.data.iter().map(|(k, v)| rsx!(li { key: "{k}", "List item: {v}" }))}
    })
};

/// Antipattern: Deeply nested fragments
/// ------------------------------------
///
/// This particular antipattern is not necessarily an antipattern in other frameworks but does has a performance impact
/// in Dioxus apps. Fragments don't mount a physical element to the dom immediately, so Dioxus must recurse into its
/// children to find a physical dom node. This process is called "normalization". Other frameworks perform an agressive
/// mutative normalization while Dioxus keeps your VNodes immutable. This means that deepely nested fragments make Dioxus
/// perform unnecessary work. Prefer one or two levels of fragments / nested components until presenting a true dom element.
///
/// Only Component and Fragment nodes are susceptible to this issue. Dioxus mitigates this with components by providing
/// an API for registering shared state without the ContextProvider pattern.
static Blah: FC<()> = |cx| {
    // Try to avoid
    rsx!(in cx,
        Fragment {
            Fragment {
                Fragment {
                    Fragment {
                        Fragment {
                            div { "Finally have a real node!" }
                        }
                    }
                }
            }
        }
    )
};
