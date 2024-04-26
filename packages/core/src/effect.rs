/// Effects will always run after all changes to the DOM have been applied.
/// 
/// Effects are the lowest priority task in the scheduler.
/// They are run after all other dirty scopes and futures have been resolved. Other dirty scopes and futures may cause the component this effect is attached to to rerun, which would update the DOM.
struct Effect {
    // The callback that will be run when the effect is rerun
    callback: Box<dyn FnMut() + 'static>,
}
