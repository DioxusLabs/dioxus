//! # Dioxus uses a scheduler to run queued work in the correct order.
//!
//! ## Goals
//! We try to prevent three different situations:
//! 1. Running queued work after it could be dropped. Related issues (<https://github.com/DioxusLabs/dioxus/pull/1993>)
//!
//! User code often assumes that this property is true. For example, if this code reruns the child component after signal is changed to None, it will panic
//! ```rust, ignore
//! fn ParentComponent() -> Element {
//!     let signal: Signal<Option<i32>> = use_signal(None);
//!
//!     rsx! {
//!         if signal.read().is_some() {
//!             ChildComponent { signal }
//!         }
//!     }
//! }
//!
//! #[component]
//! fn ChildComponent(signal: WriteSignal<Option<i32>>) -> Element {
//!     // It feels safe to assume that signal is some because the parent component checked that it was some
//!     rsx! { "{signal.read().unwrap()}" }
//! }
//! ```
//!
//! 2. Running effects before the dom is updated. Related issues (<https://github.com/DioxusLabs/dioxus/issues/2307>)
//!
//! Effects can be used to run code that accesses the DOM directly. They should only run when the DOM is in an updated state. If they are run with an out of date version of the DOM, unexpected behavior can occur.
//! ```rust, ignore
//! fn EffectComponent() -> Element {
//!     let id = use_signal(0);
//!     use_effect(move || {
//!         let id = id.read();
//!         // This will panic if the id is not written to the DOM before the effect is run
//!         document::eval(format!(r#"document.getElementById("{id}").innerHTML = "Hello World";"#));
//!     });
//!
//!     rsx! {
//!         div { id: "{id}" }
//!     }
//! }
//! ```
//!
//! 3. Observing out of date state. Related issues (<https://github.com/DioxusLabs/dioxus/issues/1935>)
//!
//! Where ever possible, updates should happen in an order that makes it impossible to observe an out of date state.
//! ```rust, ignore
//! fn OutOfDateComponent() -> Element {
//!     let id = use_signal(0);
//!     // When you read memo, it should **always** be two times the value of id
//!     let memo = use_memo(move || id() * 2);
//!     assert_eq!(memo(), id() * 2);
//!
//!     // This should be true even if you update the value of id in the middle of the component
//!     id += 1;
//!     assert_eq!(memo(), id() * 2);
//!
//!     rsx! {
//!         div { id: "{id}" }
//!     }
//! }
//! ```
//!
//! ## Implementation
//!
//! There are three different types of queued work that can be run by the virtualdom:
//! 1. Dirty Fibers:
//!    Description: When a scope is marked dirty, its fiber is scheduled for diffing. This causes the scope to rerun and update the DOM if any changes are detected during diffing.
//!    Priority: These are the highest priority tasks. Dirty fibers are diffed in order from the scope closest to the root to the scope furthest from the root. We follow this order to ensure that if a higher component reruns and drops a lower component, the lower component will not be run after it should be dropped.
//!
//! 2. Tasks:
//!    Description: Futures spawned in the dioxus runtime each have an unique task id. When the waker for that future is called, the task is rerun.
//!    Priority: These are the second highest priority tasks. They are run after all dirty fibers have been resolved because those fibers may cause children (and the tasks those children own) to drop which should cancel the futures.
//!
//! 3. Effects:
//!    Description: Effects should always run after all changes to the DOM have been applied.
//!    Priority: These are the lowest priority tasks in the scheduler. They are run after all dirty fibers and futures have been resolved. Other tasks may cause components to rerun, which would update the DOM. These effects should only run after the DOM has been updated.

use crate::ScopeId;
use crate::Task;
use crate::VirtualDom;
use crate::innerlude::Effect;
mod api;
mod driver;
mod fairness;
mod message;
mod queues;
mod work;
pub use api::*;
pub(crate) use fairness::SchedulerFairness;
pub(crate) use message::SchedulerMsg;
pub(crate) use queues::*;
pub(crate) use work::{DirtyFiber, Work, WorkCandidate};

impl VirtualDom {
    pub(crate) fn queue_component_props_diff(
        &mut self,
        priority: UpdatePriority,
        updates: Vec<ComponentPropsUpdate>,
    ) {
        let mut updates_to_queue = Vec::new();

        'updates: for update in updates {
            for queued in self
                .component_props_work
                .iter_mut()
                .filter(|queued| queued.priority == priority)
            {
                if let Some(existing) = queued
                    .updates
                    .iter_mut()
                    .find(|existing| existing.scope == update.scope)
                {
                    *existing = update;
                    continue 'updates;
                }
            }

            updates_to_queue.push(update);
        }

        if updates_to_queue.is_empty() {
            return;
        }

        let diff = ComponentPropsDiff {
            priority,
            updates: updates_to_queue,
        };
        match self
            .component_props_work
            .iter()
            .position(|queued| priority < queued.priority)
        {
            Some(index) => self.component_props_work.insert(index, diff),
            None => self.component_props_work.push_back(diff),
        }
    }

    /// Queue a task to be polled
    pub(crate) fn queue_task(&mut self, task: Task, order: ScopeOrder) {
        if !self.scope_has_live_parent_chain(order.id) {
            return;
        }
        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        match dirty_tasks.get(&order) {
            Some(scope) => scope.queue_task(task),
            None => {
                let scope = DirtyTasks::from(order);
                scope.queue_task(task);
                dirty_tasks.insert(scope);
            }
        }
    }

    /// Queue a scope's fiber to be diffed
    pub(crate) fn queue_scope(&mut self, order: ScopeOrder) {
        if !self.scope_has_live_parent_chain(order.id) {
            return;
        }
        self.dirty_fibers.insert(order);
    }

    /// Check if any scopes are queued for diffing.
    pub fn has_dirty_scopes(&self) -> bool {
        self.has_dirty_fibers()
    }

    pub(crate) fn has_dirty_fibers(&self) -> bool {
        !self.dirty_fibers.is_empty() || !self.component_props_work.is_empty()
    }

    pub(crate) fn next_work_priority(&mut self) -> Option<UpdatePriority> {
        self.next_work_candidate(false)
            .map(|(_, order)| order.priority)
            .or_else(|| {
                (!self.runtime.pending_effects.borrow().is_empty()).then_some(UpdatePriority::Idle)
            })
    }

    pub(crate) fn deferred_priority_for_subtree(
        &self,
        id: ScopeId,
        current: UpdatePriority,
    ) -> Option<UpdatePriority> {
        let dirty_fiber_priority = self
            .dirty_fibers
            .iter()
            .filter(|order| order.priority > current)
            .filter(|order| {
                self.scope_has_live_parent_chain(order.id)
                    && (order.id == id || self.runtime.is_descendant_of(order.id, id))
            })
            .map(|order| order.priority)
            .min();

        let component_props_priority = self
            .component_props_work
            .iter()
            .filter(|diff| diff.priority > current)
            .filter(|diff| {
                diff.updates.iter().any(|update| {
                    self.runtime.try_get_state(update.scope).is_some()
                        && (update.scope == id || self.runtime.is_descendant_of(update.scope, id))
                })
            })
            .map(|diff| diff.priority)
            .min();

        dirty_fiber_priority
            .into_iter()
            .chain(component_props_priority)
            .min()
    }

    fn first_dirty_fiber(&mut self) -> Option<ScopeOrder> {
        loop {
            let order = self.dirty_fibers.first()?;
            if self.scope_has_live_parent_chain(order.id) {
                return Some(order);
            }
            self.dirty_fibers.pop_first();
        }
    }

    fn first_dirty_fiber_lower_priority_than(
        &self,
        priority: UpdatePriority,
    ) -> Option<ScopeOrder> {
        self.dirty_fibers
            .iter()
            .copied()
            .filter(|order| order.priority > priority && self.scope_has_live_parent_chain(order.id))
            .min()
    }

    fn scope_has_live_parent_chain(&self, scope_id: ScopeId) -> bool {
        let mut current = scope_id;
        while let Some(state) = self.runtime.try_get_state(current) {
            let parent = state.parent_id();
            drop(state);
            let Some(parent) = parent else { break };
            if self.runtime.try_get_state(parent).is_none() {
                return false;
            }
            current = parent;
        }
        self.runtime.try_get_state(scope_id).is_some()
    }

    fn component_props_order(
        &self,
        _index: usize,
        diff: &ComponentPropsDiff,
    ) -> Option<ScopeOrder> {
        diff.updates
            .iter()
            .filter_map(|update| {
                self.runtime.try_get_state(update.scope).map(|scope| {
                    ScopeOrder::with_priority(scope.height, update.scope, diff.priority)
                })
            })
            .min()
    }

    fn first_component_props_order(&self) -> Option<(usize, ScopeOrder)> {
        self.component_props_work
            .iter()
            .enumerate()
            .filter_map(|(index, diff)| {
                self.component_props_order(index, diff)
                    .map(|order| (index, order))
            })
            .min_by_key(|(_, order)| *order)
    }

    fn first_component_props_order_at_priority(
        &self,
        priority: UpdatePriority,
    ) -> Option<(usize, ScopeOrder)> {
        self.component_props_work
            .iter()
            .enumerate()
            .filter(|(_, diff)| diff.priority == priority)
            .filter_map(|(index, diff)| {
                self.component_props_order(index, diff)
                    .map(|order| (index, order))
            })
            .min_by_key(|(_, order)| *order)
    }

    fn dirty_ancestor_for(
        &self,
        scope_id: ScopeId,
        priority: UpdatePriority,
    ) -> Option<ScopeOrder> {
        self.dirty_fibers
            .iter()
            .copied()
            .filter(|order| {
                order.id != scope_id
                    && order.priority >= priority
                    && self.scope_has_live_parent_chain(order.id)
                    && self.runtime.is_descendant_of(scope_id, order.id)
            })
            .min_by(|left, right| {
                left.height
                    .cmp(&right.height)
                    .then(left.priority.cmp(&right.priority))
                    .then(left.id.cmp(&right.id))
            })
    }

    fn dependency_for_candidate(
        &self,
        candidate: WorkCandidate,
        order: ScopeOrder,
    ) -> (WorkCandidate, ScopeOrder) {
        self.dirty_ancestor_for(order.id, order.priority)
            .map(|ancestor| (WorkCandidate::Fiber, ancestor))
            .unwrap_or((candidate, order))
    }

    fn first_dirty_task_order(&mut self) -> Option<ScopeOrder> {
        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        let mut dirty_task = dirty_tasks.first();
        while let Some(task) = dirty_task {
            if task.tasks_queued.borrow().is_empty()
                || !self.scope_has_live_parent_chain(task.order.id)
            {
                dirty_tasks.pop_first();
                dirty_task = dirty_tasks.first()
            } else {
                break;
            }
        }
        dirty_task.map(|task| task.order)
    }

    fn first_dirty_task_order_at_priority(&self, priority: UpdatePriority) -> Option<ScopeOrder> {
        self.runtime
            .dirty_tasks
            .borrow()
            .iter()
            .filter(|task| task.order.priority == priority)
            .filter(|task| {
                !task.tasks_queued.borrow().is_empty()
                    && self.scope_has_live_parent_chain(task.order.id)
            })
            .map(|task| task.order)
            .min()
    }

    fn next_work_candidate_at_priority(
        &self,
        priority: UpdatePriority,
    ) -> Option<(WorkCandidate, ScopeOrder)> {
        let dirty_fiber = self
            .dirty_fibers
            .iter()
            .copied()
            .filter(|order| order.priority == priority)
            .filter(|order| self.scope_has_live_parent_chain(order.id))
            .min();
        let dirty_task = self.first_dirty_task_order_at_priority(priority);
        let dirty_fragment = self.first_component_props_order_at_priority(priority);

        let mut selected = None;
        for (candidate, order) in [
            dirty_fiber.map(|order| (WorkCandidate::Fiber, order)),
            dirty_task.map(|order| (WorkCandidate::Task, order)),
            dirty_fragment.map(|(index, order)| (WorkCandidate::Fragment(index), order)),
        ]
        .into_iter()
        .flatten()
        {
            if selected
                .as_ref()
                .is_none_or(|(_, selected_order)| order < *selected_order)
            {
                selected = Some((candidate, order));
            }
        }

        selected
    }

    fn next_work_candidate(
        &mut self,
        allow_fair_lane_start: bool,
    ) -> Option<(WorkCandidate, ScopeOrder)> {
        let dirty_fiber = self.first_dirty_fiber();
        #[cfg(debug_assertions)]
        if let Some(scope) = &dirty_fiber {
            assert!(self.scopes.contains(scope.id.0));
            assert!(self.scope_has_live_parent_chain(scope.id));
        }

        let dirty_task = self.first_dirty_task_order();
        let dirty_fragment = self.first_component_props_order();

        let mut selected = None;
        let mut fair = None;
        for (candidate, order) in [
            dirty_fiber.map(|order| (WorkCandidate::Fiber, order)),
            dirty_task.map(|order| (WorkCandidate::Task, order)),
            dirty_fragment.map(|(index, order)| (WorkCandidate::Fragment(index), order)),
        ]
        .into_iter()
        .flatten()
        {
            if selected
                .as_ref()
                .is_none_or(|(_, selected_order)| order < *selected_order)
            {
                selected = Some((candidate, order));
            }
        }

        let Some((selected_candidate, selected_order)) = selected else {
            return None;
        };

        if let Some(active_lane) = self.scheduler_fairness.active_lane()
            && selected_order.priority >= active_lane
        {
            if let Some(candidate) = self.next_work_candidate_at_priority(active_lane) {
                return Some(self.dependency_for_candidate(candidate.0, candidate.1));
            }
            self.scheduler_fairness.clear_active_lane();
        }

        let fair_dirty_fiber = self.first_dirty_fiber_lower_priority_than(selected_order.priority);
        for (candidate, order) in [
            fair_dirty_fiber.map(|order| (WorkCandidate::Fiber, order)),
            dirty_task.map(|order| (WorkCandidate::Task, order)),
            dirty_fragment.map(|(index, order)| (WorkCandidate::Fragment(index), order)),
        ]
        .into_iter()
        .flatten()
        .filter(|(_, order)| order.priority > selected_order.priority)
        {
            if fair
                .as_ref()
                .is_none_or(|(_, fair_order)| order < *fair_order)
            {
                fair = Some((candidate, order));
            }
        }

        let candidate = if allow_fair_lane_start
            && self
                .scheduler_fairness
                .should_run_lower_priority_work(selected_order.priority, fair.is_some())
        {
            let fair = fair.unwrap();
            self.scheduler_fairness.start_active_lane(fair.1.priority);
            fair
        } else {
            (selected_candidate, selected_order)
        };

        Some(self.dependency_for_candidate(candidate.0, candidate.1))
    }

    /// Take the top task from the highest scope
    pub(crate) fn pop_task(&mut self) -> Option<Task> {
        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        let tasks = dirty_tasks.first()?;

        // The scope that owns the effect should still exist. We can't just ignore the task if the scope doesn't exist
        // because the scope id may have been reallocated
        debug_assert!(self.scopes.contains(tasks.order.id.0));

        let mut tasks = tasks.tasks_queued.borrow_mut();
        let task = tasks.pop_front()?;
        if tasks.is_empty() {
            drop(tasks);
            dirty_tasks.pop_first();
        }
        Some(task)
    }

    /// Take any effects from the highest scope. This should only be called if there is no pending fiber diff or tasks
    pub(crate) fn pop_effect(&mut self) -> Option<Effect> {
        let mut pending_effects = self.runtime.pending_effects.borrow_mut();
        let effect = pending_effects.pop_first()?;

        // The scope that owns the effect should still exist. We can't just ignore the effect if the scope doesn't exist
        // because the scope id may have been reallocated
        debug_assert!(self.scopes.contains(effect.order.id.0));

        Some(effect)
    }

    /// Take any work from the highest scope. This may include diffing a fiber and/or running tasks
    pub(crate) fn pop_work(&mut self) -> Option<Work> {
        let Some((candidate, order)) = self.next_work_candidate(true) else {
            return self.pop_effect().map(Work::RunEffect);
        };
        self.scheduler_fairness.record(order.priority);

        match candidate {
            WorkCandidate::Fiber => self
                .dirty_fibers
                .remove_exact(&order)
                .then_some(order)
                .map(|scope| Work::DiffFiber(DirtyFiber::new(scope))),
            WorkCandidate::Task => Some(Work::PollTask(self.pop_task().unwrap())),
            WorkCandidate::Fragment(index) => self
                .component_props_work
                .remove(index)
                .map(Work::DiffComponentProps),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_fibers_pop_priority_before_scope_order() {
        let mut queue = DirtyFiberQueue::default();
        queue.insert(ScopeOrder::with_priority(
            0,
            ScopeId(0),
            UpdatePriority::Transition,
        ));
        queue.insert(ScopeOrder::with_priority(
            10,
            ScopeId(1),
            UpdatePriority::SyncInput,
        ));

        assert_eq!(queue.pop_first().unwrap().id, ScopeId(1));
        assert_eq!(queue.pop_first().unwrap().id, ScopeId(0));
    }

    #[test]
    fn dirty_fibers_keep_scope_order_within_priority() {
        let mut queue = DirtyFiberQueue::default();
        queue.insert(ScopeOrder::with_priority(
            1,
            ScopeId(2),
            UpdatePriority::Default,
        ));
        queue.insert(ScopeOrder::with_priority(
            10,
            ScopeId(1),
            UpdatePriority::Default,
        ));

        assert_eq!(queue.pop_first().unwrap().id, ScopeId(1));
        assert_eq!(queue.pop_first().unwrap().id, ScopeId(2));
    }

    #[test]
    fn dirty_fibers_keep_multiple_priorities_for_existing_scope() {
        let mut queue = DirtyFiberQueue::default();
        queue.insert(ScopeOrder::with_priority(
            0,
            ScopeId(0),
            UpdatePriority::Transition,
        ));
        queue.insert(ScopeOrder::with_priority(
            0,
            ScopeId(0),
            UpdatePriority::SyncInput,
        ));

        let order = queue.pop_first().unwrap();
        assert_eq!(order.id, ScopeId(0));
        assert_eq!(order.priority, UpdatePriority::SyncInput);
        assert_eq!(
            queue.deferred_priority_for_scope(ScopeId(0), UpdatePriority::SyncInput),
            Some(UpdatePriority::Transition)
        );
        let order = queue.pop_first().unwrap();
        assert_eq!(order.id, ScopeId(0));
        assert_eq!(order.priority, UpdatePriority::Transition);
        assert!(queue.is_empty());
    }

    #[test]
    fn update_priority_classifies_event_names() {
        assert_eq!(
            UpdatePriority::from_event_name("click"),
            UpdatePriority::SyncInput
        );
        assert_eq!(
            UpdatePriority::from_event_name("onkeydown"),
            UpdatePriority::SyncInput
        );
        assert_eq!(
            UpdatePriority::from_event_name("pointermove"),
            UpdatePriority::ContinuousInput
        );
        assert_eq!(
            UpdatePriority::from_event_name("load"),
            UpdatePriority::Default
        );
    }
}
