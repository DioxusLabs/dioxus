# Suspense, Async, and More

This doc goes into the design of asynchronicity in Dioxus.


## for UI elements

`suspend`-ing a future submits an &mut future to Dioxus. the future must return VNodes. the future is still waiting before the component renders, the `.await` is dropped and tried again. users will want to attach their future to a hook so the future doesn't really get dropped.


## for tasks

for more general tasks, we need some way of submitting a future or task into some sort of task system. 


`use_task()` submits a future to Dioxus. the future is polled infinitely until it finishes. The caller of `use_task` may drop, pause, restart, or insert a new the task

```rust

let task = use_hook(|| { /* */ });
cx.poll_future()
// let recoil_event_loop = cx.use_task(move |_| async move {
//     loop {
//         let msg = receiver.await?;
//     }
// });
// where suspend wraps use_task
```
