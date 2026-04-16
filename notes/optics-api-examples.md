# Optic API Examples

This note gives a small series of examples that justify the unified `Optic`
surface in `packages/optics`.

The claim is not "this is better because it is newer."

The claim is narrower:

- one public wrapper type is easier to teach than separate required/optional
  wrappers
- path composition should be independent from access mode
- the same path algebra should work across sync, async, optional, and
  collection cases

Each example below is mirrored by a real test in
[`packages/optics/tests/optics.rs`](../packages/optics/tests/optics.rs).

## 1. One Wrapper For Normal And Optional Paths

```rust
let active = app
    .map_ref_mut(app_user, app_user_mut)
    .map_some()
    .map_ref_mut(user_active, user_active_mut);

assert_eq!(*active.read_opt().unwrap(), true);
*active.write_opt().unwrap() = false;
```

What this proves:

- the public wrapper stays `Optic<_, _>` the whole way through
- optionality is path state, not a second public API family
- the field combinator does not need a different name for optional children

This is the smallest example of why `Optic<A, Path>` is cleaner than
`Signal<A>` plus `OptionalSignal<A>`:

- the user learns one wrapper
- `map_ref_mut` works the same before and after `map_some`
- only the terminal changes from `read`/`write` to `read_opt`/`write_opt`

Reference:
[`optional_projection_supports_read_and_write`](../packages/optics/tests/optics.rs).

## 2. Nested Shapes Compose Without Changing Abstractions

```rust
let active = Optic::new(Some(Ok::<User, String>(User { active: true })))
    .map_some()
    .map_ok()
    .map_ref_mut(user_active, user_active_mut);

assert_eq!(*active.read_opt().unwrap(), true);

let active = Optic::new(Ok::<Option<User>, String>(Some(User { active: true })))
    .map_ok()
    .map_some()
    .map_ref_mut(user_active, user_active_mut);

assert_eq!(*active.read_opt().unwrap(), true);
```

What this proves:

- nested shape composition is just method chaining
- order matters naturally in the way the data is shaped
- no wrapper conversion is exposed to the caller

This matters because nested data is where overloaded APIs usually start leaking
internal distinctions. Here the public story stays flat:

- `map_some` means "step through `Option`"
- `map_ok` means "step through `Result::Ok`"
- `map_ref_mut` still means "project a field"

Reference:
[`nested_shape_projection_composes`](../packages/optics/tests/optics.rs).

## 3. Access Mode Is Chosen Last

```rust
let active = Optic::new(Some(Ok::<User, String>(User { active: true })))
    .map_some()
    .map_ok()
    .map_ref_mut(user_active, user_active_mut);

let borrowed = active.read_opt().map(|value| *value);
let owned: Option<bool> = active.value();
```

What this proves:

- the path is built first
- the terminal decides the access mode
- borrow-oriented and owned extraction do not require different path builders

That is the main API advantage of the current design. The path algebra does not
change because the caller wants:

- a borrow now
- an owned snapshot now
- or a future later

Reference:
[`owned_value_projection_composes_through_fields_and_shapes`](../packages/optics/tests/optics.rs).

## 4. Async Uses The Same Path

```rust
let future_carrier = Optic::from_access(ResultFutureCarrier::new(
    Ok::<Option<User>, String>(Some(User { active: true })),
));

let fut = future_carrier
    .map_ok()
    .map_some()
    .map_ref_mut(user_active, user_active_mut)
    .future();
```

What this proves:

- async does not introduce a different projection vocabulary
- `future()` is just another terminal on the same path
- shape operations still compose before the await boundary

This is a better API because async is no longer a special graph of bespoke
projection types. The only explicit async decision is the terminal:

- build the path
- then ask for the future

Reference:
[`result_projection_supports_read_write_and_future`](../packages/optics/tests/optics.rs) and
[`nested_shape_projection_composes`](../packages/optics/tests/optics.rs).

## 5. Collections Still Return Optics

```rust
let todos = Optic::new(vec![
    Todo { done: false, title: "write code".into() },
    Todo { done: false, title: "ship".into() },
])
.each::<Todo>();

let second_title: String = todos
    .index(1)
    .map_ref_mut(todo_title, todo_title_mut)
    .value();

*todos.index(0).map_ref_mut(todo_done, todo_done_mut).write() = true;
```

What this proves:

- indexing a collection still gives back `Optic`
- child collection items use the same field projection API as roots
- there is no separate "item signal" abstraction

This is a strong sign that the abstraction is actually unified rather than just
"flattened at the root." The same wrapper survives:

- root value
- projected field
- indexed collection child
- field inside that child

Reference:
[`vec_collection_supports_lookup_and_mutation_helpers`](../packages/optics/tests/optics.rs).

## 6. Keyed Collections Follow The Same Rule

```rust
let users = Optic::new(HashMap::from([
    ("alice".to_string(), User { active: true }),
    ("bob".to_string(), User { active: false }),
]))
.each_hash_map::<String, User, std::collections::hash_map::RandomState>();

let alice = users.get("alice").unwrap();
*alice.clone().map_ref_mut(user_active, user_active_mut).write() = false;
```

What this proves:

- keyed lookup also returns `Optic`
- map children do not need a separate mutation vocabulary
- vector children and keyed children feel the same from the caller side

This is where "one clean abstraction" starts to mean something real. The API
does not fork when you move from:

- direct field access
- indexed collection access
- keyed collection access

Reference:
[`hash_map_projection_supports_lookup_iteration_and_mutation`](../packages/optics/tests/optics.rs) and
[`btree_map_projection_supports_lookup_iteration_and_mutation`](../packages/optics/tests/optics.rs).

## 7. Resources Reuse The Same Field Projection

```rust
let resource = Optic::from_access(Resource::resolved(User { active: true }));
let projected = resource.map_ref_mut(user_active, user_active_mut);

assert_eq!(*projected.read_opt().unwrap(), true);
let value: Option<bool> = projected.value();
let fut = projected.future();
```

What this proves:

- the field combinator is not tied to root `CopyValue` storage
- optional live reads, owned values, and futures all share the same path
- async-capable carriers do not need a second projection API

That is the cleanest argument for the current optics direction: the same lens
step applies to:

- plain roots
- nested optional shapes
- collection children
- resource-backed async carriers

Reference:
[`owned_value_projection_composes_through_fields_and_shapes`](../packages/optics/tests/optics.rs) and
[`resource_projection_composes_with_future_projection`](../packages/optics/tests/optics.rs).

## 8. Shape Operations Stay Separate From Access Operations

```rust
let nested = Optic::from_access(NestedOptionFutureCarrier(Some(Some(10))));
let fut = nested.flatten_some().future();
```

What this proves:

- `flatten_some()` is a normal path step
- `future()` is a normal terminal
- there is no bespoke "flatten future" abstraction

This is better API structure because it preserves the separation between:

- what path you want
- how you want to read it

Reference:
[`flatten_some_collapses_nested_option`](../packages/optics/tests/optics.rs) and
[`flatten_some_composes_separately_with_future_access`](../packages/optics/tests/optics.rs).

## Bottom Line

These examples do not prove that `dioxus-optics` is more integrated than the
projector branch. It is not.

They do prove that the public surface is cleaner in three specific ways:

1. There is one wrapper type: `Optic`.
2. Path-building is separate from access mode.
3. The same lens vocabulary survives across normal fields, nested shapes,
   collections, keyed collections, and async-capable carriers.

That is the sense in which this is a better API: it is easier to describe,
easier to extend locally, and less likely to split into parallel user-facing
abstractions as more carriers are added.
