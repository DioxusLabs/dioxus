# Signals and State Management

Dioxus state management is built on a layered architecture: generational-box for memory, signals for reactivity, and stores for nested data.

## Generational Box

### Core Concept
Provides `Copy` semantics for references through generation-based validation:
```
GenerationalBox<T, S>
├── raw: GenerationalPointer<S>
│   ├── storage: &'static S
│   └── location: GenerationalLocation
│       ├── generation: NonZeroU64
│       └── created_at: &'static Location (debug)
└── _marker: PhantomData<T>
```

### Memory Management

**Allocation:**
```rust
pub trait Storage<Data> {
    fn new(value: Data, caller: Location) -> GenerationalPointer<Self>;
    fn new_rc(value: Data, caller: Location) -> GenerationalPointer<Self>;
}
```

**Storage Variants:**
- `UnsyncStorage` - Single-threaded, uses `RefCell` (fast, no locking)
- `SyncStorage` - Multi-threaded, uses `RwLock` (thread-safe)

**StorageEntry:**
```
StorageEntry<Data>
├── generation: NonZeroU64
├── refcount: u32
└── data: Data  // Empty, Data, Rc, or Reference
```

### Generation Tracking
1. Caller provides pointer with generation X
2. Storage checks: `entry.generation == X`
3. If yes: data valid, proceed
4. If no: return `BorrowError::Dropped`

When dropped: generation incremented, data cleared, existing pointers invalidated.

### Owner Pattern
```rust
pub struct Owner<S> {
    owned: Vec<GenerationalPointer<S>>,
}

impl<S> Drop for Owner<S> {
    fn drop(&mut self) {
        for location in self.owned.drain(..) {
            location.recycle();  // Invalidates all pointers
        }
    }
}
```

## Signals

### Architecture
```
Signal<T, S>
└── inner: CopyValue<SignalData<T>, S>
    └── SignalData<T>
        ├── value: T
        └── subscribers: Arc<Mutex<HashSet<ReactiveContext>>>
```

### Signal Types

**Signal<T, S>** - Primary mutable reactive primitive:
- Implements `Readable` for subscribed reads
- Implements `Writable` for reactive updates
- `.read()` subscribes current scope
- `.peek()` reads without subscribing

**Memo<T>** - Derived reactive value:
- Wraps `Signal<T>` for value storage
- Contains `UpdateInformation` tracking dirty state
- Lazy evaluation: only recomputes when dependencies change
- Uses `PartialEq` to skip updates if unchanged

**CopyValue<T, S>** - Generic mutable wrapper:
- Lower-level than Signal
- No built-in reactivity
- Building block for Signal

**Global<T, R>** - Lazy singleton:
- Created once per app on first access
- Stored in `ScopeId::ROOT` context
- Uses `InitializeFromFunction` trait

**ReadSignal<T>** / **WriteSignal<T>** - Type-erased boxed signals:
- Store `Box<dyn DynReadable>`
- Flexible APIs accepting any readable type

**MappedSignal<O, V, F>** - Derived readonly:
- Maps inner signal through function F
- `signal.map(|x| &x.field)`

### Reactivity System

**Automatic Subscription:**
```
signal.read()
  → try_read_unchecked()
  → Check ReactiveContext::current()
  → If exists: reactive_context.subscribe(signal.subscribers)
  → Later writes call mark_dirty() on all subscribers
```

**Update Propagation:**
```
signal.write()
  → WriteLock created
  → User modifies value
  → WriteLock dropped → SignalSubscriberDrop::drop()
  → signal.update_subscribers():
    → Get subscriber snapshot (brief lock)
    → Call mark_dirty() on each
    → Re-extend subscriber list
```

### Global Signals

**GlobalLazyContext:**
```
GlobalLazyContext
└── map: Rc<RefCell<HashMap<GlobalKey, Box<dyn Any>>>>
```

**Resolution:**
1. Static const defines `GlobalSignal<T>`
2. First access calls `resolve()`:
   - Check HashMap by key
   - If found: return clone
   - If not: run constructor in ROOT scope
   - Store result
3. Subsequent accesses return same instance

**Key Types:**
- `GlobalKey::File { file, line, column, index }`
- `GlobalKey::Raw(&'static str)`

## Hooks

### Core Hook Pattern
```rust
#[track_caller]
pub fn use_hook<T>(f: impl FnOnce() -> T) -> T {
    let component_id = current_scope_id();
    let mut hooks = get_hooks(component_id);

    if hooks.len() <= hook_index {
        hooks.push(Box::new(f()));  // First render
    }
    hooks[hook_index].clone()  // Return existing
}
```

### Hook Types

**use_signal<T>() -> Signal<T>**
- Creates local signal owned by component
- State persists across renders

**use_memo<R>() -> Memo<R>**
- Creates memoized computation
- Reruns when read signals change

**use_effect(callback)**
- Runs side effects when dependencies change
- Creates `ReactiveContext` to track reads
- Queues effect rerun on dependency notification

**use_resource<T, F>(future_fn) -> Resource<T>**
- Async state management
- Watches dependencies, reruns future when they change
- Returns `Resource<T>` with value, state, task

**use_callback<I, O>() -> Callback<I, O>**
- Memoized callback
- Prevents unnecessary closures

**use_coroutine(init)**
- Long-lived task
- Spawned once at component creation

**use_context<T>() -> T**
- Retrieves ancestor context

### Hook Rules
1. Must call same hooks every render
2. Must call in same order
3. Order determines hook identity
4. Breaking rules causes panic or bugs

## Stores

### Purpose
Signals work for scalar state. Stores provide:
- Granular reactivity per field
- Lazy signal creation
- Ergonomic field access

### Store Architecture
```
Store<T, Lens>
└── selector: SelectorScope<Lens>
    ├── subscriptions: StoreSubscriptions
    ├── path: TinyVec<u16>
    └── value: Lens
```

### Subscription Tree
```
StoreSubscriptions
└── inner: CopyValue<StoreSubscriptionsInner>
    └── root: SelectorNode
        ├── subscribers: HashSet<ReactiveContext>
        └── root: HashMap<PathKey, SelectorNode>
            └── [0] → SelectorNode
            └── [1] → SelectorNode
```

### Path Tracking
```rust
let store = Store::new(vec![a, b, c]);
let item_1 = store[1];    // path = [1]
let field = item_1.name;  // path = [1, field_hash]

// Writing store[1].name only marks dirty:
// - subscribers at path [1]
// - subscribers at path [1, field_hash]
// - NOT path [0] or [2]
```

### Store Macro
```rust
#[derive(Store)]
struct TodoItem {
    checked: bool,
    contents: String,
}

// Generates:
pub trait TodoItemStoreExt<__Lens> {
    fn checked(self) -> Store<bool, __Lens::MappedSignal>;
    fn contents(self) -> Store<String, __Lens::MappedSignal>;
    fn transpose(self) -> TodoItemStoreTransposed;
}

pub struct TodoItemStoreTransposed {
    pub checked: Store<bool>,
    pub contents: Store<String>,
}
```

### Enum Support
```rust
#[derive(Store)]
enum Status {
    Loading,
    Ready(String),
    Error(String),
}

// Generates:
fn is_loading(self) -> bool;
fn ready(self) -> Option<Store<String, ...>>;
fn transpose(self) -> StatusStoreTransposed;
```

## Key Traits

### Readable
```rust
pub trait Readable {
    type Target;
    type Storage;
    fn try_read_unchecked(&self) -> Result<ReadableRef<T>>;
    fn try_peek_unchecked(&self) -> Result<ReadableRef<T>>;
    fn subscribers(&self) -> Subscribers;
}
```

### Writable
```rust
pub trait Writable: Readable {
    type WriteMetadata;
    fn try_write_unchecked(&self) -> Result<WritableRef<T>>;
}
```

### Storage (AnyStorage)
```rust
pub trait AnyStorage {
    type Ref<'a, T>: Deref<Target = T>;
    type Mut<'a, T>: DerefMut<Target = T>;
    fn map<T, U>(ref_: Ref<T>) -> Ref<U>;
    fn map_mut<T, U>(mut_ref: Mut<T>) -> Mut<U>;
}
```

## Comparison

| Feature | Signal | Memo | Store |
|---------|--------|------|-------|
| Mutability | Mutable | Read-only | Mutable |
| Dependencies | None | Tracks reads | Implicit via paths |
| Granularity | Single value | Computation | Per-field |
| When to use | Direct state | Derived values | Nested structures |
| Subscriptions | Simple HashSet | Via context | Tree structure |
| Performance | O(1) update | O(deps) recompute | O(path) update |

## Memory Model

1. **Heap**: Values in storage singletons per-scope
2. **Stack**: Only pointers (GenerationalBox) in components
3. **Cleanup**: Owner drops when scope dies
4. **Validity**: Generation checking on each access
5. **Sharing**: Arc+Mutex for subscribers, Rc+RefCell for ownership
