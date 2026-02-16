# Dioxus CLI Architecture

The `dx` CLI is a comprehensive build tool for Dioxus applications, supporting development, building, bundling, and cross-platform deployment.

## Command Structure

Entry point: `main.rs` with async tokio runtime and `TraceController` for error handling.

### Commands Enum
- `Serve` - Watch and serve with hot-reloading
- `Build` - Full production build
- `Bundle` - Package for distribution
- `Run` - Run without hot-reload
- `Create` / `Init` - Project scaffolding
- `Check` - Lint/validate RSX
- `Translate` - Convert HTML/JSX to RSX
- `Autoformat` - Format RSX code
- `Config` - Manage Dioxus.toml
- `Doctor` - Diagnose environment
- `Components` - Component registry
- `SelfUpdate` - Update CLI
- `Tools` - Internal tools (build-assets, hotpatch)

## Platform Support

### Platform Enum
- `Web` - wasm32-unknown-unknown
- `MacOS` / `Windows` / `Linux` - Desktop webview
- `Ios` / `Android` - Mobile webview
- `Server` - SSR/fullstack server
- `Liveview` - WebSocket streaming
- `Unknown` - Auto-select

### CommandWithPlatformOverrides
Allows separate client/server args for fullstack:
```bash
dx serve @client --features web @server --features server
```

Decomposes into client BuildArgs, server BuildArgs, and shared args.

## Build System

### BuildRequest
The "plan" for a single build:
```
BuildRequest
├── workspace: Workspace metadata
├── config: DioxusConfig
├── package: Package details
├── features: Vec<String>
├── target: Triple
├── bundle: BundleFormat
├── profile: Profile
├── rustflags: Vec<String>
└── custom_linker: Option<PathBuf>
```

Responsible for:
- Writing build artifacts
- Processing assets
- Writing platform-specific metadata (Info.plist, AndroidManifest.xml)
- Platform-specific linking (Android uses dx as opaque linker, hotpatching uses custom linker)

### BuildMode Enum
- `Base { run: bool }` - Normal cargo rustc build
- `Fat` - Full build with all symbols for hot-patching
- `Thin { rustc_args, changed_files, aslr_reference, cache }` - Fast rebuild

### BuildContext
Runtime configuration with progress channel:
- `ProgressTx` for status updates
- `BuildId` enum: PRIMARY (client), SECONDARY (server)
- Helper methods: `compiling()`, `linking()`, `bindgen()`, etc.

### AppBuilder
State machine managing ongoing builds:
- Build task handle and artifacts
- App child process (stdout/stderr)
- Hot-patching state (patches, module cache, ASLR offset)
- Build metadata: stage, compiled crates, timing

## Dev Server (Serve)

### AppServer (runner.rs)
Primary orchestration:
```
AppServer
├── workspace: Workspace metadata
├── client / server: AppBuilder instances
├── watcher: File system watcher
├── file_map: Cached RSX files for hot-reload diffing
├── devserver: WebServer
└── flags: use_hotpatch_engine, automatic_rebuilds, hot_reload, etc.
```

### WebServer (server.rs)
HTTP + WebSocket server providing:
- **Hot-reload socket** - RSX and asset updates
- **Build-status socket** - Compilation progress
- **HTTP server** - Static assets (public folder, WASM)
- **Proxy support** - Fullstack backend
- **DevTools** - Connected clients with build_id, ASLR reference, PID

### Output (output.rs)
Terminal UI with ratatui:
- Build progress with crate-by-crate counts
- Asset copying progress
- Colored output, throbber animations
- Keyboard input (r=rebuild, p=pause, v=verbose)
- 100ms tick-based rendering

### File Change Handling
```
File modified → detect type
├── Rust file → diff RSX using dioxus_rsx_hotreload::diff_rsx()
│   ├── Extract changed templates
│   ├── Build HotReloadMsg
│   └── Send via WebSocket
├── Asset file → copy to bundle, signal reload
└── If can't hot-reload → queue full rebuild
```

### serve_all() Main Loop
Uses `tokio::select!` to multiplex:
- Builder updates (compilation, process output)
- WebServer events (connections, messages)
- Output (TUI updates, input)
- TraceController (panic handling)
- File watcher (filesystem changes)

## Asset Processing

### Pipeline (build/assets.rs)
1. **Manganis Integration** - Extract `__ASSETS__` symbols from binary
2. **Asset Hashing** - Compute hashes for cache-busting
3. **Two Format Support** - Legacy (0.7.0-0.7.1) and New (0.7.2+) serialization
4. **Platform Paths** - Assets relative to executable; Android uses "asset root"

### Processing by Type
- **Images**: Resize, convert format (Avif, WebP), optimize
- **CSS/JS**: Minify, hash-suffix naming
- **Folders**: Recursive copy
- **Unknown**: Direct copy with optional hash

## Bundling

### Bundle System
Platform-specific packaging:
- **Web/Server**: No special bundling, outputs to web/ directory
- **Desktop**: Uses tauri-bundler (dmg, msi, appimage)
- **iOS**: App bundle format with codesigning warnings
- **Android**: Runs `gradle bundleRelease` for AAB

### BundleConfig
```
BundleConfig
├── identifier: String
├── publisher: String
├── icon: Vec<PathBuf>
├── resources: Vec<PathBuf>
├── deb / macos / windows / android: Platform-specific
```

## Hot-Patching

### HotpatchModuleCache
Caches compiled dependency symbols:
- Symbol table with function/data info
- Pre-computed for Fat builds
- Reused in Thin builds for fast linking

### Patch Creation Flow
1. Compile app with Fat mode (all symbols)
2. On change: compile Thin (small, fast)
3. Analyze binary diff → create JumpTable
4. Apply patches to running executable
5. If patch fails → fall back to full rebuild

### Platform Implementations
- `create_windows_jump_table()` - x86/x64 jump stubs
- `create_native_jump_table()` - macOS/Linux function overrides
- `create_wasm_jump_table()` - WASM indirect call table updates

## Configuration

### DioxusConfig (Dioxus.toml)
```toml
[application]
asset_dir = "assets"
out_dir = "dist"
tailwind_input/output = "..."
ios_info_plist = "Info.plist"
android_manifest = "AndroidManifest.xml"

[web]
app.title = "My App"
https.enabled = true
pre_compress = false

[bundle]
identifier = "com.example.app"
publisher = "Company"
```

### Resolution Order
1. Default values
2. Dioxus.toml in project root
3. CLI argument overrides
4. Environment variables

## Workspace

### Workspace Struct (Singleton)
```
Workspace
├── krates: Krates (cargo metadata, dependency graph)
├── settings: CliSettings (user settings ~/.config/dioxus/)
├── sysroot: Rust target sysroot
├── rustc_version: Version
├── wasm_opt: Optional path
├── cargo_toml: Parsed Cargo.toml
└── android_tools: Optional NDK/SDK
```

Lazy-loaded via `Workspace::current()` with tokio Mutex.

## Extension Points

### Adding New Bundlers
1. New variant in `BundleFormat` enum
2. New variant in `Renderer` enum
3. New config struct (e.g., `MyPlatformSettings`)
4. Implementation in `Bundle::bundle()`
5. Asset directory structure definition
6. Linking strategy if needed

### Adding New Commands
1. New variant in `Commands` enum
2. New command module in `cli/`
3. Register in `TraceController::main()` match
4. Optional `StructuredOutput` variant for results

### Customizing Build Hooks
- Pre/post-build scripts in Dioxus.toml
- Custom linker integration
- Asset transformation pipeline

## Key Patterns

### Channel-Based Progress
```
Build Task → BuilderUpdate Channel → AppBuilder → ServeUpdate → Output TUI
```
Decouples build process from UI for real-time updates.

### Lazy Workspace Loading
Single workspace load with tokio Mutex, shows spinner if cargo-metadata takes >1 second.

### Renderer/Bundle Inference
1. Explicit flags: `--web`, `--webview`, `--native`
2. Platform aliases: `--ios`, `--android`, `--desktop`
3. Feature detection from Cargo.toml
4. Default: Infer from target triple
