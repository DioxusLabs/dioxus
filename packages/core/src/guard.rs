use std::{fmt, rc::Rc};

use crate::Runtime;

/// A guard for a new runtime. This must be used to override the current runtime when importing components from a dynamic library that has it's own runtime.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_core::{Runtime, RuntimeGuard};
///
/// fn main() {
///     let virtual_dom = VirtualDom::new(app);
/// }
///
/// fn app() -> Element {
///     rsx! { Component { runtime: Runtime::current().unwrap() } }
/// }
///
/// // In a dynamic library
/// #[derive(Props, Clone)]
/// struct ComponentProps {
///    runtime: std::rc::Rc<Runtime>,
/// }
///
/// impl PartialEq for ComponentProps {
///     fn eq(&self, _other: &Self) -> bool {
///         true
///     }
/// }
///
/// fn Component(props: ComponentProps) -> Element {
///     use_hook(|| {
///         let _guard = RuntimeGuard::new(props.runtime.clone());
///     });
///
///     rsx! { div {} }
/// }
/// ```
pub struct RuntimeGuard(());

impl RuntimeGuard {
    /// Create a new runtime guard that sets the current Dioxus runtime. The runtime will be reset when the guard is dropped
    pub fn new(runtime: Rc<Runtime>) -> Self {
        Runtime::push(runtime);
        Self(())
    }
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        Runtime::pop();
    }
}

/// Missing Dioxus runtime error.
pub struct RuntimeError {
    _priv: (),
}

impl RuntimeError {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self { _priv: () }
    }
}

impl fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeError").finish()
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Must be called from inside a Dioxus runtime.

Help: Some APIs in dioxus require a global runtime to be present.
If you are calling one of these APIs from outside of a dioxus runtime
(typically in a web-sys closure or dynamic library), you will need to
grab the runtime from a scope that has it and then move it into your
new scope with a runtime guard.

For example, if you are trying to use dioxus apis from a web-sys
closure, you can grab the runtime from the scope it is created in:

```rust
use dioxus::prelude::*;
static COUNT: GlobalSignal<i32> = Signal::global(|| 0);

#[component]
fn MyComponent() -> Element {{
    use_effect(|| {{
        // Grab the runtime from the MyComponent scope
        let runtime = Runtime::current().expect(\"Components run in the Dioxus runtime\");
        // Move the runtime into the web-sys closure scope
        let web_sys_closure = Closure::new(|| {{
            // Then create a guard to provide the runtime to the closure
            let _guard = RuntimeGuard::new(runtime);
            // and run whatever code needs the runtime
            tracing::info!(\"The count is: {{COUNT}}\");
        }});
    }})
}}
```"
        )
    }
}

impl std::error::Error for RuntimeError {}
