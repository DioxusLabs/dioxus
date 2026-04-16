# Optics vs Projector Comparison

This note compares the experimental optics crate in
[`packages/optics`](../packages/optics) with the projector-based implementation
in:

- [`packages/signals/src/project.rs`](../packages/signals/src/project.rs)
- [`packages/stores/src/project.rs`](../packages/stores/src/project.rs)
- [`packages/hooks/src/use_resource.rs`](../packages/hooks/src/use_resource.rs)

The focus here is narrow:

- lens/projection features
- extension boundaries
- what generated field accessors would and would not compose with

This note is intentionally strict about the difference between:

- what is implemented today
- what would work if the design were extended further

## Current Snapshot

| Question | `dioxus-optics` today | Projector branch today | Comparison |
| --- | --- | --- | --- |
| Core abstraction | `Signal<A>` wraps an arbitrary carrier `A`, and composition is expressed as `Combinator<A, Op>` plus `Transform<Op>` / `Resolve<Op>` ([signal.rs](../packages/optics/src/signal.rs), [combinator.rs](../packages/optics/src/combinator.rs)) | Projection is trait-driven: `ProjectLens`, `ProjectCompose`, `ProjectPath`, `ProjectReact`, and `ProjectAwait` ([project.rs](../packages/signals/src/project.rs)) | Optics is smaller and more algebraic. Projector is broader and more integrated with real Dioxus carrier types. |
| Public extraction boundary | Explicit terminals: `read`, `write`, `read_opt`, `write_opt`, `value`, `future` ([signal.rs](../packages/optics/src/signal.rs)) | Projection mostly stays inside existing signal/store/resource APIs; async extraction is the explicit `project_future` boundary ([project.rs](../packages/signals/src/project.rs), [use_resource.rs](../packages/hooks/src/use_resource.rs)) | Optics now has the cleaner terminal split. Projector keeps extraction closer to each concrete carrier family. |
| Field projection | `map_ref_mut(read, write)` ([signal.rs](../packages/optics/src/signal.rs)) | `project_map(map, map_mut)` and keyed `project_child(...)` ([project.rs](../packages/signals/src/project.rs)) | Same basic capability. Projector adds keyed path scoping. |
| `Option<T>` child projection | `map_some()` plus `read_opt()` / `write_opt()` ([signal.rs](../packages/optics/src/signal.rs)) | `ProjectOption`-style helpers such as `transpose`, `unwrap`, `expect`, `filter`, `as_deref`, and `as_slice` ([use_resource.rs](../packages/hooks/src/use_resource.rs), [project.rs](../packages/signals/src/project.rs)) | Optics is smaller and more uniform. Projector is richer on shape-specific helpers. |
| `Result<T, E>` projection | Not present | Present through `ProjectResult` ([use_resource.rs](../packages/hooks/src/use_resource.rs), [project.rs](../packages/signals/src/project.rs)) | Projector-only today. |
| Generic sync derived values | `lens_map(Fn(In) -> Out)` produces an owned derived sync value ([signal.rs](../packages/optics/src/signal.rs)) | No equally generic sync terminal in the projector traits; the common pattern is dedicated helpers on projected shapes | Optics is cleaner here. |
| Sync `Option<Option<T>>` flatten | `flatten_some()` is a first-class op ([collection.rs](../packages/optics/src/collection.rs)) | Async flatten exists, but there is no equivalent first-class sync flatten helper in the projector surface ([project.rs](../packages/signals/src/project.rs)) | Optics is cleaner here too. |
| Vec child iteration | `each::<T>().iter()` over projected `Vec<T>` children ([collection.rs](../packages/optics/src/collection.rs)) | Slice/vector indexing and iteration through the projector collection traits ([project.rs](../packages/signals/src/project.rs)) | Similar capability. Projector is more complete. |
| Keyed collections | Not present | `HashMap` and `BTreeMap` keyed projection plus async forwarding ([project.rs](../packages/signals/src/project.rs)) | Projector-only today. |
| Async projection | `future()` plus `AwaitTransform`; the same op model applies at await time ([resource.rs](../packages/optics/src/resource.rs)) | `ProjectAwait` plus forwarding through mapped, indexed, and keyed projections ([project.rs](../packages/signals/src/project.rs), [use_resource.rs](../packages/hooks/src/use_resource.rs)) | Same direction. Projector covers more carrier shapes. |
| Path-aware invalidation | Not modeled | `ProjectPath` and `ProjectReact` provide child identity and dirty marking, especially for stores ([project.rs](../packages/signals/src/project.rs), [stores/src/project.rs](../packages/stores/src/project.rs)) | Projector has the real subscription story. |
| Real Dioxus integration | Experimental standalone crate with its own `Signal` wrapper and toy `Resource` ([lib.rs](../packages/optics/src/lib.rs), [resource.rs](../packages/optics/src/resource.rs)) | Implemented on actual Dioxus signal/store/resource machinery ([project.rs](../packages/signals/src/project.rs), [stores/src/project.rs](../packages/stores/src/project.rs), [use_resource.rs](../packages/hooks/src/use_resource.rs)) | Projector is much stronger here. |

## Where Optics Is Cleaner

These are the places where the optics crate currently reads better as a design.

### Explicit terminals

Optics makes the extraction mode part of the method name:

- `read()` / `write()`
- `read_opt()` / `write_opt()`
- `value()`
- `future()`

That is a good fit for composition because the optics path is built first and
the extraction mode is chosen last. It avoids overloading one generic `get()`
for both sync and async use cases ([signal.rs](../packages/optics/src/signal.rs)).

### Small, uniform `Option` story

In optics, optional child access stays inside one pipeline:

- `map_some()`
- `read_opt()`
- `write_opt()`
- `flatten_some()`

That is a smaller surface than the projector shape traits, but it is also
easier to read locally ([signal.rs](../packages/optics/src/signal.rs),
[collection.rs](../packages/optics/src/collection.rs)).

### Generic sync derived values

`lens_map` is a genuinely generic sync terminal:

```rust
let todo_count = app
    .map_ref_mut(app_todos, app_todos_mut)
    .lens_map(todos_len)
    .value();
```

The projector branch has many convenient shape-specific helpers, but not an
equally general sync "map this projection to an owned value" primitive
([signal.rs](../packages/optics/src/signal.rs)).

## Where Projector Is Stronger

These are not just extra helpers. They are the places where projector solves a
larger integration problem.

### It already targets real Dioxus carriers

Projector traits are implemented for:

- `Signal<T, S>`
- `ReadSignal<T, S>`
- `WriteSignal<T, S>`
- `Store<T, Lens>`

([project.rs](../packages/signals/src/project.rs),
[stores/src/project.rs](../packages/stores/src/project.rs))

Resources then reuse the store/projector stack by making `Resource<T>` a store
whose lens carries a handle (`HandledLens`) rather than inventing a second
projection system ([use_resource.rs](../packages/hooks/src/use_resource.rs)).

### It has the real path/reactivity story

The projector branch is not only about lensing. It also carries child identity
and dirty marking:

- `ProjectPath`
- `ProjectReact`
- store-backed path tracking in `SelectorScope`

That is the difference between "you can project a child" and "the framework can
subscribe to exactly that child and invalidate correctly"
([project.rs](../packages/signals/src/project.rs),
[stores/src/project.rs](../packages/stores/src/project.rs)).

### It covers more shapes

The projector surface already includes:

- `Result`-aware projection
- indexed projection
- keyed `HashMap` / `BTreeMap` projection
- async forwarding through mapped, indexed, and keyed children

([project.rs](../packages/signals/src/project.rs))

Optics does not have equivalents for most of that today.

## Generated Accessors: Carrier Matrix

This is the main extension question.

If you generated a field accessor such as `user()` or `todos()`, where would it
work?

| Carrier / environment | `dioxus-optics` today | Projector branch today | What this means |
| --- | --- | --- | --- |
| Root signal-like carrier | Yes, on `Signal<RwRoot<T>>` ([signal.rs](../packages/optics/src/signal.rs)) | Yes, on `Signal`, `ReadSignal`, and `WriteSignal` ([project.rs](../packages/signals/src/project.rs)) | Both designs support projected field access on signal-like roots. |
| Experimental resource carrier | Yes, on the optics crate's own `Resource<T>` ([resource.rs](../packages/optics/src/resource.rs)) | N/A | Optics proves the generic async projection pattern, but only inside its own experimental carrier family. |
| Real `use_resource` resource | No direct support | Yes; `Resource<T>` is a `Store<T, HandledLens<L>>`, and `ProjectAwait` is forwarded through it ([use_resource.rs](../packages/hooks/src/use_resource.rs), [stores/src/project.rs](../packages/stores/src/project.rs)) | Projector wins on actual hook integration. |
| `Store<T, Lens>` | No direct support | Yes ([stores/src/project.rs](../packages/stores/src/project.rs)) | This is the current derive boundary for projector. |
| `ReadSignal` / `WriteSignal` as distinct public types | Not modeled separately | Yes ([project.rs](../packages/signals/src/project.rs)) | Projector is already attached to the existing signal family. |
| `Memo<T>` directly | Not supported today | Not directly shown as a projector carrier | Neither design proves "generated accessors work directly on memos" today. |
| Memo-backed store | Not supported today | Yes, indirectly, by wrapping a `Readable` lens in `Store::from_lens(...)` ([store.rs](../packages/stores/src/store.rs)) | Projector can reach memo-backed data through the store wrapper, not by direct memo projection methods. |
| Bare future as a root carrier | Not supported today | Not supported today | Both designs treat async projection as something a carrier provides, not as "project arbitrary futures directly." |
| Effects / reactive graph integration | Not established | Yes, because projector rides on actual `Readable` / `Writable` / store resource types and `ProjectReact` participates in tracking ([project.rs](../packages/signals/src/project.rs)) | Projector is the one that is already connected to the framework's reactive machinery. |

## What That Means For Derived Accessors

### Optics

If optics gained generated field accessors, the natural shape would be methods on
`Signal<A>`:

```rust
fn user(self) -> Signal<Combinator<A, LensOp<App, Option<User>>>>
fn todos(self) -> Signal<Combinator<A, LensOp<App, Vec<Todo>>>>
```

That is attractive because the accessor itself is just another optics step. The
same generated accessor can then compose with whichever terminals the carrier
already supports:

- `read` / `write`
- `read_opt` / `write_opt`
- `value`
- `future`

But that does **not** mean memos, stores, or arbitrary futures get those
accessors automatically.

It means:

- the accessor shape is carrier-generic
- new carrier families can adopt it if they implement the required outputs
- those integrations do not exist yet

So the optics design has the cleaner generic boundary, but it still needs actual
carrier integrations before claims about memos or stores become true.

### Projector

The projector branch has the opposite tradeoff.

Its generated accessor boundary is currently store-centric. The important code is
already attached to:

- the raw signal family via `ProjectLens`
- stores via store-backed projector impls
- resources via `HandledLens` and `ProjectAwait`

But `derive(Store)` methods land on `Store<_, _>` and resource aliases, not on
every possible carrier type automatically.

So projector is weaker if the design goal is:

- "one generated accessor method should appear on any carrier that can adopt the algebra"

Projector is stronger if the design goal is:

- "generated accessors should compose immediately with the Dioxus signal/store/resource stack that already exists"

## Bottom Line

The two designs are optimized for different layers.

- `dioxus-optics` is the cleaner optics algebra. Its explicit terminals,
  first-class `flatten_some`, and generic `lens_map` make the local composition
  story easier to read and easier to extend in principle.
- The projector branch is the stronger framework integration layer. It already
  works with real Dioxus signal/store/resource types, carries path identity and
  dirty marking, and covers a much larger surface area.

So the comparison is not "which one is strictly better?"

It is:

- optics is better as a small generic model
- projector is better as the current implementation path inside Dioxus

If the goal is to design a future derive/accessor system, the real open question
is whether Dioxus wants to:

- derive at the small optics boundary and then add carrier integrations

or

- derive at the store/projector boundary and accept that carrier coverage is
  broader in practice today but less uniform in principle
