# This module includes all life-cycle related mechanics, including the virtual DOM, scopes, properties, and lifecycles.

---

The VirtualDom is designed as so:

VDOM contains:

- An arena of component scopes.
  - A scope contains
    - lifecycle data
    - hook data
- Event queue
  - An event

A VDOM is

- constructed from anything that implements "component"

A "Component" is anything (normally functions) that can be ran with a context to produce VNodes

- Must implement properties-builder trait which produces a properties builder

A Context

- Is a consumable struct
  - Made of references to properties
  - Holds a reference (lockable) to the underlying scope
  - Is partially thread-safe

# How to interact with the real dom?

## idea: use only u32

pros:

- allows for 4,294,967,295 nodes (enough)
- u32 is relatively small
- doesn't add type noise
- allows virtualdom to stay completely generic

cons:

- cost of querying individual nodes (about 7ns per node query for all sizes w/ nohasher)
- 2-3 ns query cost with slotmap
- old IDs need to be manually freed when subtrees are destroyed
  - can be collected as garbage after every render
- loss of ids between renders........................
  - each new render doesn't know which node the old one was connected to unless it is visited
  - When are nodes _not_ visited during diffing?
    - They are predetermined to be removed (a parent was probed)
    - something with keys?
    - I think all nodes must be visited between diffs
  -

## idea: leak raw nodes and then reclaim them on drop

# Fiber/Concurrency

Dioxus is designed to support partial rendering. Partial rendering means that not _every_ component will be rendered on every tick. If some components were diffed.

Any given component will only be rendered on a single thread, so data inside of components does not need to be send/sync.

To schedule a render outside of the main component, the `suspense` method is exposed. `Suspense` consumes a future (valid for `bump) lifetime
