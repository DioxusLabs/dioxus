# Refract: a store/lens-first reactive UI runtime

Refract is a from-scratch exploration of what Dioxus would look like if the
*only* state primitive were a store, and every derived view of state — fields,
collection items, memos, resources — were a lens over some store. It borrows
the `Deref`/`DerefMut` dirty-tracking idea from
[ealmloff/qk](https://github.com/ealmloff/qk) (`RwTrack` marks a read on
`deref` and a write on `deref_mut`) but moves the graph from compile time to a
small runtime so lenses can be composed dynamically.

## Goals

- **Stores + lenses only.** No `use_state`/`Signal` zoo. One root primitive
  (`Store<T>`), one composition primitive (`Lens`), and two derived node kinds
  (`Memo`, `Resource`) that are themselves readable through the same lens
  machinery.
- **`Deref`/`DerefMut` for tracking.** Reading happens through guards.
  `ReadGuard: Deref` subscribes the current observer on first deref.
  `WriteGuard: Deref + DerefMut` only notifies subscribers if `deref_mut` was
  actually called (mirroring qk's `RwTrack`), so `guard.len()` on a
  `WriteGuard` does not spuriously invalidate.
- **Zero-copy values.** Guards hand out `&T`/`&mut T` projected with
  `Ref::map`/`RefMut::map` through the lens chain. Values are never cloned to
  be observed; `PartialEq` cloning is only used inside memos for change
  detection.
- **Path-granular invalidation.** A lens carries a structural path
  (`[3, 0, 7]` = field 3 → index 0 → field 7). A write at path `P` invalidates
  a subscriber at path `Q` iff one is a prefix of the other: writing a field
  wakes readers of the whole struct, and writing the whole struct wakes
  readers of each field, but sibling fields never cross-talk.

## Storage: boxes as poor-man's `Pin`

All reactive values live in a thread-local runtime slab:

```text
Runtime
└── Slab<Node>
     └── Node { value: Box<RefCell<dyn Any>>, subs, deps, state, version, owned }
```

Handles (`Store<T>`, `Memo<T>`, lenses) are `Copy` — a generational id plus
`fn` pointers, no `Rc`. That means guards cannot borrow from the handle; they
must borrow from the runtime, which forces lifetime erasure:

```rust
pub struct ReadGuard<T: ?Sized + 'static> { inner: Ref<'static, T>, .. }
```

This is where `Pin` gets tricky. The `Ref<'static, T>` points into the heap
allocation of the node's `Box<RefCell<dyn Any>>`. The slab `Vec` may grow and
move `Node`s freely — that is fine, because the box's allocation never moves
(the box *is* our pinning mechanism; we do not need `Pin<Box<_>>` because we
never expose `&mut` to the box itself, only to its contents through the
`RefCell`). The single safety obligation is that **the box must not be freed
while a guard borrows it**. Two rules enforce this:

1. Handles are generational. A dangling handle panics cleanly instead of
   reusing a slot.
2. Dropping a node first tries `try_borrow_mut()` on its value. If a guard is
   still alive, the box is moved to a graveyard and freed on a later flush,
   after the borrow flag clears. Memory is quarantined, never invalidated.

The only `unsafe` in the crate is the lifetime transmute of `Ref`/`RefMut`
in `runtime.rs`, justified by the two rules above.

## Lenses

A lens is a `Copy` chain of projections rooted at a node:

```rust
struct Lens<P: Readable, T> {
    parent: P,
    segment: u32,                        // structural path element
    map: fn(&P::Target) -> &T,          // zero-copy projection
    map_mut: fn(&mut P::Target) -> &mut T,
}
```

Composition is type-level (`Lens<Lens<Store<App>, Todos>, Todo>`), like
iterator adapters, so no boxing is needed and everything stays `Copy`.
`Readable::project` recursively applies `Ref::map`, and
`Readable::push_path` recursively rebuilds the structural path at
subscription time — the handle itself never allocates.

Collections use `IndexLens` (the segment *is* the index), so pushing to a
`Vec` through the parent lens wakes per-item subscribers via the prefix rule,
while editing item 3 leaves item 4's subscribers untouched.

## The memo/resource design (the hard part)

### Why memos are dangerous with lenses

A memo must cache its value somewhere that lenses can project into with
`Ref::map` — i.e. a `RefCell` in the same slab as stores. But recomputing the
memo needs `&mut` access to that same cell, and recomputation is triggered
*lazily by reads*. If recomputation could happen while a `ReadGuard` into the
memo is alive, we would need to invalidate live `&T` references — undefined
behavior in Rust, and exactly the kind of thing frameworks with lifetime-
erased guards get wrong.

Refract's rule: **a memo is brought up to date strictly before its guard is
created, never while one exists.**

`memo.read()`:

1. `ensure(node)` — recompute if needed (may recursively ensure
   dependencies). This takes the `&mut` borrow and releases it.
2. Only then borrow the cell and build the `Ref`-projected guard.

If step 1 needs `&mut` while an older guard on the same memo is still alive,
`RefCell` panics with a clear message rather than invalidating references.
Holding a guard across a write to its own dependencies is a bug in user code;
the failure mode is a deterministic panic, never UB.

### Correct staleness: Clean / Check / Dirty

Equality-gated memos need more than a dirty bit. If `a → b → c` and `a`
changes but `b` recomputes to an equal value, `c` must *not* rerun. Refract
uses the three-state algorithm (à la Reactively/adapton):

- A store write marks overlapping direct subscribers **Dirty** and their
  transitive subscribers **Check**.
- `ensure(node)` on a **Check** node first ensures each memo dependency, then
  compares each dependency's `version` against the version recorded when the
  dep was captured. Only if a version actually advanced does the node become
  **Dirty** and recompute.
- Recomputation bumps `version` only when the new value is `!=` the old, so
  unchanged memos cut off propagation.

Cycles are caught with a **Computing** state and panic with the node's chain.

### Recomputation is a scope reset

Before a memo/effect reruns, its previous dependency edges are unsubscribed
and everything it *owns* (stores, memos, effects, DOM bindings created during
the last run) is dropped, children-first. This is what makes effects behave
like components: conditional UI created inside an effect is torn down
automatically when the effect reruns. Ownership is established implicitly:
nodes created while an observer is running are owned by that observer.

### Resources

A resource is "an async memo": a node whose value is a `ResourceState<T>`
(`Pending`, `Ready(T)`, `Reloading(T)`) plus a driver future.

- The `Fn() -> Future` source closure runs **synchronously** under tracking;
  reads made while constructing the future are the resource's dependencies.
  Reads after the first `await` are intentionally untracked (tracking across
  `await` with thread-local observers would misattribute deps from other
  tasks — this matches Dioxus `use_resource`).
- When a dependency changes, the old future is dropped (cancellation is
  `Drop`) and a new one is created; a `Ready` value degrades to
  `Reloading(old)` so the UI can keep showing stale data — zero-copy, the old
  value is moved, not cloned.
- **Pin, again:** the future is stored as `Pin<Box<dyn Future>>` *outside*
  the value cell, in the node's driver slot. It must be boxed (self-
  referential after first poll ⇒ stable address required) and it must not
  live in the value `RefCell`, otherwise polling (which needs `&mut` to the
  future) would conflict with lens reads of the resource state. Keeping
  "value readable through lenses" and "future pollable" in separate cells is
  what makes the borrow story sound.
- Completion writes into the value cell through the normal write path, so
  lenses over `resource` (e.g. projecting into `Ready`'s payload with
  `try_read`) invalidate like any store write.
- Polling uses flag-wakers (`Arc<AtomicBool>` via `std::task::Wake`), pumped
  by `flush()`; `Runtime::run_until_settled` drives examples/tests without an
  external executor.

## UI layer

A deliberately small retained DOM (`dom.rs`): `el("div")`, `text`,
`dyn_text(move || …)`, `dyn_attr`, and `dyn_children` — each `dyn_*` is just
an effect that patches the retained node in place. There is no diffing and no
VDOM: granularity comes from the lens graph, not from tree comparison.
`render_to_string` exists for tests and terminal examples.
