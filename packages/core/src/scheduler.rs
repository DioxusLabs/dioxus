/// # Dioxus uses a scheduler to run queued work in the correct order.
/// 
/// ## Goals
/// We try to prevent three different situations:
/// 1. Running queued work after it could be dropped. Related issues (https://github.com/DioxusLabs/dioxus/pull/1993)
/// 
/// User code often assumes that this property is true. For example, if this code reruns the child component after signal is changed to None, it will panic
/// ```rust
/// fn ParentComponent() -> Element {
///     let signal: Signal<Option<i32>> = use_signal(None);
/// 
///     rsx! {
///         if signal.read().is_some() {
///             ChildComponent { signal }
///         }
///     }
/// }
/// 
/// #[component]
/// fn ChildComponent(signal: Signal<Option<i32>>) -> Element {
///     // It feels safe to assume that signal is some because the parent component checked that it was some
///     rsx! { "{signal.read().unwrap()}" }
/// }
/// ```
/// 
/// 2. Running effects before the dom is updated. Related issues (https://github.com/DioxusLabs/dioxus/issues/2307)
/// 
/// Effects can be used to run code that accesses the DOM directly. They should only run when the DOM is in an updated state. If they are run with an out of date version of the DOM, unexpected behavior can occur.
/// ```rust
/// fn EffectComponent() -> Element {
///     let id = use_signal(0);
///     use_effect(move || {
///         let id = id.read();
///         // This will panic if the id is not written to the DOM before the effect is run
///         eval(format!(r#"document.getElementById("{id}").innerHTML = "Hello World";"#));
///     });
/// 
///     rsx! {
///         div { id: "{id}" }
///     }
/// }
/// 
/// 3. Observing out of date state. Related issues (https://github.com/DioxusLabs/dioxus/issues/1935)
/// 
/// Where ever possible, updates should happen in an order that makes it impossible to observe an out of date state.
/// ```rust
/// fn OutOfDateComponent() -> Element {
///     let id = use_signal(0);
///     // When you read memo, it should **always** be two times the value of id
///     let memo = use_memo(move || id() * 2);
///     assert_eq!(memo(), id() * 2);
/// 
///     // This should be true even if you update the value of id in the middle of the component
///     id += 1;
///     assert_eq!(memo(), id() * 2);
/// 
///     rsx! {
///         div { id: "{id}" }
///     }
/// }
/// ```
/// 
/// ## Implementation
/// 
/// There are three different types of queued work that can be run by the virtualdom:
/// 1. Dirty Scopes:
///    Description: When a scope is marked dirty, a rerun of the scope will be scheduled. This will cause the scope to rerun and update the DOM if any changes are detected during the diffing phase.
///    Priority: These are the highest priority tasks. Dirty scopes will be rerun in order from the scope closest to the root to the scope furthest from the root. We follow this order to ensure that if a higher component reruns and drops a lower component, the lower component will not be run after it should be dropped.
/// 
/// 2. Tasks:
///    Description: Futures spawned in the dioxus runtime each have an unique task id. When the waker for that future is called, the task is rerun.
///    Priority: These are the second highest priority tasks. They are run after all other dirty scopes have been resolved because those dirty scopes may cause children (and the tasks those children own) to drop which should cancel the futures.
/// 
/// 3. Effects:
///    Description: Effects should always run after all changes to the DOM have been applied.
///    Priority: These are the lowest priority tasks in the scheduler. They are run after all other dirty scopes and futures have been resolved. Other tasks may cause components to rerun, which would update the DOM. These effects should only run after the DOM has been updated.


