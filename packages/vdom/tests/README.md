# Testing of Dioxus core


Diffing
- [x] create elements
- [x] create text
- [x] create fragments
- [x] create empty fragments (placeholders)
- [x] diff elements
- [x] diff from element/text to fragment
- [x] diff from element/text to empty fragment
- [x] diff to element with children works too
- [x] replace with works forward
- [x] replace with works backward
- [x] un-keyed diffing
- [x] keyed diffing
- [x] keyed diffing out of order
- [x] keyed diffing with prefix/suffix
- [x] suspended nodes work

Lifecycle
- [] Components mount properly
- [] Components create new child components
- [] Replaced components unmount old components and mount new
- [] Post-render effects are called

Shared Context
- [] Shared context propagates downwards
- [] unwrapping shared context if it doesn't exist works too

Suspense
- [] use_suspense generates suspended nodes


Hooks
- [] Drop order is maintained
- [] Shared hook state is okay
- [] use_hook works
- [] use_ref works
- [] use_noderef works
- [] use_provide_state
- [] use_consume_state


VirtualDOM API
- [] work
- [] rebuild_to_vec
- [] change props
