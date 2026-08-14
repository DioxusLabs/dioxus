# Refract: a store/lens-first reactive UI runtime

Refract is a from-scratch exploration of what a Dioxus-like framework looks
like if the *only* state primitive is a store, and every view of state —
fields, collection items, memos, resources — is a lens over it. It borrows the
`Deref`/`DerefMut` dirty-tracking idea from
[ealmloff/qk](https://github.com/ealmloff/qk) (`RwTrack` marks a read on
`deref` and a write on `deref_mut`), and takes qk's other core stance
seriously: **exclusivity is enforced by the borrow checker, not at runtime**.

## Goals

- **Stores + lenses only.** One root store (`Ui<S>` owns `S`), one composition
  primitive (`Lens`), and two derived node kinds (`Memo`, `Resource`) readable
  through the same guard machinery.
- **`Deref`/`DerefMut` for tracking.** `ReadGuard: Deref` subscribes the
  current observer on first deref. `WriteGuard: Deref + DerefMut` only
  notifies subscribers if `deref_mut` was actually called (mirroring qk's
  `RwTrack`), so `guard.len()` on a `WriteGuard` never spuriously invalidates.
- **Zero-copy, no `RefCell`, no runtime borrow checking.** Guards wrap plain
  `&T` / `&mut T` projected through the lens chain. There is no interior
  mutability around state, no borrow counters, no lifetime transmutes, and no
  `unsafe`. If two accesses could alias, the program does not compile.
- **Path-granular invalidation.** A lens carries a structural path
  (`[3, 0, 7]` = field 3 → index 0 → field 7). A write at path `P` invalidates
  a subscriber at path `Q` iff one is a prefix of the other: writing a field
  wakes readers of the whole struct and vice versa, but sibling fields never
  cross-talk.

## Storage: the runtime owns the state, closures borrow it

```text
Ui<S>
├── state: S                      // the one root store, owned directly
├── memos:     Vec<MemoSlot>      // value: Box<dyn Any>, version, deps, state
├── effects:   Vec<EffectSlot>    // run: FnMut(&mut Ctx<S>), deps, owned
├── resources: Vec<ResourceSlot>  // value + *separate* driver future
└── log: Log                      // Cell-based append-only read/write log
```

Nothing reactive is reachable from user code except through `&mut Ui<S>` or
the `&mut Ctx<'_, S>` handed to reactive closures:

```rust
pub struct Ctx<'a, S> {
    state: &'a mut S,   // exclusive by construction
    log: &'a Log,
    /* split borrows of the memo/effect/resource tables */
}
```

Every effect, memo computation, and resource source is
`FnMut(&mut Ctx<'_, S>)`. Since all state access flows through one `&mut`
chain, Rust's aliasing rules are the borrow discipline — a guard borrows the
`Ctx`, so you cannot hold a read guard while writing, or two write guards at
once, and the compiler says so at compile time. Multiple simultaneous *read*
guards work fine (`ctx.get` takes `&Ctx`).

Lenses are pure `Copy` values (a chain of `fn(&U) -> &T` / `fn(&mut U) -> &mut T`
projections plus path segments) — they hold no reference and no id, so they
can be moved into closures freely and applied to whichever `&mut S` the
runtime is currently threading through.

The one interior-mutability concession is the read/write **log**, which uses
`Cell<Vec<_>>` (take → push → put back). Cell ops cannot fail or alias —
there is nothing like a `BorrowMutError` in the system.

## The memo/resource design (the hard part)

### Why memos are dangerous with lenses

A memo's cached value must be readable through the same guard machinery as
state, but recomputation needs `&mut` to that cache and is triggered lazily
*by reads*. If a stale memo could recompute while a reference into its old
value is alive, that reference would dangle. Runtimes with lifetime-erased
guards paper over this with runtime borrow flags; refract makes it a borrow
error instead:

```rust
pub fn read_memo<T>(&mut self, memo: Memo<T>) -> &T
```

`read_memo` takes `&mut self` (or `&mut Ctx`), brings the memo up to date
**first** (recursively ensuring dependencies), and only then returns `&T`
tied to that `&mut` borrow. While that `&T` lives, the borrow checker forbids
any other `Ctx` operation — recomputation while a guard exists is not a
guarded runtime panic, it is unrepresentable.

Memo values are type-erased as `Box<dyn Any>` in the slot table, but the box
is only touched through `&mut Ctx`, so no cell is needed.

### Correct staleness: Clean / Check / Dirty

Equality-gated memos need more than a dirty bit. If `a → b → c` and `a`
changes but `b` recomputes to an equal value, `c` must *not* rerun. Refract
uses the three-state algorithm (à la Reactively/adapton):

- A store write marks overlapping direct subscribers **Dirty** and their
  transitive subscribers **Check**.
- Ensuring a **Check** node first ensures each memo/resource dependency, then
  compares each dependency's `version` against the version recorded when the
  dep was captured. Only if a version actually advanced does the node become
  **Dirty** and recompute.
- Recomputation bumps `version` only when the new value is `!=` the old, so
  unchanged memos cut off propagation (`PartialEq` is required exactly here
  and nowhere else).

Cycles are caught with a computing flag and panic with a clear message.

### Recomputation is a scope reset

Before a memo/effect reruns, its previous dependency edges are dropped and
everything it *owns* (effects, DOM bindings created during the last run) is
torn down, children-first. This is what makes effects behave like components:
conditional UI created inside an effect is dismantled automatically when the
effect reruns. Ownership is implicit: nodes created while an observer runs
are owned by that observer.

### Resources

A resource is "an async memo": a value slot holding `ResourceState<T>`
(`Pending`, `Ready(T)`, `Reloading(T)`) plus a driver future.

- The `FnMut(&mut Ctx) -> impl Future` source runs **synchronously** under
  tracking; reads made while constructing the future are the resource's
  dependencies. Reads after the first `await` are intentionally impossible —
  the future cannot capture the `Ctx` borrow across `await` (the closure's
  `&mut Ctx` ends when it returns), so dependency misattribution is ruled out
  by lifetimes rather than by convention.
- When a dependency changes, the old future is dropped (cancellation is
  `Drop`) and a new one is created; a `Ready` value degrades to
  `Reloading(old)` so the UI keeps showing stale data — the old value is
  moved, not cloned.
- **Pin:** the future is stored as `Pin<Box<dyn Future>>` *outside* the value
  slot, in a driver slot. It must be boxed (self-referential after the first
  poll ⇒ stable address), and it must not live next to the readable value,
  otherwise polling (which needs `&mut` to the future) would conflict with
  lens reads of the resource state. To poll, the runtime *moves the box out
  of the slot* — moving `Pin<Box<F>>` moves the box, never the pinned future
  inside it — polls, and puts it back if pending. Value-readable and
  future-pollable never alias.
- Completion writes into the value slot through the normal notification path,
  so effects and memos reading the resource wake like any store write.
- Polling uses per-resource flag wakers (`Arc<AtomicBool>` via
  `std::task::Wake`): a future that wakes itself is re-polled, one that
  doesn't stays parked. `Ui::run_until_settled` pumps this loop for
  examples/tests without an external executor.

## UI layer

A deliberately small retained DOM (`dom.rs`): `el("div")`, `text`,
`dyn_text(move |ctx| …)`, `dyn_attr`, and `dyn_children` — each `dyn_*` is
just an effect that patches the retained node in place. There is no diffing
and no VDOM: granularity comes from the lens graph, not from tree comparison.
`render_to_string` exists for tests and terminal examples.

## What the borrow checker buys (and costs)

Compared to the `RefCell`/generational-slab design this replaced:

- No runtime borrow failures — every aliasing bug is a compile error.
- No `unsafe`, no lifetime erasure, no graveyard/quarantine machinery.
- Guards are true zero-cost wrappers over `&T`/`&mut T`.

The cost is that handles are not free-floating: you cannot read state without
a `Ui`/`Ctx` in hand, and `read_memo(a)` and `read_memo(b)` cannot be held
simultaneously (copy one out first). That constraint is the design: the
borrow checker can only prove exclusivity if all access flows through one
place.
