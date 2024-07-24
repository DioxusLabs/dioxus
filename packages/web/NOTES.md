
// ## RequestAnimationFrame and RequestIdleCallback
// ------------------------------------------------
// React implements "jank free rendering" by deliberately not blocking the browser's main thread. For large diffs, long
// running work, and integration with things like React-Three-Fiber, it's extremely important to avoid blocking the
// main thread.
//
// React solves this problem by breaking up the rendering process into a "diff" phase and a "render" phase. In Dioxus,
// the diff phase is non-blocking, using "work_with_deadline" to allow the browser to process other events. When the diff phase
// is  finally complete, the VirtualDOM will return a set of "Mutations" for this crate to apply.
//
// Here, we schedule the "diff" phase during the browser's idle period, achieved by calling RequestIdleCallback and then
// setting a timeout from the that completes when the idleperiod is over. Then, we call requestAnimationFrame
//
//     From Google's guide on rAF and rIC:
//     -----------------------------------
//
//     If the callback is fired at the end of the frame, it will be scheduled to go after the current frame has been committed,
//     which means that style changes will have been applied, and, importantly, layout calculated. If we make DOM changes inside
//      of the idle callback, those layout calculations will be invalidated. If there are any kind of layout reads in the next
//      frame, e.g. getBoundingClientRect, clientWidth, etc, the browser will have to perform a Forced Synchronous Layout,
//      which is a potential performance bottleneck.
//
//     Another reason not trigger DOM changes in the idle callback is that the time impact of changing the DOM is unpredictable,
//     and as such we could easily go past the deadline the browser provided.
//
//     The best practice is to only make DOM changes inside of a requestAnimationFrame callback, since it is scheduled by the
//     browser with that type of work in mind. That means that our code will need to use a document fragment, which can then
//     be appended in the next requestAnimationFrame callback. If you are using a VDOM library, you would use requestIdleCallback
//     to make changes, but you would apply the DOM patches in the next requestAnimationFrame callback, not the idle callback.
//
//     Essentially:
//     ------------
//     - Do the VDOM work during the idlecallback
//     - Do DOM work in the next requestAnimationFrame callback
