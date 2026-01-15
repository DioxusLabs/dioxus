# Dioxus Core - VirtualDOM Architecture

The `dioxus-core` crate is the heart of Dioxus, implementing the virtual DOM, component lifecycle, rendering pipeline, and reactive runtime.

## VirtualDom Structure

The `VirtualDom` struct orchestrates the entire reactive component tree:

```
VirtualDom
├── scopes: Slab<ScopeState>        // Arena allocator for all component scopes
├── dirty_scopes: BTreeSet<ScopeOrder>  // Scopes marked for re-render (height-ordered)
├── runtime: Rc<Runtime>            // Shared async runtime
├── resolved_scopes: Vec<ScopeId>   // Scopes resolved during suspense
└── rx: UnboundedReceiver<SchedulerMsg>  // Event channel from scheduler
```

### Key Methods

- `VirtualDom::new(app)` - Create new DOM with root component
- `rebuild()` / `rebuild_in_place()` - Full tree reconstruction
- `render_immediate()` - Render all dirty scopes without suspense blocking
- `wait_for_work()` - Async poll for futures and scheduler events
- `wait_for_suspense()` - Block until all suspended futures complete
- `mark_dirty(scope_id)` - Schedule scope for re-render
- `with_root_context<T>()` - Inject dependency into root scope

## Rendering Pipeline

### Initial Render Flow
1. `rebuild()` calls `run_scope(ScopeId::ROOT)`
2. Result wraps in `LastRenderedNode` and calls `create_scope()`
3. `create_scope()` recursively creates all child scopes and DOM elements
4. Mutations written to `WriteMutations` sink

### Update Render Flow
1. `wait_for_work()` polls scheduler and pending futures
2. `process_events()` consumes `SchedulerMsg::Immediate(scope_id)` messages
3. `render_immediate()` pops dirty scopes in height order (top-to-bottom)
4. For each scope: `run_and_diff_scope()` executes component, then `diff_scope()`
5. Diff generates mutations via `WriteMutations` trait

## Component System

### Component Definition
- `Component<P>` type alias: `fn(P) -> Element`
- `ComponentFunction<P, M>` trait for component functions
  - `fn_ptr(&self) -> usize` - Raw function pointer for identity
  - `rebuild(&self, props: P) -> Element` - Execute component

### Props System
- `Properties` trait for all props:
  - `type Builder` - For DSL prop construction
  - `builder()` - Create props builder
  - `memoize(&mut self, other: &Self) -> bool` - Check if props changed
  - `into_vcomponent()` - Convert to VComponent

### Type Erasure
`VProps<F,P,M>` wraps strongly-typed props, erased to `BoxedAnyProps` via `AnyProps` trait:
- `render(&self) -> Element`
- `memoize(&mut self, other: &dyn Any) -> bool`
- `props()` / `props_mut()` - Access as `dyn Any`
- `duplicate()` - Clone into new box

## Scope System

### ScopeId
Small unique identifier (usize index into Slab):
- `ScopeId::ROOT` (0) - Root wrapper scope
- `ScopeId::ROOT_SUSPENSE_BOUNDARY` (1) - Default suspense
- `ScopeId::ROOT_ERROR_BOUNDARY` (2) - Default error boundary
- `ScopeId::APP` (3) - User's root component

### ScopeState (Public API)
```
ScopeState
├── context_id: ScopeId
├── last_rendered_node: Option<LastRenderedNode>
├── props: BoxedAnyProps
└── reactive_context: ReactiveContext
```

### Scope (Internal State)
```
Scope
├── name: &'static str          // Component name for debugging
├── id: ScopeId
├── parent_id: Option<ScopeId>
├── height: u32                 // Distance from root
├── hooks: RefCell<Vec<Box<dyn Any>>>  // Hook state storage
├── hook_index: Cell<usize>     // Current hook being accessed
├── shared_contexts: RefCell<Vec<Box<dyn Any>>>
├── spawned_tasks: RefCell<FxHashSet<Task>>
├── before_render / after_render  // Callbacks
├── status: RefCell<ScopeStatus>  // Mounted/Unmounted
└── suspense_boundary: SuspenseLocation
```

### Context Propagation
- `provide_context<T: Clone + 'static>(context: T)` - Store in scope
- `consume_context<T>()` - Retrieve from this scope or any parent
- Lookup walks parent chain via `parent_id`

## Event System

### Event Structure
```rust
pub struct Event<T: ?Sized> {
    pub data: Rc<T>,
    pub(crate) metadata: Rc<RefCell<EventMetadata>>,
}

pub struct EventMetadata {
    pub propagates: bool,
    pub prevent_default: bool,
}
```

### Event Flow
1. `Runtime::handle_event()` looks up element by `ElementId`
2. Gets parent `ElementRef` from slab
3. If bubbles: `handle_bubbling_event()` walks parent chain
4. Collects listeners in path order, calls in reverse (parents first)
5. Breaks on `stop_propagation()`

## Diffing Algorithm

### Entry Point
`diff_scope()` gets old/new VNodes and calls `old.diff_node(new, dom, mutations)`

### VNode Diffing
- **Template Check**: Different templates → replace entire subtree
- **Identity Check**: Same pointer → skip
- **Attribute Diffing**: `diff_attributes()` for dynamic attrs
- **Dynamic Node Diffing**: Recursively diff each dynamic node

### Dynamic Node Cases
- Text → Text: `diff_vtext()` updates content
- Placeholder → Placeholder: No-op
- Fragment → Fragment: Recursively diff children
- Component → Component: Check render fn, then memoize props
- Type Change: Remove old, create new

### Keyed List Diffing
1. **Prefix Pass**: Match keys from start
2. **Suffix Pass**: Match keys from end
3. **Middle Pass**: Handle insertions/deletions/moves
4. Uses `FxHashMap` for old→new key matching

## Mutations System

### WriteMutations Trait
Interface between VirtualDOM and real DOM:
- `append_children(id, count)` - Add N nodes to element
- `assign_node_id(path, id)` - Mark element at template path
- `create_placeholder(id)` - Create marker node
- `create_text_node(value, id)` - Create text node
- `load_template(template, index, id)` - Clone from template cache
- `replace_node_with(id, count)` - Replace element
- `set_attribute(name, ns, value, id)` - Update attribute
- `create_event_listener(name, id)` - Register listener
- `remove_node(id)` - Delete element

### Mutation Enum (for testing/serialization)
```rust
pub enum Mutation {
    AppendChildren { id, m },
    CreatePlaceholder { id },
    CreateTextNode { value, id },
    LoadTemplate { index, id },
    ReplaceWith { id, m },
    SetAttribute { name, ns, value, id },
    // ... etc
}
```

## Scheduler

### Work Priority
1. **Dirty Scopes** (Highest): Re-render in height order
2. **Tasks**: Poll spawned futures
3. **Effects** (Lowest): Run after DOM changes

### ScopeOrder
Composite key: `(height: u32, id: ScopeId)` in `BTreeSet` ensures parent scopes run before children.

### Key Methods
- `queue_scope(order)` - Add to dirty_scopes
- `queue_task(task, order)` - Add to dirty_tasks
- `pop_work()` - Get next dirty scope or task
- `pop_effect()` - Get next pending effect

## Runtime

Manages async/scope/task coordination:
```
Runtime
├── scope_states: RefCell<Vec<Option<Scope>>>
├── scope_stack: RefCell<Vec<ScopeId>>
├── suspense_stack: RefCell<Vec<SuspenseLocation>>
├── tasks: RefCell<SlotMap<DefaultKey, Rc<LocalTask>>>
├── current_task: Cell<Option<Task>>
├── dirty_tasks: RefCell<BTreeSet<DirtyTasks>>
├── pending_effects: RefCell<BTreeSet<Effect>>
├── rendering: Cell<bool>
├── sender: UnboundedSender<SchedulerMsg>
├── elements: RefCell<Slab<Option<ElementRef>>>
└── mounts: RefCell<Slab<VNodeMount>>
```

## Tasks & Async

### Task Structure
- `id: TaskId` (slotmap key)
- `!Send + !Sync` marker

### Task Methods
- `Task::new(future)` - Spawn new task
- `cancel()` - Remove task
- `pause()` / `resume()` - Control polling
- `wake()` - Wake sleeping task

### LocalTask (Internal)
- Wraps `Pin<Box<dyn Future<Output = ()>>>`
- Custom waker that sends `SchedulerMsg::TaskNotified`
- Task dropped when owning scope drops

## Effects

Effects run AFTER mutations are applied to DOM:
1. Created via `use_effect()` in component
2. Queued in `Runtime::pending_effects`
3. After render, `finish_render()` sends `SchedulerMsg::EffectQueued`
4. `poll_tasks()` pops effects and calls `effect.run()`

## Suspense

### SuspenseContext
- Tracks suspended futures and placeholder nodes
- `suspended_tasks: RefCell<Vec<SuspendedFuture>>`
- `suspended_nodes: RefCell<Option<VNode>>`
- `frozen: Cell<bool>` for server-side locking

### Suspension Flow
1. Component calls `suspend()` → `Err(SuspendedFuture)`
2. `Element::Err(RenderError::Suspended)` propagates up
3. Nearest `SuspenseBoundary` catches it
4. Boundary renders placeholder
5. Future added to boundary's tasks
6. Boundary marked dirty when future completes

## VNodes & Templates

### VNode Structure
```rust
pub struct VNode {
    vnode: Rc<VNodeInner>,
    mount: Cell<MountId>,
}

pub struct VNodeInner {
    pub key: Option<String>,
    pub template: Template,
    pub dynamic_nodes: Box<[DynamicNode]>,
    pub dynamic_attrs: Box<[Box<[Attribute]>]>,
}
```

### Template (Static)
```rust
pub struct Template {
    pub roots: &'static [TemplateNode],
    pub node_paths: &'static [&'static [u8]],
    pub attr_paths: &'static [&'static [u8]],
}
```

### TemplateNode Variants
- `Element { tag, namespace, attrs, children }` - Static element
- `Text { text }` - Static text
- `Dynamic { id }` - Index into dynamic_nodes

### DynamicNode Variants
- `Component(VComponent)` - Child component
- `Text(VText)` - Text node
- `Placeholder(VPlaceholder)` - Suspense/empty marker
- `Fragment(Vec<VNode>)` - Multiple children

## Error Boundaries

### ErrorContext
- `error: Rc<RefCell<Option<CapturedError>>>`
- `subscribers: Subscribers` for listening components

### Error Flow
1. Component returns `Element::Err(error)`
2. Nearest `ErrorBoundary` catches
3. `ErrorContext` stores error
4. Boundary re-renders with `error_context.error()` available

## Root Wrapper

Default scope hierarchy:
```
ScopeId(0): RootScopeWrapper
└── ScopeId(1): SuspenseBoundary
    └── ScopeId(2): ErrorBoundary
        └── ScopeId(3): User's Root App
```

Provides default suspense and error handling for the entire tree.
