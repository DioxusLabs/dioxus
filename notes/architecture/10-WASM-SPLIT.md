# WASM Code Splitting Architecture

The WASM-Split system enables code splitting for large WebAssembly applications, allowing developers to lazily load feature chunks on demand.

## Overview

### Problem Solved

Large WASM binaries can impact initial load times. WASM-Split produces:
- **main.wasm** - Core application
- **module_N_*.wasm** - Lazy-loaded feature chunks
- **chunk_N_*.wasm** - Shared code across modules
- **__wasm_split.js** - Runtime loader

### Output Structure

```
dist/
├── main.wasm           # Core application
├── module_1_*.wasm     # Feature chunk 1
├── module_2_*.wasm     # Feature chunk 2
├── chunk_1_*.wasm      # Shared code
└── __wasm_split.js     # JavaScript loader
```

## Macro Usage

### `#[wasm_split]` Attribute Macro

Marks async functions as split boundaries:

```rust
#[wasm_split(my_feature)]
async fn load_feature() -> i32 {
    // This code will be in module_my_feature.wasm
    expensive_computation()
}
```

**Generated Names** (pattern: `__wasm_split_00<module>00_<type>_<hash>_<function>`):
- `__wasm_split_00my_feature00_import_<hash>_load_feature` - FFI import
- `__wasm_split_00my_feature00_export_<hash>_load_feature` - FFI export

**Transformation**:
1. Generates `extern "C"` export with implementation
2. Creates import declaration from `__wasm_split.js`
3. Creates thread-local `LazySplitLoader` with load function
4. Awaits loader before calling imported version

### `#[lazy_loader]` Macro

For libraries creating lazy-loadable wrappers:

```rust
#[lazy_loader(extern "auto")]
fn my_lazy_fn(x: i32) -> i32;
```

Returns `LazyLoader<Args, Ret>` with:
- `.load().await` - Async module loading
- `.call(args)` - Sync invocation after loading

**"auto" ABI**: Automatically combines all modules into one.

## Build Integration

### Input Requirements

CLI takes two binary inputs:
1. **original.wasm** - Pre-wasm-bindgen (with `--emit-relocs`)
2. **bindgened.wasm** - Post-wasm-bindgen

Compilation requirements:
- `--emit-relocs` - Relocation information
- Debug symbols - Function name resolution
- LTO - Symbol consistency

### Processing Pipeline

**Phase 1: Discovery and Graph Building**

```
1. Scan imports/exports for __wasm_split_00<module>00_* pattern
2. Parse relocations from original.wasm
3. Build call graph from:
   - CODE section relocations
   - DATA section relocations
   - Direct function calls via IR walking
4. Build parent graph (inverse for reachability)
5. Compute reachability for each split point
```

**Phase 2: Chunk Identification**

- Identify functions used by multiple modules
- Extract into shared chunks
- Build chunk dependency graph

**Phase 3: Module Emission**

Three output types (parallel via rayon):

**A. Main Module**:
- Remove split point exports
- Replace element segments with dummy functions
- Create indirect function table
- Convert split imports to stub functions
- Re-export memories, globals, tables
- Run garbage collection

**B. Split Modules** (per split point):
- Identify unique vs shared symbols
- Convert chunk functions to imports
- Clear and reconstruct data segments
- Create element segment initializers
- Export main entry function
- Run GC

**C. Chunk Modules** (shared code):
- Similar to split modules
- No main export function
- Contains commonly-used functions

## Walrus Operations

The system uses **walrus** for WASM binary manipulation:

### Function Table Management

```
1. Ensure funcref table exists (__indirect_function_table)
2. Expand table for split modules + shared functions
3. Create passive element segments for initialization
```

### Stub Function Creation

Stubs perform indirect calls:
```
1. Push function arguments onto stack
2. Push table index (pointing to real function)
3. Call via CallIndirect with table
```

### Import/Export Manipulation

- Convert imports to local stub functions
- Re-export shared resources
- Use `__wasm_split` namespace for shared imports

### Graph Analysis

**Node Types**:
- `Node::Function(FunctionId)`
- `Node::DataSymbol(usize)`

**Graphs**:
- **Call Graph**: Function → callees
- **Parent Graph**: Inverse (what calls them)
- **Reachability Graph**: Per split point, all reachable symbols
- **Main Graph**: All reachable from main exports

## Runtime Loading

### LazyLoader Structure

```rust
pub struct LazyLoader<Args, Ret> {
    // Generic loader for function (Args) -> Ret
}

impl LazyLoader {
    pub async fn load(&self) -> bool;  // Load module
    pub fn call(&self, args: Args) -> Result<Ret, SplitLoaderError>;
}
```

### LazySplitLoader State Machine

Three states:
1. **Deferred** - Not initiated, holds load function
2. **Pending** - Load initiated, waiting for callback
3. **Completed** - Loaded (with success boolean)

**Async Interface**:
- `SplitLoaderFuture` implements `Future<Output = bool>`
- `poll()` handles state transitions
- Uses `Waker` to resume on callback

### JavaScript Runtime (`__wasm_split.js`)

**`makeLoad()` Function**:
```javascript
async function(callbackIndex, callbackData) {
    // 1. Await chunk dependencies
    // 2. Check if already loaded
    // 3. Fetch module binary
    // 4. Call initSync from main.wasm
    // 5. Construct import object:
    //    - Memory from main module
    //    - Indirect function table
    //    - Stack pointers and TLS base
    //    - Main module exports as imports
    //    - Fused imports from other modules
    // 6. WebAssembly.instantiateStreaming()
    // 7. Add exports to fusedImports
    // 8. Invoke callback with table index
}
```

**Callback Mechanism**:
```javascript
__indirect_function_table.get(callbackIndex)(callbackData, true)
```
Wakes up Rust Future waiting in loader.

## Integration Flow

```
Compile Phase: Rust code with #[wasm_split]
         ↓
Macro Expansion: FFI functions + LazyLoader
         ↓
Build Phase: wasm-split CLI processes binaries
         ↓
Parse Relocations → Build Call Graph → Identify Split Points
         ↓
Compute Reachability → Parallel Emission
         ↓
Output: main.wasm + module_*.wasm + chunk_*.wasm + __wasm_split.js
         ↓
Runtime: JavaScript fetch → Instantiate → Callback → Future wakes
         ↓
Lazy functions available synchronously
```

## Key Transformations

### Main Module

1. Remove split point exports
2. Replace element segments with dummies
3. Create indirect function table
4. Convert imports to stubs (call indirect table)
5. Re-export shared resources
6. Remove relocation/linking sections
7. GC unused code

### Split Module

1. Partition functions (unique vs shared)
2. Convert chunk functions to imports
3. Clear data segments, reconstruct needed data
4. Create element segment initializers
5. Create FFI imports from main
6. Export entry function
7. Remove start function
8. GC unreachable symbols

## Usage Example

```rust
// Define lazy-loadable feature
#[wasm_split(admin_panel)]
async fn load_admin_panel() -> AdminPanel {
    AdminPanel::new()
}

// Use in application
async fn handle_route(route: Route) {
    match route {
        Route::Admin => {
            let panel = load_admin_panel().await;
            panel.render();
        }
        _ => // ...
    }
}
```

## Design Considerations

### Split Point Granularity

- One `#[wasm_split]` per feature chunk
- Too fine-grained = many small fetches
- Too coarse = large chunks

### Shared Code Optimization

- Common functions extracted to chunks
- Prevents duplication across modules
- Chunks loaded before dependent modules

### Memory Sharing

- Single memory instance shared across all modules
- Indirect function table shared for cross-module calls
- TLS base pointer coordinated

### Error Handling

```rust
pub enum SplitLoaderError {
    NotLoaded,
    LoadFailed,
}
```

Loader returns `Result` for robust error handling.

## Limitations

1. **Async boundaries**: Split points must be async functions
2. **Call graph static**: Determined at build time
3. **No dynamic imports**: All split points known at compile time
4. **WASM-specific**: Only works with wasm32 target

## Future Considerations

- Route-based automatic splitting
- Size-based chunk optimization
- Prefetching hints
- Integration with service workers for caching
