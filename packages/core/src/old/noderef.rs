// let scope = diff_machine.get_scope_mut(&trigger.originator).unwrap();

// let mut garbage_list = scope.consume_garbage();

// let mut scopes_to_kill = Vec::new();
// while let Some(node) = garbage_list.pop() {
//     match &node.kind {
//         VNodeKind::Text(_) => {
//             self.shared.collect_garbage(node.direct_id());
//         }
//         VNodeKind::Anchor(_) => {
//             self.shared.collect_garbage(node.direct_id());
//         }
//         VNodeKind::Suspended(_) => {
//             self.shared.collect_garbage(node.direct_id());
//         }

//         VNodeKind::Element(el) => {
//             self.shared.collect_garbage(node.direct_id());
//             for child in el.children {
//                 garbage_list.push(child);
//             }
//         }

//         VNodeKind::Fragment(frag) => {
//             for child in frag.children {
//                 garbage_list.push(child);
//             }
//         }

//         VNodeKind::Component(comp) => {
//             // TODO: run the hook destructors and then even delete the scope

//             let scope_id = comp.ass_scope.get().unwrap();
//             let scope = self.get_scope(scope_id).unwrap();
//             let root = scope.root();
//             garbage_list.push(root);
//             scopes_to_kill.push(scope_id);
//         }
//     }
// }

// for scope in scopes_to_kill {
//     // oy kill em
//     log::debug!("should be removing scope {:#?}", scope);
// }

// // On the primary event queue, there is no batching, we take them off one-by-one
// let trigger = match receiver.try_next() {
//     Ok(Some(trigger)) => trigger,
//     _ => {
//         // Continuously poll the future pool and the event receiver for work
//         let mut tasks = self.shared.async_tasks.borrow_mut();
//         let tasks_tasks = tasks.next();

//         // if the new event generates work more important than our current fiber, we should consider switching
//         // only switch if it impacts different scopes.
//         let mut ui_receiver = self.shared.ui_event_receiver.borrow_mut();
//         let ui_reciv_task = ui_receiver.next();

//         // right now, this polling method will only catch batched set_states that don't get awaited.
//         // However, in the future, we might be interested in batching set_states across await points
//         let immediate_tasks = ();

//         futures_util::pin_mut!(tasks_tasks);
//         futures_util::pin_mut!(ui_reciv_task);

//         // Poll the event receiver and the future pool for work
//         // Abort early if our deadline has ran out
//         let mut deadline = (&mut deadline_future).fuse();

//         let trig = futures_util::select! {
//             trigger = tasks_tasks => trigger,
//             trigger = ui_reciv_task => trigger,

//             // abort if we're out of time
//             _ = deadline => { return Ok(diff_machine.mutations); }
//         };

//         trig.unwrap()
//     }
// };

// async fn select_next_event(&mut self) -> Option<EventTrigger> {
//     let mut receiver = self.shared.task_receiver.borrow_mut();

//     // drain the in-flight events so that we can sort them out with the current events
//     while let Ok(Some(trigger)) = receiver.try_next() {
//         log::info!("retrieving event from receiver");
//         let key = self.shared.make_trigger_key(&trigger);
//         self.pending_events.insert(key, trigger);
//     }

//     if self.pending_events.is_empty() {
//         // Continuously poll the future pool and the event receiver for work
//         let mut tasks = self.shared.async_tasks.borrow_mut();
//         let tasks_tasks = tasks.next();

//         let mut receiver = self.shared.task_receiver.borrow_mut();
//         let reciv_task = receiver.next();

//         futures_util::pin_mut!(tasks_tasks);
//         futures_util::pin_mut!(reciv_task);

//         let trigger = match futures_util::future::select(tasks_tasks, reciv_task).await {
//             futures_util::future::Either::Left((trigger, _)) => trigger,
//             futures_util::future::Either::Right((trigger, _)) => trigger,
//         }
//         .unwrap();
//         let key = self.shared.make_trigger_key(&trigger);
//         self.pending_events.insert(key, trigger);
//     }

//     // pop the most important event off
//     let key = self.pending_events.keys().next().unwrap().clone();
//     let trigger = self.pending_events.remove(&key).unwrap();

//     Some(trigger)
// }
