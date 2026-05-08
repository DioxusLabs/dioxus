# Dioxus Fullstack Architecture

The fullstack ecosystem enables seamless client-server communication through server functions, SSR, and hydration.

## Server Functions (#[server])

### Macro Generation

The `#[server]` macro generates dual code paths:

**On Client:**
```rust
// Creates request encoding/decoding
let client = ClientRequest::new(Method::POST, endpoint, &query);
let response = ServerFnEncoder::fetch_client(client, args, unpack).await;
let result = ServerFnDecoder::decode_client_response(response).await;
```

**On Server:**
```rust
// Creates Axum handler
fn __inner__function__(state, request) -> Response {
    // Extract arguments from request
    // Call actual function
    // Serialize response
}

// Register globally
inventory::submit! {
    ServerFunction::new(Method::POST, path, handler)
}
```

### Configuration Options
- `endpoint` - Custom URL path prefix (default: `/api`)
- `input` - Request encoding (Json, Cbor, MessagePack)
- `output` - Response encoding
- `middleware` - Tower middleware layers
- Server-only extractors: `headers: HeaderMap`, `cookies: Cookies`

### Wire Protocol

**Request:**
- Method: POST
- URL: `/api/{function_name}?{query_params}`
- Content-Type: application/json
- Body: JSON-serialized struct with arguments

**Response:**
```rust
enum RestEndpointPayload<T, E> {
    Success(T),
    Error(ErrorPayload<E>),
}

struct ErrorPayload<E> {
    message: String,
    code: u16,
    data: Option<E>,
}
```

### Handler Registration
1. `ServerFunction` struct holds metadata
2. `inventory::submit!` for compile-time registration
3. `ServerFunction::collect()` returns all handlers
4. `DioxusRouterExt::register_server_functions()` registers on Axum

## SSR (Server-Side Rendering)

### SsrRendererPool
- Thread-safe pool of `dioxus_ssr::Renderer` instances
- Manages concurrent rendering
- Optional incremental cache for static generation

### Rendering Pipeline
1. Create VirtualDom with component function
2. Render to HTML string
3. Inject hydration data and scripts
4. Stream chunks via futures channel

### Streaming Modes

**Disabled (default):**
- All server futures resolved before sending
- Simple, SEO-friendly

**OutOfOrder:**
- Initial chunk sent with placeholders
- Suspense boundaries stream independently
- Uses `StreamingRenderer`

**Streaming Chunk Structure:**
```html
<!-- Initial -->
<div>Header</div>
<div>Loading...</div>

<!-- Later -->
<script>window.dx_hydrate(mount_id, "data");</script>
<div hidden id="ds-1-r">Resolved content</div>
```

## Hydration

### Server-Side
1. `HydrationContext` created at render start
2. Hooks serialize data: `use_server_future`, `use_server_cached`, `use_loader`
3. Data stored with unique IDs
4. Serialized to base64-encoded CBOR
5. Injected as `window.__DIOXUS_HYDRATION_DATA__`

### Client-Side
1. JavaScript reads hydration data from HTML
2. Runtime provides `HydrationContext` to components
3. Components call same hooks, check for cached data
4. Data deserialized and populated
5. Event listeners attached without re-rendering

### Key Types
- `SerializeContextEntry<T>` - Entry in hydration context
- `SerializedHydrationData` - Base64-encoded CBOR
- `TakeDataError` - Why data couldn't be retrieved

## Server Architecture

### FullstackState
- Wraps `FullstackContext` and runtime handle
- Passed as State to Axum handlers
- Allows request context access

### DioxusRouterExt Trait
Extension methods on `axum::Router`:
- `serve_static_assets()` - Serves `/public`
- `serve_dioxus_application(cfg, component)` - Full SSR + server functions
- `register_server_functions()` - Registers collected handlers
- `serve_api_application(cfg, component)` - API-only

### Handler Wrapping
```
Handler wrapped in:
1. FullstackContext::scope() - provides context
2. LocalPool::spawn_pinned() - enables !Send futures
3. Middleware layers
4. Response header injection
```

### FullstackContext
Task-local context providing:
- Request headers via RwLock
- Streaming status (RenderingInitialChunk, Committed, Complete)
- Response headers
- HTTP status codes
- Reactive subscriptions

### ServeConfig
- `index` - Custom IndexHtml
- `incremental` - ISR cache
- `context_providers` - Injectable contexts
- `streaming_mode` - Disabled or OutOfOrder

## Client-Server Communication

### ClientRequest
```
ClientRequest
├── url: String
├── method: Method
├── headers: HeaderMap
└── extensions: Extensions
```

### Encoding Traits
- `EncodeRequest<In, Out, R>` - Serializes and makes request
- `IntoRequest<R>` - Converts to request
- `Encoding` trait - Defines format (Json, Cbor)

### Suspense Integration
- Server functions automatically suspend
- `use_server_future` - Waits on server, caches for client
- `use_loader` - Enhanced error handling
- `use_server_cached` - Caches computed values

### Error Handling
- Server errors serialized with message, code, details
- `ServerFnError` enum standardizes representation
- `AsStatusCode` trait maps to HTTP status
- Supports `anyhow::Error`, `StatusCode`, custom types

## Extension Points

### Adding Database Support
```rust
config.provide_context(|| {
    Box::new(Database::new()) as Box<dyn Any>
});

#[server]
async fn get_user(id: i32) -> Result<User> {
    let db = consume_context::<Database>();
    db.query(...).await
}
```

### Adding Authentication
```rust
#[server(auth: AuthExtractor)]
async fn protected(data: String, auth: AuthExtractor) -> Result<String> {
    // auth extracted from request
}
```

Or via middleware:
```rust
#[server]
#[middleware(AuthLayer::new())]
async fn protected_fn() -> Result<String> { }
```

### Custom Encoding
```rust
pub struct CustomEncoding;
impl Encoding for CustomEncoding {
    fn content_type() -> &'static str { "application/custom" }
    fn encode(data: impl Serialize, buf: &mut Vec<u8>) -> Option<usize> { }
    fn decode<O: DeserializeOwned>(bytes: Bytes) -> Option<O> { }
}

#[server(input = CustomEncoding, output = CustomEncoding)]
async fn my_fn(arg: String) -> Result<String> { }
```

### Response Customization
```rust
#[server]
async fn custom_response() -> Result<String> {
    let ctx = FullstackContext::current().unwrap();
    ctx.set_response_headers(headers);
    Ok("data".to_string())
}
```

## Data Flow

### Server Function Call
```
Client Component
  → Call #[server] fn
  → Create ClientRequest, serialize args
  → HTTP POST /api/function_name
  → Server Router
  → FullstackContext::scope()
  → Extract args, call function
  → Serialize Result
  → HTTP Response
  → Client deserialize
  → Component receives value
```

### SSR Hydration
```
Server:
  Page Request → VirtualDom::new()
  → HydrationContext created
  → Render components
  → use_server_future serializes results
  → Inject __DIOXUS_HYDRATION__ script
  → HTTP 200 + HTML

Client:
  Receive HTML
  → Parse hydration data
  → VirtualDom::new()
  → HydrationContext populated
  → Components render, hooks get cached data
  → Event listeners attached
  → Hydration complete
```

## Key Patterns

### Inventory System
Global registration via `inventory::submit!` at compile time. Enables dynamic discovery without explicit registration.

### Magic Trait Deref
Multiple `&` deref layers select correct trait impl:
- Prefers `FromRequest` over `DeserializeOwned`
- Prefers `FromResponse` over serialization

### Hash-based Routing
Routes hashed using xxhash64 of module path, preventing collisions with same-name functions in different modules.

### Error Type Tiering
Auto-deref tiers implementations:
1. Typed error struct
2. ServerFnError wrapper
3. anyhow::Error
