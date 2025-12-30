# Hot-Reload and Hot-Patching Architecture

Dioxus supports two complementary hot-reload systems: RSX template hot-reload for UI changes, and Subsecond hot-patching for Rust code changes.

## Subsecond Hot-Patching

### Jump Table Architecture

Subsecond implements hot-patching through **jump table indirection**:

1. All hot-reloadable functions called through `subsecond::call()` or `HotFn::current()`
2. Runtime looks up function pointer in global `APP_JUMP_TABLE`
3. Jump table points to latest compiled version
4. When patch applied, only jump table is updated
5. Original executable remains untouched

**Advantage**: Decouples running binary from patched code. Safe memory model.

### Patch Application Flow

```
1. ThinLink compiles only modified functions → patch dylib
2. Patch sent via devtools WebSocket
3. subsecond::apply_patch() loads via libloading::Library
4. Base address calculated using main as anchor
5. Jump table updated with old→new address mappings
```

### ASLR Handling

Operating systems randomize memory addresses (ASLR), so compiled addresses don't match runtime.

**Solution**:
1. Running app captures real address of `main` via `dlsym()`/`GetProcAddress()`
2. ASLR reference sent to devserver
3. Devserver computes offset: `old_offset = aslr_reference - table.aslr_reference`
4. All jump table addresses adjusted by offset
5. Patch library base address calculated similarly
6. Final: `(old_address + old_offset) → (new_address + new_offset)`

### Thread-Local Storage (TLS)

TLS presents challenges for hot-patching:

- **Globals and statics**: Generally supported
- **Thread-locals in tip crate**: Reset to initial value on new patches
- **Thread-locals in dependencies**: Work correctly (not re-patched)

**Why**: New thread-local variables exist in patch library's TLS segment, separate from main executable's TLS. Subsecond doesn't rebind TLS accesses.

### Limitations

1. **Struct changes**: Not supported - size/alignment changes cause crashes
2. **Pointer versioning**: Not implemented - all function pointers considered "new"
3. **Workspace support**: Only tip crate patches, library crates ignored
4. **Static initializers**: Changes not observed
5. **Global destructors**: Newly added globals have destructors that never run

### Platform Support

- **Desktop**: Linux, macOS, Windows (x86_64, aarch64)
- **Mobile**: Android (arm64-v8a, armeabi-v7a), iOS Simulator
- **Web**: WASM32 (limited module reloading)
- **Not supported**: iOS device (code-signing)

## RSX Hot Reload

### Hot Literal System

RSX hot reload is **orthogonal** to subsecond. While subsecond reloads Rust functions, RSX hot reload handles template literal values:

**Hot-reloadable**:
- Formatted segments: `"{variable}"`
- Literal component properties: `Component { value: 123 }`
- Dynamic text nodes: `"{expression}"`

**Not hot-reloadable**:
- Rust code changes
- Component structure changes
- Control flow changes

### Template Diffing Algorithm

**Conservative approach**: If Rust code changes, not hot-reloadable.

```
1. Parse old and new files
2. Extract all rsx! macro invocations
3. Replace all rsx! bodies with empty rsx! {}
4. Remove doc comments
5. Compare modified files
6. If identical → Rust unchanged → proceed with template diff
7. If different → requires full rebuild
```

### Dynamic Pool System

Three pools of reusable items from last build:
1. **Dynamic text segments**: `"{class}"`, `"{id}"`
2. **Dynamic nodes**: Components, for loops, if chains
3. **Dynamic attributes**: Spread operators, dynamic values

Each item tracks usage with `Cell<bool>` for waste scoring.

### Greedy Matching Algorithm

For each new dynamic node:
1. Find all candidates from last build with compatible structure
2. Attempt to create hot-reloaded template using candidate's pool
3. Score: Count unused dynamic items remaining
4. Select candidate with least unused items
5. Greedy approach optimal because it maximizes reuse

### Change Detection

**NOT possible**:
- Number of `rsx!` macros changes
- Rust expressions change
- Component structure changes
- Control flow conditions change

**IS possible**:
- Component children content
- Reordering attributes
- Adding dynamic text segments from pool
- Changing literal values
- Shuffling template structure

## Devtools Protocol

### WebSocket Communication

Bidirectional WebSocket between app and devserver.

**Connection**:
- URL: `ws://localhost:3000/_dioxus`
- Query params: `build_id`, `pid`, `aslr_reference`
- Persistent during development

### Message Types

```rust
pub enum DevserverMsg {
    HotReload(HotReloadMsg),  // Templates + optional jump table
    HotPatchStart,            // Binary patching starting
    FullReloadStart,          // Rebuilding entire app
    FullReloadFailed,         // Build failed
    FullReloadCommand,        // Full page reload needed
    Shutdown,                 // Devserver shutting down
}
```

### HotReloadMsg Structure

```rust
pub struct HotReloadMsg {
    pub templates: Vec<HotReloadTemplateWithLocation>,
    pub assets: Vec<PathBuf>,
    pub ms_elapsed: u64,
    pub jump_table: Option<JumpTable>,
    pub for_build_id: Option<u64>,
    pub for_pid: Option<u32>,
}
```

### Message Processing

1. `connect(callback)` - Generic connection
2. `connect_subsecond()` - Subsecond-specific with jump tables
3. `apply_changes(dom, msg)`:
   - Updates signal-based template cache
   - Applies jump table if PID matches
   - Marks all components dirty

### WASM-Specific Handling

- Connection at `ws://host/_dioxus?build_id={build_id}`
- Supports playground mode (iframe via postMessage)
- Console logging sent to devserver
- Page reload on `FullReloadCommand`
- Asset cache invalidation via `dx_force_reload` query params

## Integration Flow

### Subsecond in Dioxus Apps

```rust
fn main() {
    dioxus::launch(app);
    // Devtools automatically connects during init
}
```

For non-Dioxus apps:
```rust
fn main() {
    dioxus_devtools::connect_subsecond();

    loop {
        dioxus_devtools::subsecond::call(|| {
            handle_request()
        });
    }
}
```

### Async Integration

```rust
#[tokio::main]
async fn main() {
    dioxus_devtools::serve_subsecond_with_args(
        state,
        |state| async {
            app_main(state).await
        }
    ).await;
}
```

**Process**:
1. Catches patch message
2. Applies jump table
3. Drops current future
4. Creates new future with hot function
5. Continues execution

## Build System Integration

### Fat vs Thin Builds

**Fat Build** (initial):
- Full build with all symbols
- Required for initial symbol table
- Creates `HotpatchModuleCache`

**Thin Build** (patches):
- Compiles only modified functions
- Uses cached dependency symbols
- Produces minimal patch dylib

### JumpTable Structure

```rust
pub struct JumpTable {
    pub lib: PathBuf,              // Path to patch dylib
    pub map: HashMap<u64, u64>,    // old_addr → new_addr
    pub aslr_reference: u64,
    pub new_base_address: u64,
    pub ifunc_count: u32,
}
```

### Platform-Specific Patching

- `create_windows_jump_table()` - x86/x64 jump stubs
- `create_native_jump_table()` - macOS/Linux function overrides
- `create_wasm_jump_table()` - WASM indirect call table updates

## Key Design Decisions

1. **Jump table indirection**: Safe, no memory corruption
2. **Conservative RSX diffing**: Rust changes trigger rebuild
3. **Greedy pool matching**: Optimal template reuse
4. **WebSocket protocol**: Real-time bidirectional updates
5. **PID filtering**: Correct process receives patches
6. **Dual system**: RSX for templates, Subsecond for logic

## Future Considerations

### Workspace Support
- Dependency graph analysis for affected crates
- Incremental library crate compilation
- Cross-crate function pointer resolution

### Remote Hot-Reloading
- SCP transport for bandwidth efficiency
- Binary diff for minimal transfers
- Cryptographic verification

### CLI Tunnels
- SSH/TCP protocol wrapping
- Connection persistence
- Latency-tolerant queueing
