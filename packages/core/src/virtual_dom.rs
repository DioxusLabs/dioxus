//! # VirtualDOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.
//!
//! In this file, multiple items are defined. This file is big, but should be documented well to
//! navigate the innerworkings of the Dom. We try to keep these main mechanics in this file to limit
//! the possible exposed API surface (keep fields private). This particular implementation of VDOM
//! is extremely efficient, but relies on some unsafety under the hood to do things like manage
//! micro-heaps for components. We are currently working on refactoring the safety out into safe(r)
//! abstractions, but current tests (MIRI and otherwise) show no issues with the current implementation.
//!
//! Included is:
//! - The [`VirtualDom`] itself
//! - The [`Scope`] object for mangning component lifecycle
//! - The [`ActiveFrame`] object for managing the Scope`s microheap
//! - The [`Context`] object for exposing VirtualDOM API to components
//! - The [`NodeFactory`] object for lazyily exposing the `Context` API to the nodebuilder API
//!
//! This module includes just the barebones for a complete VirtualDOM API.
//! Additional functionality is defined in the respective files.

use crate::innerlude::*;
use std::any::Any;

/// An integrated virtual node system that progresses events and diffs UI trees.
///
/// Differences are converted into patches which a renderer can use to draw the UI.
///
/// If you are building an App with Dioxus, you probably won't want to reach for this directly, instead opting to defer
/// to a particular crate's wrapper over the [`VirtualDom`] API.
///
/// Example
/// ```rust
/// static App: FC<()> = |cx| {
///     cx.render(rsx!{
///         div {
///             "Hello World"
///         }
///     })
/// }
///
/// async fn main() {
///     let mut dom = VirtualDom::new(App);
///     let mut inital_edits = dom.rebuild();
///     initialize_screen(inital_edits);
///
///     loop {
///         let next_frame = TimeoutFuture::new(Duration::from_millis(16));
///         let edits = dom.run_with_deadline(next_frame).await;
///         apply_edits(edits);
///         render_frame();
///     }
/// }
/// ```
pub struct VirtualDom {
    scheduler: Scheduler,

    base_scope: ScopeId,

    root_fc: Box<dyn Any>,

    root_props: Box<dyn Any>,

    // we need to keep the allocation around, but we don't necessarily use it
    _root_caller: Box<dyn for<'b> Fn(&'b Scope) -> DomTree<'b> + 'static>,
}

impl VirtualDom {
    /// Create a new VirtualDOM with a component that does not have special props.
    ///
    /// # Description
    ///
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    ///
    ///
    /// # Example
    /// ```
    /// fn Example(cx: Context<()>) -> DomTree  {
    ///     cx.render(rsx!( div { "hello world" } ))
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDOM is not progressed, you must either "run_with_deadline" or use "rebuild" to progress it.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }

    /// Create a new VirtualDOM with the given properties for the root component.
    ///
    /// # Description
    ///
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    ///
    ///
    /// # Example
    /// ```
    /// #[derive(PartialEq, Props)]
    /// struct SomeProps {
    ///     name: &'static str
    /// }
    ///
    /// fn Example(cx: Context<SomeProps>) -> DomTree  {
    ///     cx.render(rsx!{ div{ "hello {cx.name}" } })
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDOM is not progressed on creation. You must either "run_with_deadline" or use "rebuild" to progress it.
    ///
    /// ```rust
    /// let mut dom = VirtualDom::new_with_props(Example, SomeProps { name: "jane" });
    /// let mutations = dom.rebuild();
    /// ```
    pub fn new_with_props<P: Properties + 'static>(root: FC<P>, root_props: P) -> Self {
        let root_fc = Box::new(root);

        let root_props: Box<dyn Any> = Box::new(root_props);

        let props_ptr = root_props.downcast_ref::<P>().unwrap() as *const P;

        // Safety: this callback is only valid for the lifetime of the root props
        let root_caller: Box<dyn Fn(&Scope) -> DomTree> = Box::new(move |scope: &Scope| unsafe {
            let props: &'_ P = &*(props_ptr as *const P);
            std::mem::transmute(root(Context { props, scope }))
        });

        let scheduler = Scheduler::new();

        let base_scope = scheduler.pool.insert_scope_with_key(|myidx| {
            Scope::new(
                root_caller.as_ref(),
                myidx,
                None,
                0,
                ScopeChildren(&[]),
                scheduler.pool.channel.clone(),
            )
        });

        Self {
            _root_caller: root_caller,
            root_fc,
            base_scope,
            scheduler,
            root_props,
        }
    }

    /// Get the [`Scope`] for the root component.
    ///
    /// This is useful for traversing the tree from the root for heuristics or altnerative renderers that use Dioxus
    /// directly.
    pub fn base_scope(&self) -> &Scope {
        self.scheduler.pool.get_scope(self.base_scope).unwrap()
    }

    /// Get the [`Scope`] for a component given its [`ScopeId`]
    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scheduler.pool.get_scope(id)
    }

    /// Update the root props of this VirtualDOM.
    ///
    /// This method retuns None if the old props could not be removed. The entire VirtualDOM will be rebuilt immediately,
    /// so calling this method will block the main thread until computation is done.
    ///
    /// ## Example
    ///
    /// ```rust
    /// #[derive(Props, PartialEq)]
    /// struct AppProps {
    ///     route: &'static str
    /// }
    /// static App: FC<AppProps> = |cx| cx.render(rsx!{ "route is {cx.route}" });
    ///
    /// let mut dom = VirtualDom::new_with_props(App, AppProps { route: "start" });
    ///
    /// let mutations = dom.update_root_props(AppProps { route: "end" }).unwrap();
    /// ```
    pub fn update_root_props<'s, P: 'static>(&'s mut self, root_props: P) -> Option<Mutations<'s>> {
        let root_scope = self.scheduler.pool.get_scope_mut(self.base_scope).unwrap();

        // Pre-emptively drop any downstream references of the old props
        root_scope.ensure_drop_safety(&self.scheduler.pool);

        let mut root_props: Box<dyn Any> = Box::new(root_props);

        if let Some(props_ptr) = root_props.downcast_ref::<P>().map(|p| p as *const P) {
            // Swap the old props and new props
            std::mem::swap(&mut self.root_props, &mut root_props);

            let root = *self.root_fc.downcast_ref::<FC<P>>().unwrap();

            let root_caller: Box<dyn Fn(&Scope) -> DomTree> =
                Box::new(move |scope: &Scope| unsafe {
                    let props: &'_ P = &*(props_ptr as *const P);
                    std::mem::transmute(root(Context { props, scope }))
                });

            root_scope.update_scope_dependencies(&root_caller, ScopeChildren(&[]));

            drop(root_props);

            Some(self.rebuild())
        } else {
            None
        }
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom from scratch
    ///
    /// The diff machine expects the RealDom's stack to be the root of the application.
    ///
    /// Tasks will not be polled with this method, nor will any events be processed from the event queue. Instead, the
    /// root component will be ran once and then diffed. All updates will flow out as mutations.
    ///
    /// All state stored in components will be completely wiped away.
    ///
    /// # Example
    /// ```
    /// static App: FC<()> = |cx| cx.render(rsx!{ "hello world" });
    /// let mut dom = VirtualDom::new();
    /// let edits = dom.rebuild();
    ///
    /// apply_edits(edits);
    /// ```
    pub fn rebuild<'s>(&'s mut self) -> Mutations<'s> {
        self.scheduler.rebuild(self.base_scope)
    }

    /// Compute a manual diff of the VirtualDOM between states.
    ///
    /// This can be useful when state inside the DOM is remotely changed from the outside, but not propogated as an event.
    ///
    /// In this case, every component will be diffed, even if their props are memoized. This method is intended to be used
    /// to force an update of the DOM when the state of the app is changed outside of the app.
    ///
    ///
    /// # Example
    /// ```rust
    /// #[derive(PartialEq, Props)]
    /// struct AppProps {
    ///     value: Shared<&'static str>,
    /// }
    ///
    /// static App: FC<AppProps> = |cx| {
    ///     let val = cx.value.borrow();
    ///     cx.render(rsx! { div { "{val}" } })
    /// };
    ///
    /// let value = Rc::new(RefCell::new("Hello"));
    /// let mut dom = VirtualDom::new_with_props(
    ///     App,
    ///     AppProps {
    ///         value: value.clone(),
    ///     },
    /// );
    ///
    /// let _ = dom.rebuild();
    ///
    /// *value.borrow_mut() = "goodbye";
    ///
    /// let edits = dom.diff();
    /// ```
    pub fn diff<'s>(&'s mut self) -> Mutations<'s> {
        self.scheduler.hard_diff(self.base_scope)
    }

    /// Runs the virtualdom immediately, not waiting for any suspended nodes to complete.
    ///
    /// This method will not wait for any suspended nodes to complete. If there is no pending work, then this method will
    /// return "None"
    pub fn run_immediate<'s>(&'s mut self) -> Option<Vec<Mutations<'s>>> {
        if self.scheduler.has_any_work() {
            Some(self.scheduler.work_sync())
        } else {
            None
        }
    }

    /// Run the virtualdom with a deadline.
    ///
    /// This method will progress async tasks until the deadline is reached. If tasks are completed before the deadline,
    /// and no tasks are pending, this method will return immediately. If tasks are still pending, then this method will
    /// exhaust the deadline working on them.
    ///
    /// This method is useful when needing to schedule the virtualdom around other tasks on the main thread to prevent
    /// "jank". It will try to finish whatever work it has by the deadline to free up time for other work.
    ///
    /// Due to platform differences in how time is handled, this method accepts a future that resolves when the deadline
    /// is exceeded. However, the deadline won't be met precisely, so you might want to build some wiggle room into the
    /// deadline closure manually.
    ///
    /// The deadline is polled before starting to diff components. This strikes a balance between the overhead of checking
    /// the deadline and just completing the work. However, if an individual component takes more than 16ms to render, then
    /// the screen will "jank" up. In debug, this will trigger an alert.
    ///
    /// If there are no in-flight fibers when this method is called, it will await any possible tasks, aborting early if
    /// the provided deadline future resolves.
    ///
    /// For use in the web, it is expected that this method will be called to be executed during "idle times" and the
    /// mutations to be applied during the "paint times" IE "animation frames". With this strategy, it is possible to craft
    /// entirely jank-free applications that perform a ton of work.
    ///
    /// # Example
    ///
    /// ```no_run
    /// static App: FC<()> = |cx| rsx!(in cx, div {"hello"} );
    /// let mut dom = VirtualDom::new(App);
    /// loop {
    ///     let deadline = TimeoutFuture::from_ms(16);
    ///     let mutations = dom.run_with_deadline(deadline).await;
    ///     apply_mutations(mutations);
    /// }
    /// ```
    ///
    /// ## Mutations
    ///
    /// This method returns "mutations" - IE the necessary changes to get the RealDOM to match the VirtualDOM. It also
    /// includes a list of NodeRefs that need to be applied and effects that need to be triggered after the RealDOM has
    /// applied the edits.
    ///
    /// Mutations are the only link between the RealDOM and the VirtualDOM.
    pub async fn run_with_deadline<'s>(
        &'s mut self,
        deadline: impl std::future::Future<Output = ()>,
    ) -> Vec<Mutations<'s>> {
        self.scheduler.work_with_deadline(deadline).await
    }

    pub fn get_event_sender(&self) -> futures_channel::mpsc::UnboundedSender<SchedulerMsg> {
        self.scheduler.pool.channel.sender.clone()
    }

    /// Waits for the scheduler to have work
    /// This lets us poll async tasks during idle periods without blocking the main thread.
    pub async fn wait_for_work(&mut self) {
        if self.scheduler.has_any_work() {
            return;
        }

        use futures_util::StreamExt;
        futures_util::select! {
            // hmm - will this resolve to none if there are no async tasks?
            _ = self.scheduler.async_tasks.next() => {}
            msg = self.scheduler.receiver.next() => self.scheduler.handle_channel_msg(msg.unwrap()),
        }
    }
}
