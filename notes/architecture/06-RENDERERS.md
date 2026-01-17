# Dioxus Renderers Architecture

Dioxus supports multiple rendering backends through a common trait-based abstraction. Each renderer implements the same interfaces to enable cross-platform component reuse.

## Core Abstraction

### WriteMutations Trait
The bridge between VirtualDOM and real DOM implementations:
- `append_children(id, count)` - Add N nodes to element
- `assign_node_id(path, id)` - Mark element at template path
- `create_placeholder(id)` - Create marker node
- `create_text_node(value, id)` - Create text node
- `load_template(template, index, id)` - Clone from template cache
- `replace_node_with(id, count)` - Replace element
- `set_attribute(name, ns, value, id)` - Update attribute
- `create_event_listener(name, id)` - Register listener
- `remove_node(id)` - Delete element

### HtmlEventConverter Trait
Maps platform-specific events to standardized Dioxus types:
- Each renderer provides its own implementation
- Converts `PlatformEventData` (renderer-specific) to typed event data
- Enables event polymorphism across platforms

### Event Flow Pattern
```
Platform Event → Captured by renderer
    → Converted via HtmlEventConverter
    → runtime.handle_event(name, event, element_id)
    → VirtualDOM handlers invoked
    → State changes → Re-render
    → WriteMutations applied
```

## Web Renderer (dioxus-web)

### WebsysDom Structure
```
WebsysDom
├── interpreter: Sledgehammer JS interpreter
├── document: web_sys::Document reference
├── root: Root DOM node
├── templates: HashMap<Template, u16>
├── runtime: Rc<Runtime>
└── (hydration): skip_mutations, suspense_hydration_ids
```

### Mutation Implementation
- Directly delegates to JavaScript interpreter via wasm-bindgen
- Templates serialized once, stored in JS, instantiated by reference
- Sledgehammer interpreter maintains stack of nodes being constructed
- Binary protocol enables efficient mutation batching

### Event Handling
- Single delegated listener on root element
- Event.target walked up DOM tree for `data-dioxus-id` attribute
- `WebEventConverter` converts web_sys events to Dioxus types
- Supports all standard DOM events (mouse, keyboard, touch, etc.)

### Hydration System
1. SSR server renders HTML with `dio_el` data attributes
2. Client receives base64-encoded hydration context
3. VirtualDOM rebuilt with `skip_mutations = true`
4. Client traverses pre-rendered DOM assigning element IDs
5. Streaming hydration for suspense boundaries via `rehydrate_streaming()`

### Launch Flow
```
launch(root_component, contexts, config)
  → Create VirtualDom
  → Create WebsysDom wrapper
  → If hydrate: Deserialize data, rebuild with skip_mutations
  → Otherwise: vdom.rebuild(&mut websys_dom)
  → Main loop: wait_for_work() → render_immediate() → flush_edits()
```

## Desktop Renderer (dioxus-desktop)

### Wry/Tao Integration
Uses Wry webview library with Tao window management.

### App Structure
```
App
├── unmounted_dom: Cell<Option<VirtualDom>>
├── webviews: HashMap<WindowId, WebviewInstance>
├── shared: Rc<SharedContext>
│   ├── event_handlers: WindowEventHandlers
│   ├── pending_webviews: Vec<PendingWebview>
│   ├── shortcut_manager: ShortcutRegistry
│   └── websocket: EditWebsocket
└── control_flow: ControlFlow
```

### WebviewEdits
Implements WriteMutations for wry-based rendering:
- Delegates to `WryQueue` managing mutation batch
- WebSocket server on random port for mutation transmission
- Binary protocol via Sledgehammer interpreter

### IPC (Interprocess Communication)
```
Browser event → JavaScript → window.postMessage()
    → Wry intercepts request
    → Extract dioxus-data header (base64 JSON)
    → IpcMessage { method, params }
    → Handle: UserEvent, Query, BrowserOpen, Initialize
```

### Protocol Handler
- `dioxus://` custom protocol for asset serving
- Handles `__events` path for event processing
- `__file_dialog` for file selection
- Custom handler namespaces for user-provided handlers
- Checks `dioxus_asset_resolver` for bundled assets

### Native Features
- Menu integration via muda crate
- System tray via trayicon
- Global hotkeys via global_hotkey crate
- File dialogs and drag-drop support
- Headless mode for testing

### Configuration
```
Config
├── WindowBuilder customization
├── Custom event loop
├── Protocols and async protocols
├── Pre-rendered HTML template
├── Context menu disable flag
├── Background color (RGBA)
└── Devtools support toggle
```

## Native Renderer (dioxus-native)

### Blitz Integration
- Blitz layout engine for CSS layout
- Vello for GPU-accelerated vector rendering
- Winit for cross-platform window management
- Not a browser engine - custom native rendering pipeline

### DioxusNativeWindowRenderer
```
DioxusNativeWindowRenderer
├── anyrender-vello wrapper
├── WindowRenderer trait implementation
├── GPU features configurable
└── Custom paint support
```

### Rendering Pipeline
```
VirtualDOM components
    → DioxusNativeDOM
    → Blitz DOM tree with CSS
    → Blitz layout engine
    → Vello renderer
    → GPU rendering
```

### Layout and CSS
- CSS 2.1+ parser (not full CSS 3)
- Flexbox-based layout model
- Computed styles attached to element nodes
- Layout computed bottom-up during mutation

### Application Handler (Winit)
```
Event::NewEvents(StartCause::Init)
    → Create initial window
    → DioxusDocument with VirtualDOM

Resumed → Renderer.resume()

WindowEvent::RedrawRequested
    → VirtualDOM.render_immediate()
    → Mutations applied to Blitz DOM
    → Layout computed
    → Render frame

WindowEvent::Resized → Queue redraw
```

## LiveView Renderer (dioxus-liveview)

### Server-Rendered Architecture
```
Client (Browser with WebSocket)
    ↓ events
Server (VirtualDOM runs here)
    ↓ mutations (binary protocol)
Client receives mutations → Sledgehammer applies
```

### LiveViewPool
- Thread pool using `tokio_util::task::LocalPoolHandle`
- Each client spawns pinned task
- VirtualDOM runs on task's executor
- Pool handles multiple concurrent clients

### Per-Client Lifecycle
```
LiveViewPool::launch_virtualdom(ws, || VirtualDom::new())
    → Create MutationState, QueryEngine
    → vdom.rebuild() → Send initial HTML
    → Loop:
        tokio::select! {
            ws.next() → Handle event/query
            vdom.wait_for_work() → Has work
            query_rx.recv() → JS query
            hot_reload_rx.recv() → Hot reload
        }
        render_immediate() → Send mutations
```

### Binary Protocol
```
MutationState::write_memory_into(&mut bytes)
    → Sledgehammer binary encoding
    → WebSocket transmission
    → Client: window.interpreter.handleEdits(bytes)
```

### Message Types
1. **Binary Frames** (mutations)
2. **Text Frames** (queries/metadata)
3. **Incoming Events** (user interactions)

### Mounted Element Queries
```
LiveviewElement methods:
    scroll_offset() → JS: getScrollLeft/Top()
    scroll_size() → JS: getScrollWidth/Height()
    client_rect() → JS: getBoundingClientRect()
    scroll_to(options) → JS: element.scrollTo()
```

## Interpreter Package (dioxus-interpreter-js)

### Sledgehammer Framework
Ultra-compact binary protocol for DOM mutations shared across renderers.

### Core Components
1. **INTERPRETER_JS** - Base interpreter class
2. **NATIVE_JS** - Platform-specific extensions
3. **SLEDGEHAMMER_JS** - Generated bindings from Rust

### MutationState
- Implements WriteMutations for binary serialization
- Channel-based mutation recording
- Template caching and deduplication

### Stack Machine Operations
```
create_text_node(value, id) → Push text node
create_placeholder(id) → Push comment node
append_children(parent_id, count) → Pop and append
replace_with(id, count) → Replace element
set_attribute(id, name, value, ns) → Set DOM attribute
new_event_listener(name, id, bubbles) → Register listener
```

## Common Patterns

### 1. WriteMutations Implementation
Every renderer implements the trait differently:
- **Web**: Delegates to Sledgehammer JS
- **Desktop**: Accumulates in MutationState, sends via WebSocket
- **Native**: Applies directly to Blitz DOM
- **Liveview**: Accumulates and sends via WebSocket

### 2. Configuration Pattern
Renderers use `Box<dyn Any>` for extensible config:
```rust
launch(app, configs: Vec<Box<dyn Any>>)
    → For each config: try downcast to expected type
```

### 3. Lazy Initialization
Most renderers delay context setup until first window/request.

### 4. Event Loop Integration
- **Web**: WASM bindgen + browser event loop
- **Desktop**: Tao event loop with UserWindowEvent
- **Native**: Winit ApplicationHandler
- **Liveview**: Tokio select! macro

### 5. Mutation Batching
All non-web renderers batch mutations for efficiency:
- Accumulate in temporary structure
- Periodically flush to transport

## Adding a New Renderer

1. **Implement WriteMutations**
   - Define how platform applies DOM-like mutations
   - Handle template system
   - Manage stack machine state

2. **Implement HtmlEventConverter**
   - Map platform events to Dioxus types
   - Handle serialization if needed

3. **Create Launch Function**
   - Create VirtualDOM
   - Initialize renderer structures
   - Enter platform event loop
   - Call `vdom.render_immediate(&mut mutations)`

4. **Define Configuration**
   - Create Config struct
   - Support `Box<dyn Any>` downcasting

5. **Optional Enhancements**
   - Custom mount event data
   - Document integration
   - History support
   - Asset serving
