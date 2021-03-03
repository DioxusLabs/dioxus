# March 3, 2021

Still TODO:
- Wire up Nodebuilder to track listeners as they are added.                     (easyish)
- Wire up attrs on nodes to track listeners properly
  - Could be done in the nodebuilder where the attrs are added automatically    (easyish)
  - Could just inject context into the diffing algorithm                        (hardish)
- Wire up component syntax                                                      (easy)
- Wire up component calling approach                                            (easyish)
- Wire up component diffing                                                     (hardish)


Approach:
- move listeners out of vnode diffing
- move listeners onto scope via nodebuilder
- instead of a listeners list, store a list of listeners and their IDs
  - this way means the diffing algorithm doesn't need to know that context
- This should fix our listener approach
- The only thing from here is child component


Thoughts:
- the macros should generate a static set of attrs into a [attr] array (faster, more predictable, no allocs)
- children should be generated as a static set if no parans are detected
  - More complex in the macro sized, unfortunately, not *too* hard
- Listeners should also be a static set (dynamic listeners don't make too much sense) 
  - use the builder syntax if you're doing something wild and need this granular control
- Tags should also be &'static str - no reason to generate them on the fly

Major milestones going forward:
- Scheduled updates
- String renderer (and methods for accessing vdom directly as a tree of nodes)
  - good existing work on this in some places
- Suspense
- Child support, nested diffing
- State management
- Tests tests tests
  
Done so far:
- websys 
- webview
- rsx! macro
- html! macro
- lifecycles
- scopes
- hooks
- context API
- bump


## Solutions from today's thinking session...

### To solve children:

- maintain a map of `ScopeIdx` to `Node` in the renderer
- Add new patch commands
    - traverse_to_known (idx)
        - Pop known component onto stack (super easy)
    - add_known (idx)
        - Save top of stack as root associated with idx
    - remove_known (idx)
        - Remove node on top of stack from known roots
    - ... Something like this
- Continue with BFS exploration of child components, DFS of VNodes
    - Easier to write, easier to reason about

### To solve listeners:

- Map listeners directly as attrs before diffing via a listenerhandle
- Evaluation of nodes is now stateful where we track listeners as they are added
