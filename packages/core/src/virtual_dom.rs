//! # VirtualDOM Implementation for Rust
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
//! - The [`Hook`] object for exposing state management in components.
//!
//! This module includes just the barebones for a complete VirtualDOM API.
//! Additional functionality is defined in the respective files.
use crate::innerlude::*;
use futures_util::{pin_mut, Future, FutureExt};
use std::{
    any::{Any, TypeId},
    pin::Pin,
};

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
///
///
///
///
///
///
///
pub struct VirtualDom {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    ///
    /// This is wrapped in an UnsafeCell because we will need to get mutable access to unique values in unique bump arenas
    /// and rusts's guartnees cannot prove that this is safe. We will need to maintain the safety guarantees manually.
    pub shared: SharedResources,

    /// The index of the root component
    /// Should always be the first (gen=0, id=0)
    base_scope: ScopeId,

    scheduler: Scheduler,

    // for managing the props that were used to create the dom
    #[doc(hidden)]
    _root_prop_type: std::any::TypeId,

    #[doc(hidden)]
    _root_props: std::pin::Pin<Box<dyn std::any::Any>>,
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
    /// fn Example(cx: Context<SomeProps>) -> VNode  {
    ///     cx.render(rsx!{ div{"hello world"} })
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
    /// fn Example(cx: Context<SomeProps>) -> VNode  {
    ///     cx.render(rsx!{ div{"hello world"} })
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDOM is not progressed, you must either "run_with_deadline" or use "rebuild" to progress it.
    pub fn new_with_props<P: Properties + 'static>(root: FC<P>, root_props: P) -> Self {
        let components = SharedResources::new();

        let root_props: Pin<Box<dyn Any>> = Box::pin(root_props);
        let props_ptr = root_props.as_ref().downcast_ref::<P>().unwrap() as *const P;

        let link = components.clone();

        let base_scope = components.insert_scope_with_key(move |myidx| {
            let caller = NodeFactory::create_component_caller(root, props_ptr as *const _);
            let name = type_name_of(root);
            Scope::new(caller, myidx, None, 0, ScopeChildren(&[]), link, name)
        });

        Self {
            base_scope,
            _root_props: root_props,
            scheduler: Scheduler::new(components.clone()),
            shared: components,
            _root_prop_type: TypeId::of::<P>(),
        }
    }

    pub fn base_scope(&self) -> &Scope {
        self.shared.get_scope(self.base_scope).unwrap()
    }

    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.shared.get_scope(id)
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom rom scratch
    ///
    /// The diff machine expects the RealDom's stack to be the root of the application
    ///
    /// Events like garabge collection, application of refs, etc are not handled by this method and can only be progressed
    /// through "run". We completely avoid the task scheduler infrastructure.
    pub fn rebuild<'s>(&'s mut self) -> Mutations<'s> {
        let mut fut = self.rebuild_async().boxed_local();

        loop {
            if let Some(edits) = (&mut fut).now_or_never() {
                break edits;
            }
        }
    }

    /// Rebuild the dom from the ground up
    ///
    /// This method is asynchronous to prevent the application from blocking while the dom is being rebuilt. Computing
    /// the diff and creating nodes can be expensive, so we provide this method to avoid blocking the main thread. This
    /// method can be useful when needing to perform some crucial periodic tasks.
    pub async fn rebuild_async<'s>(&'s mut self) -> Mutations<'s> {
        let mut diff_machine = DiffMachine::new(Mutations::new(), self.base_scope, &self.shared);

        let cur_component = self
            .shared
            .get_scope_mut(self.base_scope)
            .expect("The base scope should never be moved");

        // // We run the component. If it succeeds, then we can diff it and add the changes to the dom.
        if cur_component.run_scope().is_ok() {
            diff_machine
                .stack
                .create_node(cur_component.frames.fin_head(), MountType::Append);
            diff_machine.work().await;
        } else {
            // todo: should this be a hard error?
            log::warn!(
                "Component failed to run succesfully during rebuild.
                This does not result in a failed rebuild, but indicates a logic failure within your app."
            );
        }

        diff_machine.mutations
    }

    pub fn diff_sync<'s>(&'s mut self) -> Mutations<'s> {
        let mut fut = self.diff_async().boxed_local();

        loop {
            if let Some(edits) = (&mut fut).now_or_never() {
                break edits;
            }
        }
    }

    pub async fn diff_async<'s>(&'s mut self) -> Mutations<'s> {
        let mut diff_machine = DiffMachine::new(Mutations::new(), self.base_scope, &self.shared);

        let cur_component = self
            .shared
            .get_scope_mut(self.base_scope)
            .expect("The base scope should never be moved");

        cur_component.run_scope().unwrap();

        diff_machine.stack.push(DiffInstruction::DiffNode {
            old: cur_component.frames.wip_head(),
            new: cur_component.frames.fin_head(),
        });

        diff_machine.work().await;

        diff_machine.mutations
    }

    /// Runs the virtualdom immediately, not waiting for any suspended nodes to complete.
    ///
    /// This method will not wait for any suspended nodes to complete.
    pub fn run_immediate<'s>(&'s mut self) -> Mutations<'s> {
        todo!()
        // use futures_util::FutureExt;
        // let mut is_ready = || false;
        // self.run_with_deadline(futures_util::future::ready(()), &mut is_ready)
        //     .now_or_never()
        //     .expect("this future will always resolve immediately")
    }

    /// Runs the virtualdom with no time limit.
    ///
    /// If there are pending tasks, they will be progressed before returning. This is useful when rendering an application
    /// that has suspended nodes or suspended tasks. Be warned - any async tasks running forever will prevent this method
    /// from completing. Consider using `run` and specifing a deadline.
    pub async fn run_unbounded<'s>(&'s mut self) -> Mutations<'s> {
        self.run_with_deadline(async {}).await.unwrap()
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
        deadline: impl Future<Output = ()>,
    ) -> Option<Mutations<'s>> {
        let mut committed_mutations = Mutations::new();
        let mut deadline = Box::pin(deadline.fuse());

        // TODO:
        // the scheduler uses a bunch of different receivers to mimic a "topic" queue system. The futures-channel implementation
        // doesn't really have a concept of a "topic" queue, so there's a lot of noise in the hand-rolled scheduler. We should
        // explore abstracting the scheduler into a topic-queue channel system - similar to Kafka or something similar.
        loop {
            // Internalize any pending work since the last time we ran
            self.scheduler.manually_poll_events();

            // Wait for any new events if we have nothing to do
            if !self.scheduler.has_any_work() {
                self.scheduler.clean_up_garbage();
                let deadline_expired = self.scheduler.wait_for_any_trigger(&mut deadline).await;

                if deadline_expired {
                    return Some(committed_mutations);
                }
            }

            // Create work from the pending event queue
            self.scheduler.consume_pending_events();

            // Work through the current subtree, and commit the results when it finishes
            // When the deadline expires, give back the work
            match self.scheduler.work_with_deadline(&mut deadline) {
                FiberResult::Done(mut mutations) => {
                    committed_mutations.extend(&mut mutations);

                    /*
                    quick return if there's no work left, so we can commit before the deadline expires
                    When we loop over again, we'll re-wait for any new work.

                    I'm not quite sure how this *should* work.

                    It makes sense to try and progress the DOM faster
                    */

                    if !self.scheduler.has_any_work() {
                        return Some(committed_mutations);
                    }
                }
                FiberResult::Interrupted => return None,
            }
        }
    }

    pub fn get_event_sender(&self) -> futures_channel::mpsc::UnboundedSender<EventTrigger> {
        self.shared.ui_event_sender.clone()
    }

    pub fn has_work(&self) -> bool {
        true
    }

    pub async fn wait_for_any_work(&self) {}
}

// TODO!
// These impls are actually wrong. The DOM needs to have a mutex implemented.
unsafe impl Sync for VirtualDom {}
unsafe impl Send for VirtualDom {}
