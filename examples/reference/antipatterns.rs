//! Example: Antipatterns
//! ---------------------
//!
//! This example shows what *not* to do and provides a reason why a given pattern is considered an "AntiPattern". Most
//! anti-patterns are considered wrong to due performance reasons or violate the "rules" of Dioxus. These rules are
//! borrowed from other successful UI frameworks, and Dioxus is more focused on providing a familiar, ergonomic interface
//! rather than building new harder-to-misuse patterns.
//!
//! In this list we showcase:
//! - Not adding keys for iterators
//! - Heavily nested fragments
//! - Understadning ordering of set_state
//! - Naming conventions
//! - Rules of hooks
//!
//! Feel free to file a PR or Issue if you run into another antipattern that you think users of Dioxus should know about.
use dioxus::prelude::*;

/// Antipattern: Iterators without keys
/// -----------------------------------
///
/// This is considered an anti-pattern for performance reasons. Dioxus will diff your current and old layout and must
/// take a slower path if it can't correlate old elements with new elements. Lists are particularly susceptible to the
/// "slow" path, so you're strongly encouraged to provide a unique ID stable between renders. Additionally, providing
/// the *wrong* keys is even worse - props might be assigned to the wrong components! Keys should be:
/// - Unique
/// - Stable
/// - Predictable
///
/// Dioxus will log an error in the console if it detects that your iterator does not properly generate keys
#[derive(PartialEq, Props)]
struct NoKeysProps {
    data: std::collections::HashMap<u32, String>,
}
static AntipatternNoKeys: FC<NoKeysProps> = |cx| {
    // WRONG: Make sure to add keys!
    rsx!(in cx, ul {
        {cx.data.iter().map(|(k, v)| rsx!(li { "List item: {v}" }))}
    });
    // RIGHT: Like this:
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
static AntipatternNestedFragments: FC<()> = |cx| {
    // Try to avoid heavily nesting fragments
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

/// Antipattern: Using state after its been updated
/// -----------------------------------------------
///
/// This is an antipattern in other frameworks, but less so in Dioxus. However, it's important to highlight that use_state
/// does *not* work the same way as it does in React. Rust provides explicit guards against mutating shared data - a huge
/// problem in JavaScript land. With Rust and Dioxus, it's nearly impossible to misuse `use_state` - you simply can't
/// accidentally modify the state you've received!
///
/// However, calling set_state will *not* update the current version of state in the component. This should be easy to
/// recognize from the function signature, but Dioxus will not update the "live" version of state. Calling `set_state`
/// merely places a new value in the queue and schedules the component for a future update.
static AntipaternRelyingOnSetState: FC<()> = |cx| {
    let (state, set_state) = use_state(cx, || "Hello world").classic();
    set_state("New state");
    // This will return false! `state` will *still* be "Hello world"
    assert!(state == &"New state");
    todo!()
};

/// Antipattern: Capitalization
/// ---------------------------
///
/// This antipattern is enforced to retain parity with other frameworks and provide useful IDE feedback, but is less
/// critical than other potential misues. In short:
/// - Only raw elements may start with a lowercase character
/// - All components must start with an uppercase character
///
/// IE: the following component will be rejected when attempted to be used in the rsx! macro
static antipattern_component: FC<()> = |cx| todo!();

/// Antipattern: Misusing hooks
/// ---------------------------
///
/// This pattern is an unfortunate one where Dioxus supports the same behavior as in other frameworks. Dioxus supports
/// "hooks" - IE "memory cells" that allow a value to be stored between renders. This allows other hooks to tap into
/// a components "memory" without explicitly adding all of its data to a struct definition. In Dioxus, hooks are allocated
/// with a bump arena and then immediately sealed.
///
/// This means that hooks may not be misued:
/// - Called out of order
/// - Called in a conditional
/// - Called in loops or callbacks
///
/// For the most part, Rust helps with rule #3 but does not save you from misusing rule #1 or #2. Dioxus will panic
/// if hooks do not downcast the same data between renders. This is validated by TypeId - and eventually - a custom key.
#[derive(PartialEq, Props)]
struct MisuedHooksProps {
    should_render_state: bool,
}
static AntipatternMisusedHooks: FC<MisuedHooksProps> = |cx| {
    if cx.should_render_state {
        // do not place a hook in the conditional!
        // prefer to move it out of the conditional
        let (state, set_state) = use_state(cx, || "hello world").classic();
        rsx!(in cx, div { "{state}" })
    } else {
        rsx!(in cx, div { "Not rendering state" })
    }
};

/// Antipattern: Downcasting refs and panicing
/// ------------------------------------------
///
/// Occassionally it's useful to get the ref of an element to handle it directly. Elements support downcasting to
/// Dioxus's virtual element types as well as their true native counterparts. Downcasting to Dioxus' virtual elements
/// will never panic, but downcasting to native elements will fail if on an unsupported platform. We recommend avoiding
/// publishing hooks and components that deply rely on control over elements using their native `ref`, preferring to
/// use their Dioxus Virtual Element counterpart instead.
// This particular code *will panic* due to the unwrap. Try to avoid these types of patterns.
/// ---------------------------------
/// TODO: Get this to compile properly
/// let div_ref = use_node_ref(&cx);
///
/// cx.render(rsx!{
///     div { ref: div_ref, class: "custom class",
///         button { "click me to see my parent's class"
///             onclick: move |_| if let Some(div_ref) = div_ref {
///                 panic!("Div class is {}", div_ref.to_native::<web_sys::Element>().unwrap().class())
///             }
///         }
///     }
/// })
static _example: FC<()> = |cx| todo!();

/// Antipattern: publishing components and hooks with all features enabled
/// ----------------------------------------------------------------------
///
/// The `dioxus` crate combines a bunch of useful utilities together (like the rsx! and html! macros, hooks, and more).
/// However, when publishing your custom hook or component, we highly advise using only the `core` feature on the dioxus
/// crate. This makes your crate compile faster, makes it more stable, and avoids bringing in incompatible libraries that
/// might make it not compile on unsupported platforms.
///
/// We don't have a code snippet for this, but just prefer to use this line:
///     dioxus = { version = "*", features = ["core"]}
/// instead of this one:
///     dioxus = { version = "*", features = ["web", "desktop", "full"]}
/// in your Cargo.toml
///
/// This will only include the `core` dioxus crate which is relatively slim and fast to compile and avoids target-specific
/// libraries.
static __example: FC<()> = |cx| todo!();

pub static Example: FC<()> = |cx| {
    cx.render(rsx! {
        AntipatternNoKeys { data: std::collections::HashMap::new() }
        AntipatternNestedFragments {}
    })
};
