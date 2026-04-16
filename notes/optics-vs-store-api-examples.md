# Optic vs Existing Store API

This note compares the unified `Optic` API shape against the current store API
in `packages/stores`.

This is a comparison of user-facing API design, not subscription semantics.
Stores still win on path-aware invalidation and current framework integration.

There is one important assumption in the `Optic` snippets below:

- the same derive machinery that currently generates field and variant accessors
  for `Store<T, Lens>` would generate accessors on `Optic<_, _>`

So when a snippet shows `todo.checked()` or `state.todos()`, that is not a
claim that `packages/optics` already ships those derive-generated methods.
It is a claim about the API shape you get if the existing derive surface is
retargeted onto the unified optics model.

## 1. Explicit Read/Write Terminals

Current store API:

```rust
let checked = item.checked();
let current: bool = checked();
checked.set(true);
```

Unified optic API:

```rust
let checked = item.checked();
let current: bool = checked.value();
*checked.write() = true;
```

Why this is better:

- the access mode is part of the method name
- reads, writes, owned snapshots, and async extraction all follow the same
  terminal pattern
- there is no special call syntax for "read the current value"

The store API is compact, but it mixes two ideas:

- path selection: `checked()`
- extraction: also `checked()`

The optic version keeps those separate:

- path selection: `checked()`
- extraction: `read`, `write`, `value`, `future`

References:
[packages/stores-macro/src/lib.rs](../packages/stores-macro/src/lib.rs),
[packages/optics/src/signal.rs](../packages/optics/src/signal.rs).

## 2. Optional Paths Stay Inside The Pipeline

Current store API:

```rust
let entry = todos.todos().get(id).unwrap();
let contents = entry.contents();
let text: String = contents();
```

Unified optic API:

```rust
let contents = todos.todos().get(id).contents();
let text: Option<String> = contents.value();
```

Why this is better:

- `get(id)` does not force you out into `Option<Store<_>>`
- you keep composing the path before choosing how to handle absence
- the same field accessor works on required and optional paths

This is one of the clearest API wins. In the store API, optionality escapes the
projection system:

- first you get `Option<Store<_>>`
- then you have to unwrap or branch
- only then can you keep projecting fields

In the unified optic model, optionality remains part of the path and only shows
up at the terminal.

Reference:
[examples/01-app-demos/todomvc_store.rs](../examples/01-app-demos/todomvc_store.rs).

## 3. No Generated `transpose()` Type Families

Current store API:

```rust
let TodoItemStoreTransposed { checked, contents } = store.transpose();

use EnumStoreTransposed::*;
match store.transpose() {
    Bar(bar) => { /* ... */ }
    Baz { foo, bar } => { /* ... */ }
    _ => {}
}
```

Unified optic API:

```rust
let checked = store.checked();
let contents = store.contents();

let bar = store.bar();
let foo = store.foo();
```

Why this is better:

- no generated mirror struct types like `TodoItemStoreTransposed`
- no generated mirror enum types like `EnumStoreTransposed`
- destructuring does not require a second type-level representation of your
  data model

The current store API solves destructuring ergonomically, but it does it by
creating a parallel generated type family for every store-backed type.

The unified optic model does not need that. The accessors themselves are the
destructuring surface.

Reference:
[packages/stores-macro/src/lib.rs](../packages/stores-macro/src/lib.rs),
[packages/stores/tests/marco.rs](../packages/stores/tests/marco.rs).

## 4. Variants Do Not Need Two APIs

Current store API:

```rust
let foo = store.is_foo();
let bar = store.is_bar();

let bar_value: Option<Store<String, _>> = store.bar();
if let Some(bar_value) = bar_value {
    println!("{bar_value}");
}
```

Unified optic API:

```rust
let bar = store.bar();

if let Some(value) = bar.read_opt() {
    println!("{value}");
}

let is_bar = bar.read_opt().is_some();
```

Why this is better:

- one accessor gives you both the optional path and the optional read
- there is no need for both `is_bar()` and `bar()`
- the same terminal vocabulary works for enum variants, map lookups, and
  optional fields

Today the store API has two separate surfaces for variants:

- predicate methods like `is_bar()`
- downcast methods like `bar() -> Option<Store<...>>`

The unified optic model reduces that to one path operation plus the normal
optional terminals.

Reference:
[packages/stores-macro/src/lib.rs](../packages/stores-macro/src/lib.rs),
[packages/stores/tests/marco.rs](../packages/stores/tests/marco.rs).

## 5. Nested Optional Composition Keeps Reading Left-To-Right

Current store API:

```rust
let entry = todos.todos().get(id).unwrap();
let checked = entry.checked();
checked.set(true);
```

Unified optic API:

```rust
*todos
    .todos()
    .get(id)
    .checked()
    .write_opt()
    .unwrap() = true;
```

Why this is better:

- the code keeps following the data path left-to-right
- you do not need to materialize intermediate store variables just to get past
  an `Option`
- the final terminal makes the optionality explicit exactly where it matters

This is the strongest ergonomic argument for keeping optionality in the path
type instead of in `Option<Store<_>>`.

Reference:
[examples/01-app-demos/todomvc_store.rs](../examples/01-app-demos/todomvc_store.rs),
[packages/optics/tests/optics.rs](../packages/optics/tests/optics.rs).

## 6. Collections Still Return The Same Wrapper

Current store API:

```rust
let mut children = value.children();
for child in children.iter() {
    child.count().set(1);
}
```

Unified optic API:

```rust
let mut children = value.children();
for child in children.iter() {
    *child.count().write() = 1;
}
```

Why this is better:

- the collection child still uses the same extraction vocabulary as the root
- there is no special mutation API for collection elements
- indexed, keyed, optional, and root paths all terminate the same way

This is not a huge difference in amount of code, but it is a significant
difference in consistency. The same terminal names survive across:

- root fields
- iterated children
- indexed children
- keyed children

References:
[packages/stores/README.md](../packages/stores/README.md),
[packages/optics/tests/optics.rs](../packages/optics/tests/optics.rs).

## 7. The Async Story Is Actually One Story

Current store API:

```rust
let checked = store.checked();
let current: bool = checked();

// For async/resource cases you leave the normal store API and go through
// use_resource / resource-specific machinery.
```

Unified optic API:

```rust
let checked = resource.todo().checked();

let current = checked.read_opt();
let snapshot: Option<bool> = checked.value();
let next = checked.future();
```

Why this is better:

- sync and async do not require different projection vocabularies
- `future()` is just another terminal on the same path
- the access mode is chosen last in the same way for sync and async

The existing store API is good at nested sync state. The unified optic model is
better if the design goal is one lens API that also reaches async carriers.

Reference:
[packages/stores/src/store.rs](../packages/stores/src/store.rs),
[packages/optics/src/resource.rs](../packages/optics/src/resource.rs).

## 8. The Type Count Drops

Current store API introduces several user-facing forms:

- `Store<T, Lens>`
- `Option<Store<T, Lens>>`
- generated field accessor traits
- generated `transpose()` mirror structs
- generated `transpose()` mirror enums
- predicate methods plus downcast methods for variants

Unified optic API reduces that to:

- `Optic<A, Required>`
- `Optic<A, Optional>`
- one terminal vocabulary: `read`, `write`, `read_opt`, `write_opt`, `value`,
  `future`

Why this is better:

- fewer concepts have to be learned
- optionality becomes a path-state detail instead of a wrapper escape hatch
- extensions can target one abstraction instead of a family of generated helper
  types

This is the main API argument in one sentence:

The store API is a good nested state API, but the unified optic API is a
smaller lens language.

## Bottom Line

Compared to the existing store API, the unified optic model is better in these
specific ways:

1. access mode is explicit and chosen last
2. optionality stays in the path instead of escaping into `Option<Store<_>>`
3. enum variants do not need separate predicate and downcast APIs
4. `transpose()` mirror types are no longer necessary
5. the same path vocabulary can extend naturally to async carriers

What this note does **not** claim:

- that optics already has better derive integration than stores
- that optics already has better subscriptions than stores
- that optics should replace the store runtime as-is

It only argues that if Dioxus wants one lens abstraction that can grow across
stores, resources, memos, and future carriers, `Optic` is a cleaner public API
surface than the current generated store API.
